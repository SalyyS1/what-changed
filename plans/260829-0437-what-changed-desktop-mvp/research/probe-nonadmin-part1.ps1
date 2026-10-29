# Non-admin capability probe for "What Changed" collector planning.
# Run as the normal (non-elevated) user. Prints PASS/FAIL per capability.
$ErrorActionPreference = 'Continue'
function Rep($name, $ok, $detail) {
  $status = if ($ok) { 'PASS' } else { 'FAIL' }
  Write-Output ("[{0}] {1} :: {2}" -f $status, $name, $detail)
}

# --- 1. WMI process start trace ---
try {
  Register-CimIndicationEvent -Query "SELECT * FROM Win32_ProcessStartTrace" -SourceIdentifier 'wcProcStart' -ErrorAction Stop | Out-Null
  Start-Process -FilePath "cmd.exe" -ArgumentList "/c ver" -WindowStyle Hidden | Out-Null
  Start-Sleep -Seconds 3
  $ev = Get-Event -SourceIdentifier 'wcProcStart' -ErrorAction SilentlyContinue
  Rep 'WMI Win32_ProcessStartTrace subscribe' ($null -ne $ev) ("events captured: " + (@($ev).Count))
  Unregister-Event -SourceIdentifier 'wcProcStart' -ErrorAction SilentlyContinue
} catch { Rep 'WMI Win32_ProcessStartTrace subscribe' $false $_.Exception.Message }

# --- 2. WMI instance-creation polling event on Win32_Process ---
try {
  Register-CimIndicationEvent -Query "SELECT * FROM __InstanceCreationEvent WITHIN 1 WHERE TargetInstance ISA 'Win32_Process'" -SourceIdentifier 'wcProcCreate' -ErrorAction Stop | Out-Null
  Start-Process -FilePath "cmd.exe" -ArgumentList "/c ver" -WindowStyle Hidden | Out-Null
  Start-Sleep -Seconds 3
  $ev2 = Get-Event -SourceIdentifier 'wcProcCreate' -ErrorAction SilentlyContinue
  Rep 'WMI __InstanceCreationEvent Win32_Process' ($null -ne $ev2) ("events captured: " + (@($ev2).Count))
  Unregister-Event -SourceIdentifier 'wcProcCreate' -ErrorAction SilentlyContinue
} catch { Rep 'WMI __InstanceCreationEvent Win32_Process' $false $_.Exception.Message }

# --- 3. Process enumeration + exe path + version + publisher ---
try {
  $procs = @(Get-Process -ErrorAction SilentlyContinue)
  $withPath = @($procs | Where-Object { $_.Path }).Count
  Rep 'Process enumeration' $true ("total=" + $procs.Count + " with-readable-Path=" + $withPath)
} catch { Rep 'Process enumeration' $false $_.Exception.Message }
try {
  $p = @(Get-Process -Name explorer -ErrorAction Stop)[0]
  $fv = $p.MainModule.FileVersionInfo
  Rep 'File version + publisher read' $true ("explorer.exe ver=" + $fv.FileVersion + " company=" + $fv.CompanyName)
} catch { Rep 'File version + publisher read' $false $_.Exception.Message }
try {
  $sig = Get-AuthenticodeSignature "$env:WINDIR\explorer.exe" -ErrorAction Stop
  Rep 'Authenticode signature read' $true ("status=" + $sig.Status)
} catch { Rep 'Authenticode signature read' $false $_.Exception.Message }

# --- 4. TCP/UDP listeners with owning PID ---
try {
  $tcp = @(Get-NetTCPConnection -State Listen -ErrorAction Stop)
  $s = $tcp[0]
  Rep 'TCP listeners + owning PID' $true ("listeners=" + $tcp.Count + " sample=" + $s.LocalAddress + ":" + $s.LocalPort + " pid=" + $s.OwningProcess)
} catch { Rep 'TCP listeners + owning PID' $false $_.Exception.Message }
try {
  $udp = @(Get-NetUDPEndpoint -ErrorAction Stop)
  Rep 'UDP endpoints + owning PID' $true ("endpoints=" + $udp.Count)
} catch { Rep 'UDP endpoints + owning PID' $false $_.Exception.Message }

# --- 5. Services ---
try {
  $svc = @(Get-Service -ErrorAction Stop)
  Rep 'Service enumeration' $true ("services=" + $svc.Count)
} catch { Rep 'Service enumeration' $false $_.Exception.Message }
try {
  $cfg = @(Get-CimInstance -ClassName Win32_Service -ErrorAction Stop)
  $n = @($cfg | Where-Object { $_.PathName }).Count
  Rep 'Service config (StartMode/PathName)' $true ("rows=" + $cfg.Count + " with-PathName=" + $n)
} catch { Rep 'Service config (StartMode/PathName)' $false $_.Exception.Message }
try {
  $a = Get-Service -Name Audiosrv -ErrorAction Stop
  Rep 'Specific service query (Audiosrv)' $true ("status=" + $a.Status + " startType=" + $a.StartType)
} catch { Rep 'Specific service query (Audiosrv)' $false $_.Exception.Message }

Write-Output "--- probe part 1 complete ---"
