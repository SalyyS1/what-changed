$ErrorActionPreference = 'Stop'
$runner = Join-Path $PSScriptRoot '../../../scripts/soak.ps1'
$tokens = $null
$errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile([IO.Path]::GetFullPath($runner), [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw ($errors.Message -join '; ') }
foreach ($name in @('Assert-PlainPath', 'Get-BackendProtocol', 'Assert-DistinctBackendOutput', 'Read-BackendJson', 'Assert-BackendResult', 'Invoke-TriggerFile')) {
  $function = $ast.Find({ param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name }, $true)
  if ($null -eq $function) { throw "Missing function: $name" }
  . ([scriptblock]::Create($function.Extent.Text))
}
function Must-Fail([scriptblock]$Action) {
  $failed = $false
  try { & $Action | Out-Null } catch { $failed = $true }
  if (-not $failed) { throw 'Expected rejection did not occur.' }
}
function Evidence {
  @'
{"schema_version":1,"mode":"backend-only","run_id":"test-run","process_id":42,"requested_seconds":14400,"expected_collectors":["docker","processes"],"ready":true,"shutdown_complete":true,"observed_seconds":14400,"status":"pass","writer_loss":0,"triggers_committed":2,"outcomes":{"successful_snapshots":{"docker":2,"processes":3},"failure_observations":{},"monitoring_paused":false}}
'@ | ConvertFrom-Json
}
$ready = Evidence
$report = Evidence
Assert-BackendResult $ready $report 42 14400 0
Must-Fail { Assert-BackendResult $ready $null 42 14400 0 }
Must-Fail { Assert-BackendResult $null $report 42 14400 0 }
Must-Fail { Assert-BackendResult $ready $report 42 14400 1 }
foreach ($field in @('run_id', 'process_id', 'status', 'ready', 'shutdown_complete', 'observed_seconds', 'writer_loss')) {
  $bad = Evidence
  switch ($field) {
    'run_id' { $bad.run_id = 'stale' }
    'process_id' { $bad.process_id = 43 }
    'status' { $bad.status = 'fail' }
    'ready' { $bad.ready = $false }
    'shutdown_complete' { $bad.shutdown_complete = $false }
    'observed_seconds' { $bad.observed_seconds = 30 }
    'writer_loss' { $bad.writer_loss = 1 }
  }
  Must-Fail { Assert-BackendResult $ready $bad 42 14400 0 }
}
$bad = Evidence
$bad.outcomes.successful_snapshots.docker = 0
Must-Fail { Assert-BackendResult $ready $bad 42 14400 0 }
$bad = Evidence
$bad.outcomes.failure_observations | Add-Member NoteProperty docker 1
Must-Fail { Assert-BackendResult $ready $bad 42 14400 0 }
$bad = Evidence
$bad.expected_collectors = @('processes')
Must-Fail { Assert-BackendResult $ready $bad 42 14400 0 }

$temporaryBase = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$root = Join-Path $temporaryBase ('what-changed-perf-soak-script-test-' + [guid]::NewGuid().ToString('N'))
[void][IO.Directory]::CreateDirectory($root)
try {
  $trigger = Join-Path $root 'trigger.txt'
  $ack = Join-Path $root 'ack.txt'
  $null = Get-BackendProtocol $trigger $ack
  Must-Fail { Get-BackendProtocol $trigger $trigger }
  Must-Fail { Get-BackendProtocol $trigger $trigger.Replace('\', '/').ToUpperInvariant() }
  $protocol = Get-BackendProtocol $trigger.Replace('\', '/') $ack.Replace('\', '/')
  foreach ($candidate in @($trigger, $ack, $protocol.Ready, $protocol.Report)) {
    Must-Fail { Assert-DistinctBackendOutput $candidate.Replace('\', '/').ToUpperInvariant() $protocol }
  }
  Assert-DistinctBackendOutput (Join-Path $root 'samples.csv') $protocol
  Must-Fail { Get-BackendProtocol $trigger (Join-Path $temporaryBase 'outside.txt') }
  Must-Fail { Assert-PlainPath (Join-Path $root '../escape.txt') }
  Must-Fail { Read-BackendJson (Join-Path $root 'missing.json') }
  [IO.File]::WriteAllText($ack, 'existing')
  Must-Fail { Invoke-TriggerFile $trigger $ack 0 }
  if ([IO.File]::ReadAllText($ack) -ne 'existing') { throw 'Existing acknowledgement was modified.' }
  $target = Join-Path $root 'target'
  [void][IO.Directory]::CreateDirectory($target)
  $alias = Join-Path $root 'alias'
  New-Item -ItemType Junction -Path $alias -Target $target | Out-Null
  Must-Fail { Assert-PlainPath (Join-Path $alias 'escape.txt') }
  # Remove the junction itself before recursive cleanup, preserving its target.
  [IO.Directory]::Delete($alias)
} finally {
  $resolved = [IO.Path]::GetFullPath($root)
  if (-not $resolved.StartsWith($temporaryBase, [StringComparison]::OrdinalIgnoreCase) -or
      [IO.Path]::GetFileName($resolved) -notlike 'what-changed-perf-soak-script-test-*') { throw 'Unsafe test cleanup path.' }
  Remove-Item -LiteralPath $resolved -Recurse -Force
}
Write-Output 'PASS: backend report, exit, readiness identity, collector loss, reuse, and reparse rejection.'
