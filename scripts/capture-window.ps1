# Captures the What Changed window to a PNG for visual verification.
# Uses PrintWindow so the capture comes from the target window's own surface,
# never from whatever happens to be on top of it on screen.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-window.ps1 -Out shot.png
param(
  [string]$Out = "window-capture.png",
  [string]$ProcessName = "what-changed"
)

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32 {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

# Without this, GetWindowRect returns DPI-virtualised coordinates and the
# bitmap clips the physical surface.
[void][Win32]::SetProcessDPIAware()

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { Write-Error "no window for process '$ProcessName'"; exit 1 }

$r = New-Object Win32+RECT
[void][Win32]::GetWindowRect($proc.MainWindowHandle, [ref]$r)
$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
# PW_RENDERFULLCONTENT (2) is required for WebView2/DirectComposition surfaces.
$ok = [Win32]::PrintWindow($proc.MainWindowHandle, $hdc, 2)
$g.ReleaseHdc($hdc)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
if (-not $ok) { Write-Warning "PrintWindow returned false; image may be blank" }
Write-Output "saved $Out ($w x $h)"
