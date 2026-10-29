# Sends a mouse click at window-relative physical coordinates to the What
# Changed window, for exercising controls during visual verification.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/click-window.ps1 -X 1179 -Y 28
param(
  [int]$X,
  [int]$Y,
  [string]$ProcessName = "what-changed"
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Click {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

[void][Win32Click]::SetProcessDPIAware()
$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { Write-Error "no window for process '$ProcessName'"; exit 1 }

$r = New-Object Win32Click+RECT
[void][Win32Click]::GetWindowRect($proc.MainWindowHandle, [ref]$r)
[void][Win32Click]::SetForegroundWindow($proc.MainWindowHandle)
Start-Sleep -Milliseconds 200
[void][Win32Click]::SetCursorPos($r.Left + $X, $r.Top + $Y)
Start-Sleep -Milliseconds 100
# MOUSEEVENTF_LEFTDOWN (2) then MOUSEEVENTF_LEFTUP (4)
[Win32Click]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
[Win32Click]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
Write-Output "clicked ($X,$Y)"
