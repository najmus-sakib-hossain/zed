
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


$hWnd = [IntPtr]::new(1507778)

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
    if (-not $true) {
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

$savePath = 'F:\Dx\.dx\mcp\window-2026-02-20T11-45-18-050Z-db2737d4.png'
if ($savePath -and $savePath.Length -gt 0) {
  $dir = [System.IO.Path]::GetDirectoryName($savePath)
  if ($dir -and -not (Test-Path -LiteralPath $dir)) {
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
  }
  [System.IO.File]::WriteAllBytes($savePath, $bytes)
}

    [pscustomobject]@{ pngBase64 = $b64; savedTo = $savePath; captureMethod = $captureMethod } | ConvertTo-Json -Depth 3
