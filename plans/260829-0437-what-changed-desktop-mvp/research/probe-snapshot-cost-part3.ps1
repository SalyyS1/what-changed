# Probe part 3 — measure snapshot costs so the plan can choose real poll intervals.
$ErrorActionPreference = 'Continue'
function Timed($name, $block) {
  $t = [System.Diagnostics.Stopwatch]::StartNew()
  try { $r = & $block; $t.Stop(); Write-Output ("[{0,7:N1} ms] {1} :: {2}" -f $t.Elapsed.TotalMilliseconds, $name, $r) }
  catch { $t.Stop(); Write-Output ("[{0,7:N1} ms] {1} :: ERROR {2}" -f $t.Elapsed.TotalMilliseconds, $name, $_.Exception.Message) }
}

Write-Output "=== cold run ==="
Timed 'Process snapshot (Get-Process)'            { "n=" + @(Get-Process).Count }
Timed 'TCP listener snapshot'                     { "n=" + @(Get-NetTCPConnection -State Listen).Count }
Timed 'UDP endpoint snapshot'                     { "n=" + @(Get-NetUDPEndpoint).Count }
Timed 'Service snapshot (Get-Service)'            { "n=" + @(Get-Service).Count }
Timed 'Service config snapshot (Win32_Service)'   { "n=" + @(Get-CimInstance Win32_Service).Count }
Timed 'Uninstall registry scan (3 hives)'         { $c=0; foreach($p in @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall','HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall')){ $c += @(Get-ChildItem $p -ErrorAction SilentlyContinue).Count }; "subkeys=$c" }
Timed 'Driver snapshot (Win32_PnPSignedDriver)'    { "n=" + @(Get-CimInstance Win32_PnPSignedDriver).Count }
Timed 'Network config snapshot (adapters+DNS+route)' { "adapters=" + @(Get-NetAdapter).Count + " dns=" + @(Get-DnsClientServerAddress).Count + " routes=" + @(Get-NetRoute).Count }
Timed 'WUA history count'                          { $s=New-Object -ComObject Microsoft.Update.Session; "total=" + $s.CreateUpdateSearcher().GetTotalHistoryCount() }
Timed 'Scheduled task snapshot'                    { "n=" + @(Get-ScheduledTask).Count }

Write-Output "=== warm run (second pass, caches hot) ==="
Timed 'Process snapshot (warm)'                   { "n=" + @(Get-Process).Count }
Timed 'TCP listener snapshot (warm)'              { "n=" + @(Get-NetTCPConnection -State Listen).Count }
Timed 'Service snapshot (warm)'                   { "n=" + @(Get-Service).Count }
Timed 'Uninstall registry scan (warm)'            { $c=0; foreach($p in @('HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall','HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall','HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall')){ $c += @(Get-ChildItem $p -ErrorAction SilentlyContinue).Count }; "subkeys=$c" }
Timed 'Driver snapshot (warm)'                    { "n=" + @(Get-CimInstance Win32_PnPSignedDriver).Count }

Write-Output "NOTE: PowerShell/CIM overhead inflates these numbers substantially versus direct Win32 calls from Rust."
Write-Output "Treat them as UPPER BOUNDS, useful for ranking which snapshots are cheap vs expensive."
Write-Output "--- probe part 3 complete ---"
