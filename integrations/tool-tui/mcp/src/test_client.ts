import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { spawn } from 'node:child_process';

async function getForegroundWindowTitleWin32(): Promise<string | undefined> {
    if (process.platform !== 'win32') {
        return undefined;
    }

    const script = `
$ErrorActionPreference='Stop'

Add-Type -TypeDefinition @"
using System;
using System.Text;
using System.Runtime.InteropServices;

public static class Win32 {
  [DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();
  [DllImport(\"user32.dll\")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
  [DllImport(\"user32.dll\")] public static extern int GetWindowTextLength(IntPtr hWnd);
}
"@

$hwnd = [Win32]::GetForegroundWindow()
if ($hwnd -eq [IntPtr]::Zero) { '' ; exit 0 }
$len = [Win32]::GetWindowTextLength($hwnd)
if ($len -le 0) { '' ; exit 0 }
$sb = New-Object System.Text.StringBuilder ($len + 1)
[void][Win32]::GetWindowText($hwnd, $sb, $sb.Capacity)
$sb.ToString()
`;

    const child = spawn('powershell', ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-Command', script], {
        windowsHide: true
    });

    let stdout = '';
    let stderr = '';

    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', chunk => (stdout += chunk));
    child.stderr.on('data', chunk => (stderr += chunk));

    const exitCode: number = await new Promise((resolve, reject) => {
        child.on('error', reject);
        child.on('close', resolve);
    });

    if (exitCode !== 0) {
        throw new Error(`Failed to get foreground window title (exit ${exitCode}): ${stderr || stdout}`);
    }

    const title = stdout.trim();
    return title || undefined;
}

