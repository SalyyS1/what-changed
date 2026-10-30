<# Builds local runtime resources from a previously verified spike; no downloads. #>
[CmdletBinding()]
param(
  [string]$SourceBundle = 'runner/bundle',
  [string]$ResumeStage
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$source = [IO.Path]::GetFullPath((Join-Path $repoRoot $SourceBundle))
$outputRoot = Join-Path $repoRoot 'runner'
$stage = Join-Path $outputRoot ('bundle-stage-' + [Guid]::NewGuid().ToString('N'))
if ($ResumeStage) {
  $stage = [IO.Path]::GetFullPath((Join-Path $outputRoot $ResumeStage))
  if ([IO.Path]::GetDirectoryName($stage) -ne $outputRoot -or [IO.Path]::GetFileName($stage) -notmatch '^bundle-stage-[a-f0-9]{32}$') { throw 'Resume stage must be a generated directory directly inside runner' }
  if (-not (Test-Path -LiteralPath $stage -PathType Container)) { throw 'Resume stage does not exist' }
}
$destination = Join-Path $outputRoot 'bundle'
function Assert-RegularTree([string]$Path) {
  $pending = [Collections.Generic.Queue[string]]::new()
  $pending.Enqueue($Path)
  while ($pending.Count) {
    $entry = Get-Item -LiteralPath $pending.Dequeue() -Force
    if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Runtime source or destination is redirected' }
    if ($entry.PSIsContainer) { foreach ($child in Get-ChildItem -LiteralPath $entry.FullName -Force) { $pending.Enqueue($child.FullName) } }
  }
}
foreach ($path in @($source, $outputRoot)) {
  $cursor = $path
  while ($cursor) { if (Test-Path -LiteralPath $cursor) { if (((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Runtime ancestor is redirected' } }; $parent = [IO.Directory]::GetParent($cursor); $cursor = if ($parent) { $parent.FullName } else { $null } }
}
Assert-RegularTree $source
$inventory = Get-Content -LiteralPath (Join-Path $source 'bundle-inventory.json') -Raw | ConvertFrom-Json
if ($inventory.nodeVersion -ne 'v24.12.0' -or $inventory.playwrightVersion -ne '1.62.1' -or $inventory.chromiumRevision -ne '156.0.8078.47-1.1' -or $inventory.browserExecutable -ne 'browsers/chromium-156.0.8078.47-1.1/chrome.exe' -or $inventory.checksumSource -ne 'https://nodejs.org/dist/v24.12.0/SHASUMS256.txt') { throw 'Source runtime differs from reviewed versions or checksum source' }
Write-Host "Preparing runner stage: $stage"
$runtimeEntries = @{}
foreach ($entry in $inventory.files) {
  if ($entry.path -match '(^|/)\.\.?(/|$)|\\|:' -or [IO.Path]::IsPathRooted($entry.path)) { throw 'Unsafe source inventory path' }
  $file = Get-Item -LiteralPath (Join-Path $source $entry.path)
  if ($file.PSIsContainer -or $file.Length -ne $entry.bytes -or (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw 'Spike bytes changed' }
  if ($entry.path -eq 'node.exe' -or $entry.path.StartsWith('playwright-core/') -or $entry.path.StartsWith('browsers/chromium-156.0.8078.47-1.1/')) { $runtimeEntries[$entry.path] = $entry }
}
if (-not $ResumeStage) {
  New-Item -ItemType Directory -Path $stage | Out-Null
  # Copy only inventoried runtime bytes: execution may have added caches beside
  # the verified source. Host .links and unused browser tools are excluded.
  foreach ($entry in $runtimeEntries.Values) {
    $target = Join-Path $stage $entry.path
    New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($target)) -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $source $entry.path) -Destination $target
  }
}
Assert-RegularTree $stage
# Refresh source without nesting src/src or copying the large runtime again.
$sourceRoot = Join-Path $repoRoot 'runner/src'
Assert-RegularTree $sourceRoot
foreach ($file in Get-ChildItem -LiteralPath $sourceRoot -File -Recurse -Force) {
  $target = Join-Path (Join-Path $stage 'src') $file.FullName.Substring($sourceRoot.Length + 1)
  New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($target)) -Force | Out-Null
  Copy-Item -LiteralPath $file.FullName -Destination $target -Force
}
$entries = @(foreach ($file in Get-ChildItem -LiteralPath $stage -File -Recurse -Force) {
  $name = $file.FullName.Substring($stage.Length + 1).Replace('\','/')
  if ($name -eq 'bundle-inventory.json') { continue }
  $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  if (-not $name.StartsWith('src/')) {
    $expected = $runtimeEntries[$name]
    if (-not $expected -or $expected.bytes -ne $file.Length -or $expected.sha256 -ne $hash) { throw "Staged runtime differs from verified source: $name" }
    $runtimeEntries.Remove($name)
  }
  [pscustomobject][ordered]@{ path = $name; bytes = $file.Length; sha256 = $hash }
})
if ($runtimeEntries.Count -ne 0) { throw 'Staged runtime is incomplete' }
$manifest = [ordered]@{ schemaVersion = 1; nodeVersion = $inventory.nodeVersion; playwrightVersion = $inventory.playwrightVersion; chromiumRevision = $inventory.chromiumRevision; browserExecutable = $inventory.browserExecutable; checksumSource = $inventory.checksumSource; installedBytes = ($entries | Measure-Object -Property bytes -Sum).Sum; files = @($entries | Sort-Object -Property path) }
[IO.File]::WriteAllText((Join-Path $stage 'bundle-inventory.json'), ($manifest | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
& node (Join-Path $PSScriptRoot 'check-runner-bundle.mjs') $stage
if ($LASTEXITCODE -ne 0) { throw 'Staged bundle verification failed; stage preserved for inspection' }
foreach ($target in @($stage, $destination)) { if (-not ([IO.Path]::GetFullPath($target)).StartsWith($outputRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Promotion escaped runner root' } }
if (Test-Path -LiteralPath $destination) { Assert-RegularTree $destination; Move-Item -LiteralPath $destination -Destination (Join-Path $outputRoot ('bundle-previous-' + [Guid]::NewGuid().ToString('N'))) }
Move-Item -LiteralPath $stage -Destination $destination
Write-Host 'Local runner resources prepared. Installed/runtime, confinement and third-party notice review remain separate gates.'
