<#!
.SYNOPSIS
  Collect bounded resource samples for the Phase 20 soak worksheet.

.DESCRIPTION
  The script never prints process command lines, usernames, hostnames or file
  paths.  Pass the What Changed PID with -ProcessId.  A simulator command may
  be supplied explicitly with -SimulationCommand; leaving it empty keeps the
  run read-only and records that synthetic writes were not driven.
  The command must throw or finish with a nonzero native exit code if any
  internal write fails; a successful shell command alone does not prove that
  collectors stayed healthy or that writes were committed.

  Example (short smoke run; exits review/nonzero by design):
    powershell -File scripts/soak.ps1 -Hours 0.01 -IntervalSeconds 5 -ProcessId 1234

  The release gate uses -Hours 4 and retains the generated CSV outside the
  repository.  A failed assertion is a non-zero exit code, never a silent
  pass.

  For the devtools-only backend runner, pass distinct .txt -TriggerFile and
  -AckFile paths directly inside its fresh reserved QA directory. The script
  waits for soak-ready.json and requires matching soak-result.json plus zero
  backend exit before a backend diagnostic can pass. This mode excludes native
  Tauri/WebView acceptance even after four hours. The
  runner acknowledges only after the shared sanitized simulation path commits;
  a timeout or negative acknowledgement is a failed run.
#>
[CmdletBinding()]
param(
  [double]$Hours = 4,
  [int]$IntervalSeconds = 60,
  [int]$ProcessId = 0,
  [string]$ProcessName = 'WhatChanged',
  [string]$SimulationCommand = '',
  [string]$TriggerFile = '',
  [string]$AckFile = '',
  [int]$ReadinessTimeoutSeconds = 120,
  [int]$BackendExitTimeoutSeconds = 120,
  [string]$Output = (Join-Path (Get-Location) 'phase20-soak.csv'),
  [switch]$AllowMissingProcess
)

$ErrorActionPreference = 'Stop'

if ($Hours -le 0 -or $Hours -gt 24) { throw 'Hours must be between 0 and 24.' }
if ($IntervalSeconds -le 0 -or $IntervalSeconds -gt 3600) { throw 'IntervalSeconds must be between 1 and 3600.' }
if ($ReadinessTimeoutSeconds -lt 1 -or $ReadinessTimeoutSeconds -gt 600) { throw 'ReadinessTimeoutSeconds must be between 1 and 600.' }
if ($BackendExitTimeoutSeconds -lt 1 -or $BackendExitTimeoutSeconds -gt 600) { throw 'BackendExitTimeoutSeconds must be between 1 and 600.' }

function Assert-PlainPath {
  param([string]$Path)
  if ($Path -notmatch '^[A-Za-z]:[\\/]') { throw 'Protocol and output paths must be absolute local paths.' }
  if ($Path -match '(^|[\\/])\.\.([\\/]|$)' -or $Path.Substring(2).Contains(':')) { throw 'Path traversal or alternate streams are forbidden.' }
  foreach ($component in $Path.Substring(3).Split([char[]]'\/')) {
    if ($component.EndsWith('.') -or $component.EndsWith(' ')) { throw 'Noncanonical path component.' }
  }
  $current = [IO.Path]::GetFullPath($Path)
  while ($current) {
    $item = Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue
    if ($null -ne $item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Reparse paths are forbidden.' }
    $current = Split-Path -Parent $current
  }
}

function Get-BackendProtocol {
  param([string]$Trigger, [string]$Ack)
  Assert-PlainPath $Trigger
  Assert-PlainPath $Ack
  $Trigger = [IO.Path]::GetFullPath($Trigger)
  $Ack = [IO.Path]::GetFullPath($Ack)
  $root = [IO.Path]::GetDirectoryName($Trigger)
  if ([IO.Path]::GetFileName($root) -notlike 'what-changed-perf-soak-?*' -or
      [IO.Path]::GetDirectoryName($Ack) -ine $root -or $Trigger -ieq $Ack) {
    throw 'Trigger and acknowledgement must be distinct direct children of the same reserved QA root.'
  }
  foreach ($file in @($Trigger, $Ack)) {
    if ([IO.Path]::GetFileName($file) -notmatch '^[A-Za-z0-9_.-]+\.txt$') { throw 'Protocol files must use ordinary .txt filenames.' }
  }
  [pscustomobject]@{ Root = $root; Trigger = $Trigger; Ack = $Ack; Ready = Join-Path $root 'soak-ready.json'; Report = Join-Path $root 'soak-result.json' }
}

function Assert-DistinctBackendOutput {
  param([string]$Path, $Protocol)
  $normalized = [IO.Path]::GetFullPath($Path)
  if ($normalized -in @($Protocol.Trigger, $Protocol.Ack, $Protocol.Ready, $Protocol.Report)) {
    throw 'Output collides with a protocol file.'
  }
}

function Read-BackendJson {
  param([string]$Path)
  Assert-PlainPath $Path
  if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'Required backend evidence is missing.' }
  if ((Get-Item -LiteralPath $Path).Length -gt 65536) { throw 'Backend evidence exceeds the bounded size.' }
  [IO.File]::ReadAllText($Path) | ConvertFrom-Json
}