async function main() {
    const workspaceRoot = path.resolve(process.cwd(), '..');
    const transport = new StdioClientTransport({
        command: 'bun',
        args: ['src/server.ts'],
        cwd: process.cwd(),
        env: {
            ...process.env,
            DX_WORKSPACE: workspaceRoot
        },
        // Keep server stderr visible for debugging.
        stderr: 'inherit'
    });

    const client = new Client({ name: 'dx-mcp-test-client', version: '1.0.0' });
    await client.connect(transport);

    const tools = await client.listTools();
    console.log('tools:', tools.tools.map(t => t.name));

    const testUrls = [
        'https://www.google.com/'
    ];

    for (let i = 0; i < testUrls.length; i++) {
        const url = testUrls[i]!;
        console.log(`\n--- url test ${i + 1}/${testUrls.length}: ${url} ---`);

        const screenshotResult = await client.callTool({
            name: 'screenshot_preview',
            arguments: {
                url,
                mode: 'page',
                open: false,
                waitMs: 1000,
                ocr: i === 0,
                ocrLang: 'eng',
                // prove workspace-root save works
                savePath: `.dx/mcp/url-${i + 1}.png`
            }
        });

        const contentTypes = screenshotResult.content.map(c => c.type);
        const image = screenshotResult.content.find(
            c => c.type === 'image' && (c as any).mimeType === 'image/png'
        ) as any;
        const textBlock = screenshotResult.content.find(c => c.type === 'text') as any;

        console.log('screenshot_preview isError:', screenshotResult.isError ?? false);
        console.log('screenshot_preview result content types:', contentTypes);
        console.log(
            'screenshot_preview image/png present:',
            Boolean(image && typeof image.data === 'string' && image.data.length > 0),
            image ? `(base64Len=${image.data.length})` : ''
        );
        if (i === 0) {
            const expectedSavedPath = path.join(workspaceRoot, '.dx', 'mcp', `url-${i + 1}.png`);
            const savedOk = fs.existsSync(expectedSavedPath) && fs.statSync(expectedSavedPath).size > 0;
            console.log('saved file present:', savedOk, `(${expectedSavedPath})`);

            const txt = (textBlock?.text ?? '').toString();
            const ocrIdx = txt.indexOf('OCR');
            const snippet = (ocrIdx >= 0 ? txt.slice(ocrIdx) : txt).slice(0, 300);
            console.log('screenshot_preview OCR snippet:', JSON.stringify(snippet));

            const attached = await client.callTool({
                name: 'dx_attach_latest_screenshot',
                arguments: { filename: 'latest.png', ocr: true, ocrLang: 'eng' }
            });
            const attachedImage = attached.content.find(
                c => c.type === 'image' && (c as any).mimeType === 'image/png'
            ) as any;
            console.log(
                'dx_attach_latest_screenshot image/png present:',
                Boolean(attachedImage && typeof attachedImage.data === 'string' && attachedImage.data.length > 0),
                attachedImage ? `(base64Len=${attachedImage.data.length})` : ''
            );

            const latestPath = path.join(workspaceRoot, '.dx', 'mcp', 'latest.png');
            const latestOk = fs.existsSync(latestPath) && fs.statSync(latestPath).size > 0;
            console.log('latest.png present:', latestOk, `(${latestPath})`);

            // Verify default saving behavior (savePath omitted) still writes `preview-*.png` under `.dx/mcp/`.
            const defaultShot = await client.callTool({
                name: 'screenshot_preview',
                arguments: {
                    url,
                    mode: 'page',
                    open: false,
                    waitMs: 500,
                    savePath: undefined
                }
            });
            console.log('default screenshot_preview isError:', defaultShot.isError ?? false);
            const dxMcpDir = path.join(workspaceRoot, '.dx', 'mcp');
            const previewFiles = fs
                .readdirSync(dxMcpDir)
                .filter(f => /^preview-.*\.png$/i.test(f));
            console.log('default preview-*.png count:', previewFiles.length);
        }
    }

    // Optional: capture the native DX app window by process name (dx) without depending on foreground focus.
    try {
        const beforeFocus = await getForegroundWindowTitleWin32();
        console.log('foreground before screenshot_window:', JSON.stringify(beforeFocus ?? '(unknown)'));

        const dxAppPath = path.join(workspaceRoot, '.dx', 'mcp', 'dx-app.png');
        try {
            if (fs.existsSync(dxAppPath)) {
                fs.unlinkSync(dxAppPath);
            }
        } catch {
            // ignore
        }

        const dxShot = await client.callTool({
            name: 'screenshot_window',
            arguments: {
                title: 'dx',
                savePath: '.dx/mcp/dx-app.png',
                ocr: true,
                ocrLang: 'eng',
                allowRaise: false
            }
        });

        const afterFocus = await getForegroundWindowTitleWin32();
        console.log('foreground after screenshot_window:', JSON.stringify(afterFocus ?? '(unknown)'));
        if (beforeFocus && afterFocus) {
            console.log('foreground unchanged:', beforeFocus === afterFocus);
        }

        console.log('screenshot_window(title=dx) isError:', dxShot.isError ?? false);
        if (dxShot.isError) {
            const errText = (dxShot.content.find(c => c.type === 'text') as any)?.text ?? '';
            console.log('screenshot_window(title=dx) error text:', JSON.stringify(String(errText).slice(0, 4000)));
        } else {
            const dxOk = fs.existsSync(dxAppPath) && fs.statSync(dxAppPath).size > 0;
            console.log('dx-app.png present:', dxOk, `(${dxAppPath})`);
        }

        const winText = (dxShot.content.find(c => c.type === 'text') as any)?.text ?? '';
        const ocrIdx = winText.indexOf('OCR');
        const snippet = (ocrIdx >= 0 ? winText.slice(ocrIdx) : winText).slice(0, 240);
        console.log('screenshot_window(title=dx) OCR snippet:', JSON.stringify(snippet));
    } catch (e) {
        console.warn('native dx app window screenshot skipped:', e);
    }

    await client.close();
}

main().catch(err => {
    console.error(err);
    process.exitCode = 1;
});
