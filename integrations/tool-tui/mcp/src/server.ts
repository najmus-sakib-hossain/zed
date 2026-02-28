import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import open from 'open';
import { chromium } from 'playwright';
import screenshotDesktop from 'screenshot-desktop';
import Tesseract from 'tesseract.js';
import { spawn } from 'node:child_process';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import * as z from 'zod/v4';

function getWorkspaceRoot(): string {
    return process.env.DX_WORKSPACE?.trim() || process.cwd();
}

function getDxMcpDir(): string {
    const outDir = path.resolve(getWorkspaceRoot(), '.dx', 'mcp');
    if (!fs.existsSync(outDir)) {
        fs.mkdirSync(outDir, { recursive: true });
    }
    return outDir;
}

function resolvePathInsideDir(dir: string, relOrAbs: string): string {
    if (path.isAbsolute(relOrAbs)) {
        return relOrAbs;
    }
    const resolved = path.resolve(dir, relOrAbs);
    const normalizedDir = path.resolve(dir) + path.sep;
    if (!resolved.startsWith(normalizedDir)) {
        throw new Error(`Refusing to access path outside ${dir}: ${relOrAbs}`);
    }
    return resolved;
}

function updateLatestScreenshots(png: Buffer, kind: 'preview' | 'window'): {
    latestPath: string;
    latestKindPath: string;
} {
    const dir = getDxMcpDir();
    const latestPath = path.join(dir, 'latest.png');
    const latestKindPath = path.join(dir, `latest-${kind}.png`);

    fs.writeFileSync(latestPath, png);
    fs.writeFileSync(latestKindPath, png);

    return { latestPath, latestKindPath };
}

function resolveSavePath(savePath?: string): string | undefined {
    const trimmed = savePath?.trim();
    if (!trimmed) {
        return undefined;
    }

    if (path.isAbsolute(trimmed)) {
        return trimmed;
    }

    return path.resolve(getWorkspaceRoot(), trimmed);
}

function defaultScreenshotPath(kind: 'preview' | 'window'): string {
    const outDir = getDxMcpDir();

    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    const nonce = Math.random().toString(16).slice(2, 10);
    return path.join(outDir, `${kind}-${stamp}-${nonce}.png`);
}

async function ocrPng(png: Buffer, lang: string): Promise<string> {
    const normalizedLang = (lang || 'eng').trim() || 'eng';

    const result = await Tesseract.recognize(png, normalizedLang, {
        logger: () => {
            // Avoid stdout logging in stdio MCP servers
        }
    });

    const text = (result?.data?.text ?? '').trim();
    return text;
}

function resolvePreviewUrl(args: {
    url?: string;
    port?: number;
    path?: string;
}): string {
    const pathPart = args.path ?? '/';

    if (args.url && args.port) {
        throw new Error('Provide either url or port, not both.');
    }

    if (args.url) {
        const parsed = new URL(args.url);
        if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
            throw new Error(`Unsupported protocol: ${parsed.protocol}`);
        }
        if (pathPart && pathPart !== '/') {
            parsed.pathname = pathPart.startsWith('/') ? pathPart : `/${pathPart}`;
        }
        return parsed.toString();
    }

    const port = args.port ?? 3000;
    if (!Number.isInteger(port) || port <= 0 || port > 65535) {
        throw new Error(`Invalid port: ${port}`);
    }

    const pathname = pathPart.startsWith('/') ? pathPart : `/${pathPart}`;
    return `http://127.0.0.1:${port}${pathname}`;
}

function findPlaywrightChromiumExecutablePath(): string | undefined {
    const localAppData = process.env.LOCALAPPDATA;
    if (!localAppData) {
        return undefined;
    }

    const baseDir = path.join(localAppData, 'ms-playwright');
    if (!fs.existsSync(baseDir)) {
        return undefined;
    }

    const chromiumDirs = fs
        .readdirSync(baseDir, { withFileTypes: true })
        .filter(entry => entry.isDirectory() && entry.name.startsWith('chromium-'))
        .map(entry => entry.name)
        .sort((a, b) => {
            const aNum = Number.parseInt(a.replace('chromium-', ''), 10);
            const bNum = Number.parseInt(b.replace('chromium-', ''), 10);
            return (Number.isNaN(bNum) ? 0 : bNum) - (Number.isNaN(aNum) ? 0 : aNum);
        });

    for (const dirName of chromiumDirs) {
        const exe = path.join(baseDir, dirName, 'chrome-win64', 'chrome.exe');
        if (fs.existsSync(exe)) {
            return exe;
        }
    }

    return undefined;
}

