param([string]$NodeExecutable = 'C:\Program Files\nodejs\node.exe')
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
# Verify the candidate's bytes before executing it for any purpose.
$checksums = (Invoke-WebRequest -Uri 'https://nodejs.org/dist/v24.12.0/SHASUMS256.txt' -UseBasicParsing).Content
$expectedLine = ($checksums -split "`n" | Where-Object { $_.Trim() -match ' win-x64/node.exe$' }).Trim()
if (-not $expectedLine) { throw 'Official Node checksum not found' }
$expectedNodeHash = ($expectedLine -split '\s+')[0]
if ((Get-FileHash -LiteralPath $NodeExecutable -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedNodeHash) {
  throw 'Node executable differs from official checksum inventory'
}
& $NodeExecutable (Join-Path $repoRoot 'scripts\check-ownvero-approvals.mjs') --gates G0,G1
if ($LASTEXITCODE -ne 0) { throw 'Runner scope has not been approved' }
$nodeVersion = (& $NodeExecutable --version).Trim()
if ($nodeVersion -ne 'v24.12.0') { throw 'Spike pins Node v24.12.0; review before changing it' }
$qaRoot = Join-Path $repoRoot '.qa\ownvero-runtime-spike'
$currentBundle = Join-Path $qaRoot 'bundle'
$playwrightRoot = Join-Path $repoRoot 'node_modules\playwright-core'
$browserRoot = Join-Path $currentBundle 'browsers'

function Assert-RegularAncestors([string]$Path) {
  $cursor = [IO.Path]::GetFullPath($Path)
  while ($cursor) {
    if (Test-Path -LiteralPath $cursor) {
      if (((Get-Item -LiteralPath $cursor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Bundle path has a reparse boundary' }
    }
    $parent = [IO.Directory]::GetParent($cursor)
    $cursor = if ($null -ne $parent) { $parent.FullName } else { $null }
  }
}
function Get-RegularFiles([string]$Path) {
  Assert-RegularAncestors $Path
  $pending = [Collections.Generic.Queue[string]]::new()
  $pending.Enqueue($Path)
  while ($pending.Count) {
    foreach ($entry in Get-ChildItem -LiteralPath $pending.Dequeue() -Force) {
      if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Bundle tree contains a reparse entry' }
      if ($entry.PSIsContainer) { $pending.Enqueue($entry.FullName) } else { $entry }
    }
  }
}
$null = @(Get-RegularFiles $browserRoot)
$null = @(Get-RegularFiles $playwrightRoot)
if (-not (Test-Path -LiteralPath (Join-Path $browserRoot 'chromium-1234\chrome-win64\chrome.exe'))) {
  throw 'Install matched Chromium into the QA bundle first; no runtime download fallback'
}
$stagingId = [Guid]::NewGuid().ToString('N')
$bundleRoot = Join-Path $qaRoot ('bundle-stage-' + $stagingId)
Assert-RegularAncestors $bundleRoot
New-Item -ItemType Directory -Path $bundleRoot -ErrorAction Stop | Out-Null
Copy-Item -LiteralPath $NodeExecutable -Destination (Join-Path $bundleRoot 'node.exe') -Force
Copy-Item -LiteralPath $playwrightRoot -Destination (Join-Path $bundleRoot 'playwright-core') -Recurse
Copy-Item -LiteralPath $browserRoot -Destination (Join-Path $bundleRoot 'browsers') -Recurse
$playwrightVersion = (Get-Content -LiteralPath (Join-Path $bundleRoot 'playwright-core\package.json') -Raw | ConvertFrom-Json).version
if ($playwrightVersion -ne '1.62.1') { throw 'Spike pins Playwright 1.62.1' }
foreach ($name in @('packaged-runtime-smoke.mjs', 'booking-fixture.mjs')) {
  Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $bundleRoot $name) -Force
}
$runRoot = Join-Path $bundleRoot 'run'
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null
$inventory = @()
foreach ($file in Get-RegularFiles $bundleRoot) {
  $relative = $file.FullName.Substring($bundleRoot.Length + 1).Replace('\', '/')
  if ($relative -eq 'bundle-inventory.json' -or $relative.StartsWith('run/')) { continue }
  $inventory += [pscustomobject][ordered]@{ path = $relative; bytes = $file.Length; sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
}
$manifest = [ordered]@{
  schemaVersion = 1; nodeVersion = $nodeVersion; playwrightVersion = $playwrightVersion; chromiumRevision = '1234'
  browserExecutable = 'browsers/chromium-1234/chrome-win64/chrome.exe'
  checksumSource = 'https://nodejs.org/dist/v24.12.0/SHASUMS256.txt'
  installedBytes = ($inventory | Measure-Object -Property bytes -Sum).Sum
  files = @($inventory | Sort-Object -Property path)
}
[IO.File]::WriteAllText((Join-Path $bundleRoot 'bundle-inventory.json'), ($manifest | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
# Both moves are confined to this exact QA root; keep previous evidence intact.
$previousBundle = Join-Path $qaRoot ('bundle-previous-' + $stagingId)
foreach ($movePath in @($currentBundle, $bundleRoot, $previousBundle)) {
  $absolute = [IO.Path]::GetFullPath($movePath)
  if (-not $absolute.StartsWith($qaRoot + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Bundle promotion escaped QA root' }
  Assert-RegularAncestors $absolute
}
Move-Item -LiteralPath $currentBundle -Destination $previousBundle
Move-Item -LiteralPath $bundleRoot -Destination $currentBundle
[ordered]@{ nodeVersion = $nodeVersion; playwrightVersion = $playwrightVersion; chromiumRevision = '1234'; installedBytes = $manifest.installedBytes; fileCount = $inventory.Count; readyForOwnedProcessSmoke = $true; freshMachineProof = $false } | ConvertTo-Json
