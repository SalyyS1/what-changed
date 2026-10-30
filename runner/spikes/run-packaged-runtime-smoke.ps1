$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$bundleRoot = Join-Path $repoRoot '.qa\ownvero-runtime-spike\bundle'
$node = Join-Path $bundleRoot 'node.exe'
$inventory = Get-Content -LiteralPath (Join-Path $bundleRoot 'bundle-inventory.json') -Raw | ConvertFrom-Json
$nodeEntry = @($inventory.files | Where-Object path -eq 'node.exe')
if ($nodeEntry.Count -ne 1 -or (Get-FileHash -LiteralPath $node -Algorithm SHA256).Hash.ToLowerInvariant() -ne $nodeEntry[0].sha256) {
  throw 'Packaged Node integrity check failed before execution'
}
& $node (Join-Path $repoRoot 'scripts\check-ownvero-approvals.mjs') --gates G0,G1
if ($LASTEXITCODE -ne 0) { throw 'Runner scope has not been approved' }
Add-Type -Path (Join-Path $PSScriptRoot 'windows-owned-process.cs')
$runRoot = Join-Path $bundleRoot 'run'
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
$executionId = [Guid]::NewGuid().ToString('N')
$childEnvironment = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
$childEnvironment['SystemRoot'] = $env:SystemRoot
$childEnvironment['WINDIR'] = $env:WINDIR
$childEnvironment['TEMP'] = $runRoot
$childEnvironment['TMP'] = $runRoot
$childEnvironment['USERPROFILE'] = $runRoot
$childEnvironment['PATH'] = ''
$childEnvironment['PLAYWRIGHT_BROWSERS_PATH'] = Join-Path $bundleRoot 'browsers'
$childEnvironment['OWNVERO_SPIKE_EXECUTION_ID'] = $executionId
$job = $null
try {
  $job = [OwnveroSpike.OwnedProcess]::Start($node, [string[]]@((Join-Path $bundleRoot 'packaged-runtime-smoke.mjs')), $bundleRoot, $childEnvironment)
  if (-not $job.WaitForExit(90000)) { $job.Terminate(); throw 'Packaged browser smoke timed out' }
  $exitCode = $job.ExitCode
  $remaining = @($job.ActiveProcessIds())
  if ($remaining.Count -ne 0) { $job.Terminate(); throw 'Browser descendants survived the smoke' }
  $result = Get-Content -LiteralPath (Join-Path $runRoot 'smoke-result.json') -Raw | ConvertFrom-Json
  if ($result.executionId -ne $executionId) { throw 'Smoke result belongs to another execution' }
  if ($exitCode -ne 0 -or $result.status -ne 'passed') { throw 'Packaged runtime smoke failed; inspect structured QA result' }
  $result | Add-Member -NotePropertyName launcherExitCode -NotePropertyValue $exitCode
  $result | Add-Member -NotePropertyName remainingJobProcessCount -NotePropertyValue $remaining.Count
  $result | ConvertTo-Json -Depth 6
} finally {
  if ($null -ne $job) { $job.Dispose() }
}