async function launchChromiumForScreenshot() {
    try {
        return await chromium.launch({ headless: true, timeout: 30_000 });
    } catch (error) {
        const exePath = findPlaywrightChromiumExecutablePath();
        if (!exePath) {
            throw error;
        }

        // Fallback for environments where the default headless shell launch can hang/time out.
        return await chromium.launch({
            headless: true,
            timeout: 30_000,
            executablePath: exePath,
            args: ['--disable-gpu']
        });
    }
}

function firstExistingFile(pathsToTry: string[]): string | undefined {
    for (const p of pathsToTry) {
        if (p && fs.existsSync(p)) {
            return p;
        }
    }
    return undefined;
}

function findWindowsEdgeExecutablePath(): string | undefined {
    const pf = process.env['ProgramFiles'] ?? 'C:\\Program Files';
    const pfx86 = process.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)';

    return firstExistingFile([
        path.join(pf, 'Microsoft', 'Edge', 'Application', 'msedge.exe'),
        path.join(pfx86, 'Microsoft', 'Edge', 'Application', 'msedge.exe')
    ]);
}

function findWindowsChromeExecutablePath(): string | undefined {
    const pf = process.env['ProgramFiles'] ?? 'C:\\Program Files';
    const pfx86 = process.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)';

    return firstExistingFile([
        path.join(pf, 'Google', 'Chrome', 'Application', 'chrome.exe'),
        path.join(pfx86, 'Google', 'Chrome', 'Application', 'chrome.exe')
    ]);
}

function findWindowsHeadlessBrowserExecutablePath(): { exePath: string; name: string } | undefined {
    const edge = findWindowsEdgeExecutablePath();
    if (edge) {
        return { exePath: edge, name: 'Edge' };
    }

    const chrome = findWindowsChromeExecutablePath();
    if (chrome) {
        return { exePath: chrome, name: 'Chrome' };
    }

    return undefined;
}

async function screenshotUrlWithWindowsHeadlessBrowser(args: {
    targetUrl: string;
    viewportWidth: number;
    viewportHeight: number;
    waitMs: number;
    outputPath: string;
}): Promise<{ png: Buffer; engine: string }> {
    if (!isWindows()) {
        throw new Error('Headless browser CLI screenshot fallback is only implemented on Windows.');
    }

    const found = findWindowsHeadlessBrowserExecutablePath();
    if (!found) {
        throw new Error('Could not find Microsoft Edge or Google Chrome executable to run headless screenshot.');
    }

    const outputPath = args.outputPath;
    const dir = path.dirname(outputPath);
    if (dir && dir !== '.' && !fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
    }

    try {
        if (fs.existsSync(outputPath)) {
            fs.unlinkSync(outputPath);
        }
    } catch {
        // ignore
    }

    const virtualTimeBudget = Math.min(Math.max(args.waitMs, 0) + 2_000, 25_000);
    const windowSize = `${args.viewportWidth},${args.viewportHeight}`;

    const cliArgs: string[] = [
        '--headless=new',
        '--disable-gpu',
        '--hide-scrollbars',
        `--window-size=${windowSize}`,
        `--virtual-time-budget=${virtualTimeBudget}`,
        `--screenshot=${outputPath}`,
        '--no-first-run',
        '--no-default-browser-check',
        args.targetUrl
    ];

    const child = spawn(found.exePath, cliArgs, {
        windowsHide: true,
        stdio: ['ignore', 'ignore', 'pipe']
    });

    let stderr = '';
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', chunk => {
        stderr += chunk;
    });

    const exitCode: number = await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => {
            try {
                child.kill();
            } catch {
                // ignore
            }
            reject(new Error(`Headless ${found.name} screenshot timed out.`));
        }, 25_000);

        child.on('error', err => {
            clearTimeout(timeout);
            reject(err);
        });
        child.on('close', code => {
            clearTimeout(timeout);
            resolve(code ?? 0);
        });
    });

    if (exitCode !== 0) {
        throw new Error(`Headless ${found.name} failed (exit ${exitCode}). ${stderr.trim()}`);
    }

    if (!fs.existsSync(outputPath)) {
        throw new Error(`Headless ${found.name} did not produce screenshot at ${outputPath}. ${stderr.trim()}`);
    }

    const png = fs.readFileSync(outputPath);
    if (!png || png.length === 0) {
        throw new Error(`Headless ${found.name} produced an empty screenshot at ${outputPath}. ${stderr.trim()}`);
    }

    return { png, engine: found.name };
}

