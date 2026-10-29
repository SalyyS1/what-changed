# Switches the running What Changed window to a sidebar destination by sending
# its Ctrl+<digit> shortcut, so a capture can show any route. The window is
# activated first because keyboard input only reaches the foreground window.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/navigate-window.ps1 -Shortcut 5
param(
  [ValidateRange(1, 7)][int]$Shortcut = 1,
  [string]$ProcessName = "what-changed"
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Nav {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
}
"@

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { Write-Error "no window for process '$ProcessName'"; exit 1 }

# SW_RESTORE (9) first: a minimized window captures as a black bitmap and
# receives no keyboard input.
[void][Win32Nav]::ShowWindow($proc.MainWindowHandle, 9)
Start-Sleep -Milliseconds 400
[void][Win32Nav]::SetForegroundWindow($proc.MainWindowHandle)
Start-Sleep -Milliseconds 300
[System.Windows.Forms.SendKeys]::SendWait("^$Shortcut")
Write-Output "sent Ctrl+$Shortcut to '$ProcessName'"