function Assert-BackendResult {
  param($Ready, $Report, [int]$ExpectedPid, [double]$Seconds, [int]$ExitCode)
  if ($ExitCode -ne 0 -or $null -eq $Report -or $null -eq $Ready) { throw 'Backend report and successful exit are both required.' }
  if ($Ready.schema_version -ne 1 -or $Report.schema_version -ne 1 -or
      $Ready.mode -ne 'backend-only' -or $Report.mode -ne 'backend-only' -or
      [string]::IsNullOrWhiteSpace($Ready.run_id) -or $Report.run_id -ne $Ready.run_id -or
      $Ready.process_id -ne $ExpectedPid -or $Report.process_id -ne $ExpectedPid -or
      $Report.status -ne 'pass' -or $Report.ready -ne $true -or $Report.shutdown_complete -ne $true -or
      $Report.observed_seconds -lt $Seconds -or $Report.requested_seconds -lt $Seconds -or
      $Report.requested_seconds -ne $Ready.requested_seconds -or $Report.writer_loss -ne 0 -or
      $Report.triggers_committed -lt 1 -or $Report.outcomes.monitoring_paused -ne $false) {
    throw 'Backend run identity, completion, or health evidence failed.'
  }
  $expected = @($Ready.expected_collectors | Sort-Object -Unique)
  if ($expected.Count -eq 0 -or $expected.Count -ne @($Ready.expected_collectors).Count -or
      (@($Report.expected_collectors | Sort-Object) -join '|') -ne ($expected -join '|')) {
    throw 'Backend collector denominator does not match readiness.'
  }
  foreach ($id in $expected) {
    $count = $Report.outcomes.successful_snapshots.PSObject.Properties[$id]
    if ($null -eq $count -or $count.Value -lt 1) { throw 'A collector has no successful snapshot evidence.' }
  }
  if (@($Report.outcomes.failure_observations.PSObject.Properties).Count -ne 0 -or
      @($Report.outcomes.successful_snapshots.PSObject.Properties | Where-Object { $_.Name -notin $expected }).Count -ne 0) {
    throw 'Backend lifetime outcome evidence contains failures or unknown collectors.'
  }
}

function Get-ProcessTreeIds {
  param([int]$RootPid)
  # ParentProcessId is the only process metadata needed here.  Do not query
  # CommandLine, ExecutablePath or user identity: the CSV is deliberately
  # safe to attach to a bug report.
  # An incomplete census would under-report the app's memory and handles.
  # Treat CIM failure (or a missing root) as invalid evidence.
  $rows = @(Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId -ErrorAction Stop)
  if (-not @($rows | Where-Object { [int]$_.ProcessId -eq $RootPid }).Count) {
    throw 'The process-tree census did not include the root process.'
  }
  $children = @{}
  foreach ($row in $rows) {
    $parent = [int]$row.ParentProcessId
    if (-not $children.ContainsKey($parent)) { $children[$parent] = [System.Collections.Generic.List[int]]::new() }
    [void]$children[$parent].Add([int]$row.ProcessId)
  }
  $ids = [System.Collections.Generic.List[int]]::new()
  [void]$ids.Add($RootPid)
  $cursor = 0
  while ($cursor -lt $ids.Count) {
    $current = $ids[$cursor]
    $cursor++
    if ($children.ContainsKey($current)) {
      foreach ($child in $children[$current]) {
        if (-not $ids.Contains($child)) { [void]$ids.Add($child) }
      }
    }
  }
  return $ids
}