function isWindows(): boolean {
    return process.platform === 'win32';
}

async function runPowerShellEncoded(script: string): Promise<{ stdout: string; stderr: string }> {
    if (!isWindows()) {
        throw new Error('PowerShell-based window capture is only supported on Windows.');
    }

    // PowerShell -EncodedCommand uses UTF-16LE base64.
    const encoded = Buffer.from(script, 'utf16le').toString('base64');

    const child = spawn(
        'powershell',
        ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', encoded],
        { windowsHide: true }
    );

    let stdout = '';
    let stderr = '';

    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');

    child.stdout.on('data', chunk => {
        stdout += chunk;
    });
    child.stderr.on('data', chunk => {
        stderr += chunk;
    });

    const exitCode: number = await new Promise((resolve, reject) => {
        child.on('error', reject);
        child.on('close', resolve);
    });

    if (exitCode !== 0) {
        throw new Error(`PowerShell failed (exit ${exitCode}). ${stderr || stdout}`);
    }

    return { stdout, stderr };
}

type WindowInfo = {
    hwnd: string;
    pid: number;
    processName: string;
    title: string;
    x: number;
    y: number;
    width: number;
    height: number;
};

async function listWindowsWin32(): Promise<WindowInfo[]> {
    const script = `
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class Win32 {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);

  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
}
"@

$windows = New-Object System.Collections.Generic.List[Object]

[Win32]::EnumWindows({
    param([IntPtr]$hWnd, [IntPtr]$lParam)

    if (-not [Win32]::IsWindowVisible($hWnd)) { return $true }

    $processId = 0
    [void][Win32]::GetWindowThreadProcessId($hWnd, [ref]$processId)

    $len = [Win32]::GetWindowTextLength($hWnd)
    $title = ""
    if ($len -gt 0) {
        $sb = New-Object System.Text.StringBuilder ($len + 1)
        [void][Win32]::GetWindowText($hWnd, $sb, $sb.Capacity)
        $title = $sb.ToString()
    }

    $rect = New-Object Win32+RECT
    if (-not [Win32]::GetWindowRect($hWnd, [ref]$rect)) { return $true }

    $w = $rect.Right - $rect.Left
    $h = $rect.Bottom - $rect.Top
    if ($w -lt 50 -or $h -lt 50) { return $true }

    # Get process name if available
    $processName = ""
    try {
        $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if ($proc) { $processName = $proc.ProcessName }
    } catch {}

    $windows.Add([pscustomobject]@{
        hwnd = ('0x{0:X}' -f $hWnd.ToInt64())
        pid = $processId
        processName = $processName
        title = $title
        x = $rect.Left
        y = $rect.Top
        width = $w
        height = $h
    })

    return $true
}, [IntPtr]::Zero) | Out-Null

$windows | ConvertTo-Json -Depth 3
`;

    let stdout: string;
    try {
        ({ stdout } = await runPowerShellEncoded(script));
    } catch (e) {
        const preview = script.split(/\r?\n/).slice(0, 90).join('\n');
        throw new Error(
            `PowerShell list windows failed: ${String(e)}\n\n--- script preview (first 90 lines) ---\n${preview}`
        );
    }
    const text = stdout.trim();
    if (!text) {
        return [];
    }
    const parsed = JSON.parse(text) as WindowInfo[] | WindowInfo;
    return Array.isArray(parsed) ? parsed : [parsed];
}

