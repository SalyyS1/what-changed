# Non-admin capability probe part 2 — event log, updates, registry, drivers, network.
$ErrorActionPreference = 'Continue'
function Rep($name, $ok, $detail) {
  $status = if ($ok) { 'PASS' } else { 'FAIL' }
  Write-Output ("[{0}] {1} :: {2}" -f $status, $name, $detail)
}

# --- 6. Event Log channels ---
foreach ($ch in @('Application','System','Security','Microsoft-Windows-WindowsUpdateClient/Operational','Setup')) {
  try {
    $r = @(Get-WinEvent -LogName $ch -MaxEvents 3 -ErrorAction Stop)
    Rep ("EventLog read: " + $ch) $true ("records=" + $r.Count + " newest=" + $r[0].TimeCreated + " id=" + $r[0].Id)
  } catch { Rep ("EventLog read: " + $ch) $false $_.Exception.Message }
}
try {
  $crash = @(Get-WinEvent -FilterHashtable @{LogName='Application'; ProviderName='Application Error'; Id=1000} -MaxEvents 3 -ErrorAction Stop)
  Rep 'EventLog XPath-ish filter (Application Error 1000)' $true ("found=" + $crash.Count)
} catch { Rep 'EventLog XPath-ish filter (Application Error 1000)' $false $_.Exception.Message }
try {
  $boot = @(Get-WinEvent -FilterHashtable @{LogName='System'; Id=6005,6006,6008,41,1074} -MaxEvents 5 -ErrorAction Stop)
  Rep 'EventLog boot/shutdown IDs (System 6005/6006/6008/41/1074)' $true ("found=" + $boot.Count + " newest=" + $boot[0].Id + " @ " + $boot[0].TimeCreated)
} catch { Rep 'EventLog boot/shutdown IDs' $false $_.Exception.Message }

# --- 7. Windows Update history ---
try {
  $s = New-Object -ComObject Microsoft.Update.Session
  $sr = $s.CreateUpdateSearcher()
  $cnt = $sr.GetTotalHistoryCount()
  $h = $sr.QueryHistory(0, [Math]::Min(3, $cnt))
  $first = if ($h.Count -gt 0) { $h.Item(0).Title + " | result=" + $h.Item(0).ResultCode + " | " + $h.Item(0).Date } else { "none" }
  Rep 'WUA COM QueryHistory (non-admin)' $true ("totalHistory=" + $cnt + " newest=" + $first)
} catch { Rep 'WUA COM QueryHistory (non-admin)' $false $_.Exception.Message }
try {
  $qfe = @(Get-CimInstance Win32_QuickFixEngineering -ErrorAction Stop)
  Rep 'Win32_QuickFixEngineering' $true ("rows=" + $qfe.Count + " (known to under-report; compare with WUA count above)")
} catch { Rep 'Win32_QuickFixEngineering' $false $_.Exception.Message }
try {
  $up = (Get-CimInstance Win32_OperatingSystem -ErrorAction Stop).LastBootUpTime
  Rep 'Last boot time' $true ("lastBoot=" + $up)
} catch { Rep 'Last boot time' $false $_.Exception.Message }

# --- 8. Installed applications via registry Uninstall keys ---
$paths = @(
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
)
foreach ($rp in $paths) {
  try {
    $k = @(Get-ChildItem $rp -ErrorAction Stop)
    $named = @($k | ForEach-Object { $_.GetValue('DisplayName') } | Where-Object { $_ })
    Rep ("Registry Uninstall read: " + $rp) $true ("subkeys=" + $k.Count + " withDisplayName=" + $named.Count)
  } catch { Rep ("Registry Uninstall read: " + $rp) $false $_.Exception.Message }
}

# --- 9. Startup entries ---
foreach ($rp in @('HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run','HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Run','HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run')) {
  try {
    $k = Get-Item $rp -ErrorAction Stop
    Rep ("Registry read: " + $rp) $true ("values=" + $k.ValueCount)
  } catch { Rep ("Registry read: " + $rp) $false $_.Exception.Message }
}
try {
  $sf = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup'
  $items = @(Get-ChildItem $sf -ErrorAction Stop)
  Rep 'Per-user Startup folder' $true ($sf + " items=" + $items.Count)
} catch { Rep 'Per-user Startup folder' $false $_.Exception.Message }
try {
  $t = @(Get-ScheduledTask -ErrorAction Stop)
  $logon = @($t | Where-Object { $_.Triggers.CimClass.CimClassName -contains 'MSFT_TaskLogonTrigger' })
  Rep 'Task Scheduler enumeration' $true ("tasks=" + $t.Count + " logon-triggered=" + $logon.Count)
} catch { Rep 'Task Scheduler enumeration' $false $_.Exception.Message }

# --- 10. Drivers ---
try {
  $d = @(Get-CimInstance Win32_PnPSignedDriver -ErrorAction Stop)
  $gpu = @($d | Where-Object { $_.DeviceClass -eq 'DISPLAY' } | Select-Object -First 1)
  $sample = if ($gpu.Count -gt 0) { $gpu[0].DeviceName + " v" + $gpu[0].DriverVersion + " by " + $gpu[0].DriverProviderName } else { "no DISPLAY class row" }
  Rep 'Win32_PnPSignedDriver (version+provider+friendly name)' $true ("rows=" + $d.Count + " | GPU sample: " + $sample)
} catch { Rep 'Win32_PnPSignedDriver' $false $_.Exception.Message }
try {
  $pnp = @(Get-PnpDevice -PresentOnly -ErrorAction Stop)
  Rep 'Get-PnpDevice enumeration' $true ("present devices=" + $pnp.Count)
} catch { Rep 'Get-PnpDevice enumeration' $false $_.Exception.Message }

# --- 11. Network config ---
try {
  $ad = @(Get-NetAdapter -ErrorAction Stop)
  Rep 'Network adapter enumeration' $true ("adapters=" + $ad.Count)
} catch { Rep 'Network adapter enumeration' $false $_.Exception.Message }
try {
  $dns = @(Get-DnsClientServerAddress -AddressFamily IPv4 -ErrorAction Stop | Where-Object { $_.ServerAddresses.Count -gt 0 })
  Rep 'DNS server read' $true ("interfaces-with-DNS=" + $dns.Count + " sample=" + ($dns[0].ServerAddresses -join ','))
} catch { Rep 'DNS server read' $false $_.Exception.Message }
try {
  $rt = @(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction Stop)
  Rep 'Default gateway / route table read' $true ("default routes=" + $rt.Count + " gw=" + $rt[0].NextHop)
} catch { Rep 'Default gateway / route table read' $false $_.Exception.Message }
try {
  $ie = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' -ErrorAction Stop
  Rep 'Windows proxy config read (HKCU Internet Settings)' $true ("ProxyEnable=" + $ie.ProxyEnable + " ProxyServer=" + $ie.ProxyServer + " AutoConfigURL=" + $ie.AutoConfigURL)
} catch { Rep 'Windows proxy config read' $false $_.Exception.Message }

# --- 12. Docker ---
try {
  $pipe = Test-Path '\\.\pipe\docker_engine'
  Rep 'Docker engine named pipe present' $pipe '\\.\pipe\docker_engine'
} catch { Rep 'Docker engine named pipe present' $false $_.Exception.Message }

Write-Output "--- probe part 2 complete ---"