function Get-Sample {
  param([int]$RootPid)
  $root = Get-Process -Id $RootPid -ErrorAction Stop
  if ($null -eq $root) { throw 'The root process exited during the soak.' }
  $treeIds = @(Get-ProcessTreeIds -RootPid $RootPid)
  # A missing WebView or helper child would make every aggregate artificially
  # small. Reject the whole sample if any census PID cannot be resolved.
  $processes = @(foreach ($id in $treeIds) {
    Get-Process -Id $id -ErrorAction Stop
  })
  if (-not @($processes | Where-Object { $_.Id -eq $RootPid }).Count) {
    throw 'The root process exited during process-tree sampling.'
  }
  # PrivateMemorySize64 is private commit, not private working set. The B2
  # figure comes from the per-process performance counter keyed by PID; no
  # process name, path, command line or user identity leaves this function.
  $privateWorkingByPid = @{}
  try {
    $perfRows = @(Get-CimInstance -ClassName Win32_PerfRawData_PerfProc_Process `
      -Property IDProcess, WorkingSetPrivate -ErrorAction Stop)
    foreach ($row in $perfRows) {
      if ($null -ne $row.IDProcess -and $null -ne $row.WorkingSetPrivate) {
        $privateWorkingByPid[[int]$row.IDProcess] = [int64]$row.WorkingSetPrivate
      }
    }
  } catch {
    # Keep the total working set sample; mark private working set unavailable.
  }
  $private = [int64]0
  $privateCommit = [int64]0
  $working = [int64]0
  $handles = [int64]0
  $threads = [int64]0
  $cpu = [double]0
  $workingKnown = $true
  $privateKnown = $true
  $privateCommitKnown = $true
  $handlesKnown = $true
  $threadsKnown = $true
  $cpuKnown = $true
  foreach ($process in $processes) {
    if ($privateWorkingByPid.ContainsKey([int]$process.Id)) {
      $private += $privateWorkingByPid[[int]$process.Id]
    } else {
      $privateKnown = $false
    }
    try { $privateCommit += [int64]$process.PrivateMemorySize64 } catch { $privateCommitKnown = $false }
    try { $working += [int64]$process.WorkingSet64 } catch { $workingKnown = $false }
    try { $handles += [int64]$process.HandleCount } catch { $handlesKnown = $false }
    try { $threads += [int64]$process.Threads.Count } catch { $threadsKnown = $false }
    try { $cpu += [double]$process.CPU } catch { $cpuKnown = $false }
  }
  [pscustomobject]@{
    timestamp_utc = [DateTime]::UtcNow.ToString('o')
    process_id = $RootPid
    process_count = $processes.Count
    working_set_bytes = if ($workingKnown) { $working } else { $null }
    private_working_set_bytes = if ($privateKnown) { $private } else { $null }
    private_commit_bytes = if ($privateCommitKnown) { $privateCommit } else { $null }
    handle_count = if ($handlesKnown) { $handles } else { $null }
    thread_count = if ($threadsKnown) { $threads } else { $null }
    cpu_seconds_total = if ($cpuKnown) { $cpu } else { $null }
  }
}

$script:TriggerFailurePhase = 'idle'

function Invoke-TriggerFile {
  param([string]$Path, [string]$AckPath, [int]$Sequence)
  $script:TriggerFailurePhase = 'validate'
  $ack = if ([string]::IsNullOrWhiteSpace($AckPath)) { "$Path.ack" } else { $AckPath }
  $null = Get-BackendProtocol -Trigger $Path -Ack $ack
  $script:TriggerFailurePhase = 'cleanup'
  if ((Test-Path -LiteralPath $ack) -or (Test-Path -LiteralPath $Path)) { throw 'trigger_stale_protocol' }
  $kinds = @('process_crash', 'port_change', 'dns_change', 'config_diff', 'driver_update', 'docker_restart')
  $kind = $kinds[$Sequence % $kinds.Count]
  $nonce = "{0}-{1}" -f $PID, ([guid]::NewGuid().ToString('N'))
  $temporary = "$Path.$nonce.tmp"
  $script:TriggerFailurePhase = 'write'
  try {
    $stream = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try {
      $bytes = [Text.Encoding]::UTF8.GetBytes("$nonce|$kind")
      $stream.Write($bytes, 0, $bytes.Length)
      $stream.Flush($true)
    } finally { $stream.Dispose() }
    [IO.File]::Move($temporary, $Path)
  } catch { throw 'trigger_write' }
  $script:TriggerFailurePhase = 'await'
  $deadline = (Get-Date).AddSeconds(20)
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $ack) {
      Assert-PlainPath $ack
      $script:TriggerFailurePhase = 'ack'
      try { $reply = [IO.File]::ReadAllText($ack).Trim() } catch { throw 'trigger_ack_read' }
      if ($reply.StartsWith("$nonce|ok|", [StringComparison]::Ordinal)) {
        $script:TriggerFailurePhase = 'cleanup'
        try { Remove-Item -LiteralPath $ack -Force } catch { throw 'trigger_ack_cleanup' }
        return
      }
      if ($reply.StartsWith("$nonce|error|", [StringComparison]::Ordinal)) {
        $script:TriggerFailurePhase = 'cleanup'
        try { Remove-Item -LiteralPath $ack -Force } catch { throw 'trigger_ack_cleanup' }
        throw 'headless soak trigger returned an error.'
      }
    }
    Start-Sleep -Milliseconds 100
  }
  throw 'headless soak trigger timed out.'
}

$explicitProcessId = $ProcessId -gt 0
if (-not $explicitProcessId) {
  $candidates = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
  if ($candidates.Count -gt 1) {
    throw 'More than one candidate process exists; pass the exact -ProcessId.'
  }
  if ($candidates.Count -eq 1) { $ProcessId = $candidates[0].Id }
}
if ($ProcessId -le 0 -and -not $AllowMissingProcess) {
  throw 'Pass -ProcessId (or start the app by -ProcessName), or explicitly use -AllowMissingProcess.'
}

$samples = [System.Collections.Generic.List[object]]::new()
$duration = [TimeSpan]::FromHours($Hours)
$simulationEnabled = -not [string]::IsNullOrWhiteSpace($SimulationCommand)
$triggerEnabled = -not [string]::IsNullOrWhiteSpace($TriggerFile)
Assert-PlainPath $Output
if (Test-Path -LiteralPath $Output) { throw 'Output must be a fresh file.' }
if ($triggerEnabled -and (-not $explicitProcessId -or [string]::IsNullOrWhiteSpace($AckFile) -or $simulationEnabled)) {
  throw 'Backend mode requires an explicit PID and acknowledgement path, with no separate SimulationCommand.'
}
$simulationFailures = 0
$sampleFailure = $false
$sampleRetries = 0
$rootStartTime = $null
if ($ProcessId -gt 0) {
  $rootProcess = Get-Process -Id $ProcessId -ErrorAction Stop
  if ($rootProcess.ProcessName -ne $ProcessName) {
    throw 'The selected PID does not match -ProcessName.'
  }
  $rootStartTime = $rootProcess.StartTime
  # Retain an OS process handle now; a later PID lookup cannot prove its exit
  # code or distinguish an exited process from a reused PID.
  $null = $rootProcess.Handle
}
if ($triggerEnabled) {
  $protocol = Get-BackendProtocol -Trigger $TriggerFile -Ack $AckFile
  Assert-DistinctBackendOutput $Output $protocol
  $readyDeadline = [DateTime]::UtcNow.AddSeconds($ReadinessTimeoutSeconds)
  while (-not (Test-Path -LiteralPath $protocol.Ready)) {
    if ($rootProcess.HasExited) { throw 'Backend exited before readiness.' }
    if ([DateTime]::UtcNow -ge $readyDeadline) { throw 'Backend readiness timed out.' }
    Start-Sleep -Milliseconds 100
  }
  $ready = Read-BackendJson $protocol.Ready
  if ($ready.schema_version -ne 1 -or $ready.mode -ne 'backend-only' -or $ready.process_id -ne $ProcessId -or
      [string]::IsNullOrWhiteSpace($ready.run_id) -or $ready.requested_seconds -lt $duration.TotalSeconds -or
      $ready.trigger_file -ne [IO.Path]::GetFileName($TriggerFile) -or $ready.ack_file -ne [IO.Path]::GetFileName($AckFile) -or
      $ready.ready_at_ms -lt ([DateTimeOffset]$rootStartTime).ToUnixTimeMilliseconds()) {
    throw 'Backend readiness identity or requested duration does not match this run.'
  }
}
$startedAt = [DateTime]::UtcNow
$deadline = $startedAt + $duration
$iteration = 0

while ([DateTime]::UtcNow -lt $deadline -or $iteration -eq 0) {
  if ($ProcessId -gt 0) {
    $sampleSucceeded = $false
    for ($attempt = 0; $attempt -lt 3; $attempt++) {
      try {
        # PID reuse after an app crash is not a continuation of the same run.
        $currentRoot = Get-Process -Id $ProcessId -ErrorAction Stop
        if ($currentRoot.StartTime -ne $rootStartTime -or $currentRoot.ProcessName -ne $ProcessName) {
          throw 'The root PID was reused during the soak.'
        }
        # Retry only with a fresh census; never aggregate a partially resolved
        # tree when a short-lived WebView/helper exits during a sample.
        $sample = Get-Sample -RootPid $ProcessId
        $samples.Add($sample)
        $sampleSucceeded = $true
        break
      } catch {
        if ($attempt -eq 2) {
          $sampleFailure = $true
          Write-Warning 'Process-tree sampling failed after retries; this run cannot pass.'
        } else {
          $sampleRetries++
          Start-Sleep -Milliseconds 200
        }
      }
    }
    if (-not $sampleSucceeded) { break }
  }
  if ($simulationEnabled) {
    # Caller owns the command and its arguments. Do not interpolate app data
    # into it; this is a QA hook, not a shell bridge from the UI.
    try {
      $global:LASTEXITCODE = 0
      & ([scriptblock]::Create($SimulationCommand)) | Out-Null
      # Pipeline success ($?) reflects Out-Null, not the native driver. The
      # driver contract above requires fail-fast internal errors.
      if ($LASTEXITCODE -ne 0) { throw 'The simulation command exited nonzero.' }
    } catch {
      $simulationFailures++
      Write-Warning 'Simulation command failed; this run cannot pass.'
    }
  }
  if ($triggerEnabled) {
    try {
      Invoke-TriggerFile -Path $TriggerFile -AckPath $AckFile -Sequence $iteration
    } catch {
      $simulationFailures++
      $reason = $script:TriggerFailurePhase
      $exceptionType = $_.Exception.GetType().Name
      Write-Warning "Headless soak trigger failed (${reason}:${exceptionType}); this run cannot pass."
    }
  }
  $iteration++
  if ([DateTime]::UtcNow -ge $deadline) { break }
  Start-Sleep -Seconds $IntervalSeconds
}
$completedAt = [DateTime]::UtcNow
$backendPass = $false
if ($triggerEnabled) {
  try {
    $exitDeadline = [DateTime]::UtcNow.AddSeconds($BackendExitTimeoutSeconds)
    while (-not $rootProcess.WaitForExit(200)) {
      if ([DateTime]::UtcNow -ge $exitDeadline) { throw 'Backend did not exit within the completion timeout.' }
    }
    $report = Read-BackendJson $protocol.Report
    Assert-BackendResult -Ready $ready -Report $report -ExpectedPid $ProcessId -Seconds $duration.TotalSeconds -ExitCode $rootProcess.ExitCode
    $backendPass = $true
  } catch {
    Write-Warning 'Backend report or successful exit was not proven; this run cannot pass.'
  }
}

$parent = Split-Path -Parent $Output
if ($parent -and -not (Test-Path -LiteralPath $parent)) {
  New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
Assert-PlainPath $Output
if (Test-Path -LiteralPath $Output) { throw 'Output became occupied during the run.' }
$samples | Export-Csv -LiteralPath $Output -NoTypeInformation -Encoding UTF8 -NoClobber

if ($samples.Count -lt 2) {
  if ($AllowMissingProcess) {
    Write-Warning 'No process samples were collected; this is an evidence scaffold, not a pass.'
    exit 1
  }
  throw 'Fewer than two process samples were collected.'
}

$first = $samples[0]
$last = $samples[$samples.Count - 1]
function Number([object]$Value) { if ($null -eq $Value) { return 0.0 }; return [double]$Value }
$firstWorking = Number $first.working_set_bytes
$lastWorking = Number $last.working_set_bytes
$growth = if ($firstWorking -gt 0) { (($lastWorking - $firstWorking) / $firstWorking) * 100 } else { 0 }
$firstHandles = Number $first.handle_count
$lastHandles = Number $last.handle_count
$handleDelta = if ($firstHandles -gt 0) { (($lastHandles - $firstHandles) / $firstHandles) * 100 } else { 0 }
$firstThreads = Number $first.thread_count
$lastThreads = Number $last.thread_count
$threadDelta = $lastThreads - $firstThreads
$metricsKnown = @($samples | Where-Object {
  $null -eq $_.working_set_bytes -or $null -eq $_.handle_count -or $null -eq $_.thread_count
}).Count -eq 0

# A short diagnostic run is useful, but it is never four-hour acceptance.
# Allow up to five percent missed slots for sampling overhead or scheduler
# jitter while requiring the full requested wall-clock duration.
$minimumSamples = [Math]::Floor([Math]::Ceiling($duration.TotalSeconds / $IntervalSeconds) * 0.95)
$durationPass = $explicitProcessId -and $ProcessName -eq 'WhatChanged' -and
  $Hours -ge 4 -and $IntervalSeconds -le 60 -and
  ($completedAt - $startedAt) -ge $duration -and $samples.Count -ge $minimumSamples

$growthPass = $growth -le 15
$handlesPass = [Math]::Abs($handleDelta) -le 5
$threadsPass = $threadDelta -eq 0
$status = if ($metricsKnown -and $durationPass -and $growthPass -and $handlesPass -and $threadsPass -and ($simulationEnabled -or $triggerEnabled) -and
  (-not $triggerEnabled -or $backendPass) -and
  $simulationFailures -eq 0 -and -not $sampleFailure) { 'pass' } else { 'review' }
if ($triggerEnabled) { Write-Output 'scope=backend-only native_webview_acceptance=false' }

Write-Output ("root_pid={0} samples={1} minimum_samples={2} duration_complete={3} working_set_growth_percent={4:N2} handle_delta_percent={5:N2} thread_delta={6} simulation_driver={7} simulation_failures={8} sample_retries={9} sample_failure={10} status={11}" -f `
  $ProcessId, $samples.Count, $minimumSamples, $durationPass, $growth, $handleDelta, $threadDelta, ($simulationEnabled -or $triggerEnabled), $simulationFailures, $sampleRetries, $sampleFailure, $status)
if (-not $durationPass) { Write-Warning 'Explicit PID plus four-hour sampling duration/cadence was not met; this run is review-only.' }
if (-not $simulationEnabled -and -not $triggerEnabled) { Write-Warning 'No simulation or trigger driver was supplied; collector/write assertions remain manual.' }
if (-not $metricsKnown) { Write-Warning 'One or more process-tree counters were unavailable; this run is review-only.' }
if ($samples.Where({ $null -eq $_.private_working_set_bytes }).Count -gt 0) {
  Write-Warning 'Private working set was unavailable for at least one process sample; B2 private-working-set evidence remains open.'
}
if ($status -ne 'pass') { exit 1 }