async function screenshotWindowWin32(opts: {
    titleQuery: string;
    savePath?: string;
    allowRaise?: boolean;
}): Promise<{ pngBase64: string; matchedTitle: string; savedTo?: string; captureMethod?: string }> {
    const titleQuery = opts.titleQuery.trim();
    if (!titleQuery) {
        throw new Error('titleQuery is required');
    }

    const windows = await listWindowsWin32();
    const query = titleQuery.toLowerCase();
    
    // Try exact title match first (case-insensitive)
    let match = windows.find(w => w.title.toLowerCase() === query);
    
    // Then try exact process name match
    if (!match) {
        match = windows.find(w => w.processName.toLowerCase() === query);
    }
    
    // Then try process name substring match
    if (!match) {
        match = windows.find(w => w.processName.toLowerCase().includes(query));
    }
    
    // Finally fall back to title substring match
    if (!match) {
        match = windows.find(w => w.title.toLowerCase().includes(query));
    }
    
    if (!match) {
        throw new Error(`No visible window title or process matched: ${titleQuery}`);
    }

    const hwndDecimal = Number.parseInt(match.hwnd, 16);
    if (!Number.isFinite(hwndDecimal)) {
        throw new Error(`Invalid hwnd for matched window: ${match.hwnd}`);
    }

    const outPath = opts.savePath?.trim() || '';
    const allowRaise = Boolean(opts.allowRaise);

    const script = `
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class Win32 {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd, IntPtr hDC, uint nFlags);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
}
"@


$hWnd = [IntPtr]::new(${hwndDecimal})

# Avoid stealing focus: if minimized, show without activating.
$SW_SHOWNOACTIVATE = 4
if ([Win32]::IsIconic($hWnd)) {
    [void][Win32]::ShowWindow($hWnd, $SW_SHOWNOACTIVATE)
}

$rect = New-Object Win32+RECT
if (-not [Win32]::GetWindowRect($hWnd, [ref]$rect)) { throw 'GetWindowRect failed' }

$x = $rect.Left
$y = $rect.Top
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top

if ($w -le 0 -or $h -le 0) { throw 'Invalid window dimensions' }

$bmp = New-Object System.Drawing.Bitmap $w, $h
$gfx = [System.Drawing.Graphics]::FromImage($bmp)

# Prefer PrintWindow so we capture the target window even if it's behind other windows.
$hdc = $gfx.GetHdc()
$ok = $false
try {
    $ok = [Win32]::PrintWindow($hWnd, $hdc, 0)
} finally {
    $gfx.ReleaseHdc($hdc)
}

function Test-ProbablyBlank([System.Drawing.Bitmap]$b) {
    # Heuristic: sample a grid of pixels; if there are very few distinct colors, treat as blank.
    $cols = New-Object 'System.Collections.Generic.HashSet[int]'
    $xs = 12
    $ys = 12
    for ($iy = 0; $iy -lt $ys; $iy++) {
        $py = [int](($b.Height - 1) * $iy / [Math]::Max(($ys - 1), 1))
        for ($ix = 0; $ix -lt $xs; $ix++) {
            $px = [int](($b.Width - 1) * $ix / [Math]::Max(($xs - 1), 1))
            $c = $b.GetPixel($px, $py)
            [void]$cols.Add($c.ToArgb())
            if ($cols.Count -gt 12) { return $false }
        }
    }
    return $true
}

$needsFallback = (-not $ok) -or (Test-ProbablyBlank $bmp)

$captureMethod = 'printwindow'

if ($needsFallback) {
    if (-not ${allowRaise ? '$true' : '$false'}) {
        [pscustomobject]@{ error = 'PrintWindow returned blank content. For GPU/WebView2 apps this is common. Re-run with allowRaise=true to temporarily raise the window (no-activate) for a pixel capture.' } | ConvertTo-Json -Depth 3
        exit 0
    }

    # PrintWindow often returns blank for GPU/WebView2 surfaces. As a fallback, temporarily raise
    # the target window without activating it, then copy pixels from the screen.
    $captureMethod = 'screen-copy-topmost'
    $HWND_TOPMOST = [IntPtr]::new(-1)
    $HWND_TOP = [IntPtr]::new(0)
    $HWND_NOTOPMOST = [IntPtr]::new(-2)
    $SWP_NOMOVE = 0x0002
    $SWP_NOSIZE = 0x0001
    $SWP_NOACTIVATE = 0x0010
    $SWP_SHOWWINDOW = 0x0040
    $flags = $SWP_NOMOVE -bor $SWP_NOSIZE -bor $SWP_NOACTIVATE -bor $SWP_SHOWWINDOW

    [void][Win32]::SetWindowPos($hWnd, $HWND_TOPMOST, 0, 0, 0, 0, $flags)
    [void][Win32]::SetWindowPos($hWnd, $HWND_TOP, 0, 0, 0, 0, $flags)
    Start-Sleep -Milliseconds 120

    # Re-read rect in case DPI/bounds changed.
    $rect2 = New-Object Win32+RECT
    if ([Win32]::GetWindowRect($hWnd, [ref]$rect2)) {
        $x = $rect2.Left
        $y = $rect2.Top
        $w = $rect2.Right - $rect2.Left
        $h = $rect2.Bottom - $rect2.Top
    }

    if ($w -gt 0 -and $h -gt 0) {
        if ($bmp.Width -ne $w -or $bmp.Height -ne $h) {
            $gfx.Dispose()
            $bmp.Dispose()
            $bmp = New-Object System.Drawing.Bitmap $w, $h
            $gfx = [System.Drawing.Graphics]::FromImage($bmp)
        }
        $gfx.CopyFromScreen($x, $y, 0, 0, $bmp.Size)
    }

    [void][Win32]::SetWindowPos($hWnd, $HWND_NOTOPMOST, 0, 0, 0, 0, $flags)
}

$ms = New-Object System.IO.MemoryStream
$bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
$bytes = $ms.ToArray()

$gfx.Dispose()
$bmp.Dispose()
$ms.Dispose()

$b64 = [Convert]::ToBase64String($bytes)

$savePath = ${outPath ? `'${outPath.replace(/'/g, "''")}'` : '$null'}
if ($savePath -and $savePath.Length -gt 0) {
  $dir = [System.IO.Path]::GetDirectoryName($savePath)
  if ($dir -and -not (Test-Path -LiteralPath $dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  [System.IO.File]::WriteAllBytes($savePath, $bytes)
}

    [pscustomobject]@{ pngBase64 = $b64; savedTo = $savePath; captureMethod = $captureMethod } | ConvertTo-Json -Depth 3
`;

    try {
        const { stdout } = await runPowerShellEncoded(script);
        const result = JSON.parse(stdout.trim()) as {
            pngBase64?: string;
            savedTo?: string | null;
            captureMethod?: string | null;
            error?: string | null;
        };

        if (result.error) {
            throw new Error(String(result.error));
        }

        if (!result.pngBase64) {
            throw new Error('PowerShell screenshot did not return pngBase64.');
        }

        return {
            pngBase64: result.pngBase64,
            matchedTitle: match.title,
            savedTo: result.savedTo || undefined,
            captureMethod: (result.captureMethod || undefined) ?? undefined
        };
    } catch (error) {
        throw new Error(`Window screenshot failed: ${String(error)}`);
    }

const server = new McpServer({
    name: 'DX',
    version: '1.0.0',
    description: 'Enhanced Development Experience',
    icons: (() => {
        try {
            const here = path.dirname(fileURLToPath(import.meta.url));
            const mcpRoot = path.resolve(here, '..');
            const logoPath = path.join(mcpRoot, 'media', 'logo.png');
            if (!fs.existsSync(logoPath)) {
                return undefined;
            }
            return [
                {
                    src: pathToFileURL(logoPath).toString(),
                    mimeType: 'image/png',
                    sizes: ['256x256']
                }
            ];
        } catch {
            return undefined;
        }
    })()
});

server.registerTool(
    'open_preview',
    {
        title: 'Open Preview URL',
        description:
            'Opens a localhost preview (e.g. a VS Code forwarded port) in your default browser.',
        inputSchema: {
            url: z.string().url().optional().describe('Full URL to open (http/https).'),
            port: z.number().int().positive().optional().describe('Localhost port to open.'),
            path: z
                .string()
                .optional()
                .describe('Optional path (default "/"). Used with url or port.')
        }
    },
    async args => {
        try {
            const targetUrl = resolvePreviewUrl(args);
            await open(targetUrl);
            return {
                content: [{ type: 'text', text: `Opened: ${targetUrl}` }]
            };
        } catch (error) {
            return {
                isError: true,
                content: [{ type: 'text', text: `open_preview failed: ${String(error)}` }]
            };
        }
    }
);

server.registerTool(
    'screenshot_preview',
    {
        title: 'Screenshot Preview URL',
        description:
            'Captures a screenshot for a URL/port preview. Default behavior is to open the URL and take a desktop screenshot (best for VS Code internal preview + native apps). Optional fallback: Playwright page screenshot. Always returns image/png and saves to `.dx/mcp/` by default.',
        inputSchema: {
            mode: z
                .enum(['vscode', 'screen', 'page'])
                .optional()
                .describe(
                    '"vscode" and "screen" capture the desktop (best for VS Code internal preview and native apps). "page" uses Playwright to screenshot the URL.'
                ),
            url: z.string().url().optional().describe('Full URL to screenshot (http/https).'),
            port: z.number().int().positive().optional().describe('Localhost port to screenshot.'),
            path: z
                .string()
                .optional()
                .describe('Optional path (default "/"). Used with url or port.'),
            open: z
                .boolean()
                .optional()
                .describe(
                    'If true, opens the URL in your default browser before screenshot. Defaults to true for mode="vscode"/"screen", and false for mode="page".'
                ),
            fullPage: z
                .boolean()
                .optional()
                .describe('If true, captures full page (default true).'),
            viewportWidth: z
                .number()
                .int()
                .positive()
                .optional()
                .describe('Viewport width (default 1440).'),
            viewportHeight: z
                .number()
                .int()
                .positive()
                .optional()
                .describe('Viewport height (default 900).'),
            waitMs: z
                .number()
                .int()
                .nonnegative()
                .optional()
                .describe('Extra milliseconds to wait after navigation (default 0).'),
            savePath: z
                .string()
                .optional()
                .describe(
                    'Optional file path to save the PNG (directories will be created). If omitted, saves under `.dx/mcp/`.'
                ),
            ocr: z
                .boolean()
                .optional()
                .describe('If true, runs OCR on the captured screenshot and returns extracted text.'),
            ocrLang: z
                .string()
                .optional()
                .describe('OCR language code (default: "eng").')
        }
    },
    async args => {
        let browser: Awaited<ReturnType<typeof chromium.launch>> | undefined;

        try {
            const targetUrl = resolvePreviewUrl(args);

            const mode = args.mode ?? 'vscode';
            const shouldOpen = args.open ?? (mode !== 'page');

            if (shouldOpen) {
                await open(targetUrl);
            }
            if (mode === 'vscode' || mode === 'screen') {
                const waitMs = args.waitMs ?? 750;
                if (waitMs > 0) {
                    await new Promise(resolve => setTimeout(resolve, waitMs));
                }

                const png = (await screenshotDesktop({ format: 'png' })) as Buffer;
                const base64 = Buffer.from(png).toString('base64');

                const savePath =
                    resolveSavePath(args.savePath) ?? defaultScreenshotPath('preview');
                if (savePath) {
                    const dir = path.dirname(savePath);
                    if (dir && dir !== '.' && !fs.existsSync(dir)) {
                        fs.mkdirSync(dir, { recursive: true });
                    }
                    fs.writeFileSync(savePath, png);
                }

                const latest = updateLatestScreenshots(png, 'preview');

                let extractedText: string | undefined;
                if (args.ocr) {
                    extractedText = await ocrPng(png, args.ocrLang ?? 'eng');
                }

                return {
                    content: [
                        { type: 'image', data: base64, mimeType: 'image/png' },
                        {
                            type: 'text',
                            text:
                                `Desktop screenshot captured (intended to include the VS Code preview): ${targetUrl}` +
                                `\nSaved to: ${savePath}` +
                                `\nLatest: ${latest.latestPath}` +
                                (args.ocr
                                    ? `\n\nOCR (${args.ocrLang ?? 'eng'}):\n${extractedText || '(no text detected)'}`
                                    : '')
                        }
                    ]
                };
            }

            const savePath = resolveSavePath(args.savePath) ?? defaultScreenshotPath('preview');
            const viewportWidth = args.viewportWidth ?? 1440;
            const viewportHeight = args.viewportHeight ?? 810;
            const waitMs = args.waitMs ?? 0;

            let png: Buffer;
            let engine = 'Playwright';

            // On Windows, prefer the Chrome/Edge headless CLI path first. It's non-disruptive (no visible window)
            // and has proven more reliable than Playwright when running under Bun.
            if (isWindows()) {
                try {
                    const cli = await screenshotUrlWithWindowsHeadlessBrowser({
                        targetUrl,
                        viewportWidth,
                        viewportHeight,
                        waitMs,
                        outputPath: savePath
                    });
                    png = cli.png;
                    engine = `Headless ${cli.engine}`;
                } catch {
                    // fall through to Playwright
                    png = Buffer.alloc(0);
                }
            } else {
                png = Buffer.alloc(0);
            }

            if (!png || png.length === 0) {
                browser = await launchChromiumForScreenshot();
                const page = await browser.newPage({
                    viewport: {
                        width: viewportWidth,
                        height: viewportHeight
                    }
                });

                await page.goto(targetUrl, { waitUntil: 'domcontentloaded', timeout: 25_000 });
                if (waitMs > 0) {
                    await page.waitForTimeout(waitMs);
                }

                // Scroll to top to ensure consistent screenshots
                await page.evaluate(() => window.scrollTo(0, 0));
                
                // Wait a bit after scroll to ensure rendering is complete
                await page.waitForTimeout(100);

                const buf = await page.screenshot({ fullPage: args.fullPage ?? false, type: 'png' });
                png = Buffer.from(buf);
                engine = 'Playwright';
            }

            const base64 = Buffer.from(png).toString('base64');

            if (savePath) {
                const dir = path.dirname(savePath);
                if (dir && dir !== '.' && !fs.existsSync(dir)) {
                    fs.mkdirSync(dir, { recursive: true });
                }
                fs.writeFileSync(savePath, png);
            }

            const latest = updateLatestScreenshots(Buffer.from(png), 'preview');

            let extractedText: string | undefined;
            if (args.ocr) {
                extractedText = await ocrPng(Buffer.from(png), args.ocrLang ?? 'eng');
            }

            return {
                content: [
                    { type: 'image', data: base64, mimeType: 'image/png' },
                    {
                        type: 'text',
                        text:
                            `${engine} screenshot captured: ${targetUrl}\nSaved to: ${savePath}\nLatest: ${latest.latestPath}` +
                            (args.ocr
                                ? `\n\nOCR (${args.ocrLang ?? 'eng'}):\n${extractedText || '(no text detected)'}`
                                : '')
                    }
                ]
            };
        } catch (error) {
            const hint =
                'If this is the first run, install Playwright browsers: `cd mcp && bunx playwright install chromium`.';

            return {
                isError: true,
                content: [
                    {
                        type: 'text',
                        text:
                            `screenshot_preview failed: ${String(error)}\n${hint}\n` +
                            'Tip: use mode="page" (headless) to avoid opening/redirecting any visible browser windows, or mode="vscode" to capture the currently visible VS Code preview via desktop screenshot.'
                    }
                ]
            };
        } finally {
            try {
                await browser?.close();
            } catch {
                // ignore
            }
        }
    }
);

server.registerTool(
    'list_windows',
    {
        title: 'List Native App Windows',
        description:
            'Lists visible native app windows (Windows-only). Use this to find a title to pass to screenshot_window.',
        inputSchema: {
            query: z
                .string()
                .optional()
                .describe('Optional case-insensitive substring filter for window titles.')
        }
    },
    async ({ query }) => {
        try {
            const windows = isWindows() ? await listWindowsWin32() : [];
            const filtered = query
                ? windows.filter(w => w.title.toLowerCase().includes(query.toLowerCase()))
                : windows;

            return {
                content: [
                    {
                        type: 'text',
                        text: JSON.stringify(filtered, null, 2)
                    }
                ],
                structuredContent: { windows: filtered }
            };
        } catch (error) {
            return {
                isError: true,
                content: [{ type: 'text', text: `list_windows failed: ${String(error)}` }]
            };
        }
    }
);

server.registerTool(
    'screenshot_window',
    {
        title: 'Screenshot Native App Window',
        description:
            'Captures a screenshot of a native app window by title (Windows-only). Always returns image/png and saves to `.dx/mcp/` by default (or to savePath if provided).',
        inputSchema: {
            title: z
                .string()
                .min(1)
                .describe('Substring to match against visible window titles (case-insensitive).'),
            savePath: z
                .string()
                .optional()
                .describe(
                    'Optional file path to save the PNG (directories will be created). If omitted, saves under `.dx/mcp/`.'
                ),
            ocr: z
                .boolean()
                .optional()
                .describe('If true, runs OCR on the captured screenshot and returns extracted text.'),
            ocrLang: z
                .string()
                .optional()
                .describe('OCR language code (default: "eng").'),
            allowRaise: z
                .boolean()
                .optional()
                .describe(
                    'If true, allows a fallback for GPU/WebView2 windows: temporarily raise the target window (without activation) to capture real pixels. If false/omitted, DX uses PrintWindow only and will error if the window cannot be captured that way.'
                )
        }
    },
    async ({ title, savePath, ocr, ocrLang, allowRaise }) => {
        try {
            if (!isWindows()) {
                throw new Error('screenshot_window is currently only implemented on Windows.');
            }

            const resolvedSavePath =
                resolveSavePath(savePath) ?? defaultScreenshotPath('window');
            const result = await screenshotWindowWin32({
                titleQuery: title,
                savePath: resolvedSavePath,
                allowRaise
            });

            const pngBuf = Buffer.from(result.pngBase64, 'base64');
            const latest = updateLatestScreenshots(pngBuf, 'window');

            let extractedText: string | undefined;
            if (ocr) {
                extractedText = await ocrPng(Buffer.from(result.pngBase64, 'base64'), ocrLang ?? 'eng');
            }

            return {
                content: [
                    { type: 'image', data: result.pngBase64, mimeType: 'image/png' },
                    {
                        type: 'text',
                        text:
                            `Window screenshot captured: ${result.matchedTitle}` +
                            (result.captureMethod ? `\nCapture method: ${result.captureMethod}` : '') +
                            (result.savedTo ? `\nSaved to: ${result.savedTo}` : '') +
                            `\nLatest: ${latest.latestPath}` +
                            (ocr
                                ? `\n\nOCR (${ocrLang ?? 'eng'}):\n${extractedText || '(no text detected)'}`
                                : '')
                    }
                ]
            };
        } catch (error) {
            return {
                isError: true,
                content: [{ type: 'text', text: `screenshot_window failed: ${String(error)}` }]
            };
        }
    }
);

async function attachLatestScreenshotImpl(args: {
    filename?: string;
    ocr?: boolean;
    ocrLang?: string;
}): Promise<{
    content: Array<{ type: 'text'; text: string } | { type: 'image'; mimeType: 'image/png'; data: string }>;
    isError?: boolean;
}> {
    try {
        const dir = getDxMcpDir();
        const filename = (args.filename?.trim() || 'latest.png').trim();
        const candidate = resolvePathInsideDir(dir, filename);

        let filePath = candidate;
        if (!fs.existsSync(filePath)) {
            // Fallback: pick the newest png in .dx/mcp
            const newest = fs
                .readdirSync(dir)
                .filter(name => name.toLowerCase().endsWith('.png'))
                .map(name => ({
                    name,
                    full: path.join(dir, name),
                    mtime: fs.statSync(path.join(dir, name)).mtimeMs
                }))
                .sort((a, b) => b.mtime - a.mtime)[0];

            if (!newest) {
                return {
                    isError: true,
                    content: [
                        {
                            type: 'text',
                            text: `No screenshots found in: ${dir}`
                        }
                    ]
                };
            }

            filePath = newest.full;
        }

        const png = fs.readFileSync(filePath);
        const base64 = png.toString('base64');

        let extractedText: string | undefined;
        if (args.ocr) {
            extractedText = await ocrPng(png, args.ocrLang ?? 'eng');
        }

        return {
            content: [
                {
                    type: 'text',
                    text:
                        `Attached screenshot from: ${filePath} (${png.length} bytes)` +
                        (args.ocr
                            ? `\n\nOCR (${args.ocrLang ?? 'eng'}):\n${extractedText || '(no text detected)'}`
                            : '')
                },
                {
                    type: 'image',
                    mimeType: 'image/png',
                    data: base64
                }
            ]
        };
    } catch (error) {
        return {
            isError: true,
            content: [{ type: 'text', text: `attach_latest_screenshot failed: ${String(error)}` }]
        };
    }
}

for (const toolName of ['dx_attach_latest_screenshot', 'attach_latest_screenshot'] as const) {
    server.registerTool(
        toolName,
        {
            title: 'Attach Latest Screenshot',
            description:
                'Loads a saved screenshot (default: .dx/mcp/latest.png) and returns it as a proper image/png attachment. Use this when a chat client shows the image to the user but the model did not receive image pixels from prior steps.',
            inputSchema: {
                filename: z
                    .string()
                    .optional()
                    .describe('PNG filename within .dx/mcp (default: latest.png).'),
                ocr: z
                    .boolean()
                    .optional()
                    .describe('If true, runs OCR on the attached screenshot and returns extracted text.'),
                ocrLang: z.string().optional().describe('OCR language code (default: "eng").')
            }
        },
        attachLatestScreenshotImpl
    );
}

async function main() {
    const transport = new StdioServerTransport();
    await server.connect(transport);
}

main().catch(error => {
    // IMPORTANT: never log to stdout in stdio MCP servers
    console.error(error);
    process.exitCode = 1;
});
