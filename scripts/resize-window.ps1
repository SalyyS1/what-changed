# Resizes the What Changed window to a logical size without stealing focus,
# so small-window layouts can be captured with capture-window.ps1.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/resize-window.ps1 -Width 900 -Height 600
param(
  [int]$Width = 900,
  [int]$Height = 600,
  [string]$ProcessName = "what-changed"
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Resize {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
}
"@

[void][Win32Resize]::SetProcessDPIAware()
$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { Write-Error "no window for process '$ProcessName'"; exit 1 }

# SW_SHOWNOACTIVATE (4) leaves a maximized window restored without focusing it;
# SetWindowPos is ignored while maximized.
[void][Win32Resize]::ShowWindow($proc.MainWindowHandle, 4)
$dpi = [Win32Resize]::GetDpiForWindow($proc.MainWindowHandle)
$scale = $dpi / 96.0
$cx = [int]($Width * $scale); $cy = [int]($Height * $scale)
# SWP_NOZORDER (0x4) | SWP_NOACTIVATE (0x10)
[void][Win32Resize]::SetWindowPos($proc.MainWindowHandle, [IntPtr]::Zero, 80, 80, $cx, $cy, 0x14)
Write-Output "resized to ${Width}x${Height} logical (${cx}x${cy} physical at $dpi dpi)"
