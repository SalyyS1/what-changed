<#
.SYNOPSIS
  Runs release checks and produces a per-user NSIS installer.

.DESCRIPTION
  The default performance gate calls `bun run test:perf`, which checks query
  plans and runs the eight-shape query and database-size tests at 10k, 100k,
  and 1M events. Any failed scale stops release before installer creation.
  -SkipPerf is only for an explicitly incomplete local rehearsal; a run with
  that switch is not evidence that the Phase 20 performance gate passed.
#>
[CmdletBinding()]
param(
    [switch]$AllowDirty,
    [switch]$SkipInstaller,
    [switch]$SkipPerf,
    [switch]$IncludePreviewRunner
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
$env:CARGO_BUILD_JOBS = if ($env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS } else { "1" }
$targetDir = if ($env:CARGO_TARGET_DIR) {
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        $env:CARGO_TARGET_DIR
    } else {
        Join-Path $root $env:CARGO_TARGET_DIR
    }
} else {
    Join-Path $root "src-tauri/target"
}
$targetDir = [System.IO.Path]::GetFullPath($targetDir)
$gateResults = New-Object System.Collections.Generic.List[object]

function Invoke-Gate([string]$Label, [scriptblock]$Command) {
    Write-Host "[release] $Label" -ForegroundColor Cyan
    try {
        & $Command
        $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
        if ($exitCode -ne 0) { throw "release gate failed: $Label (exit $exitCode)" }
        [void]$gateResults.Add([pscustomobject]@{ Label = $Label; ExitCode = $exitCode; Status = "PASS" })
    } catch {
        $exitCode = if ($null -eq $LASTEXITCODE -or $LASTEXITCODE -eq 0) { 1 } else { [int]$LASTEXITCODE }
        [void]$gateResults.Add([pscustomobject]@{ Label = $Label; ExitCode = $exitCode; Status = "FAIL" })
        throw
    }
}

function Resolve-RequiredFile([string]$Path, [string]$Label) {
    $item = Get-Item -LiteralPath $Path -ErrorAction SilentlyContinue
    if (-not $item -or $item.PSIsContainer) { throw "release gate: $Label is missing: $Path" }
    return $item
}

function Resolve-RequiredDirectory([string]$Path, [string]$Label) {
    $item = Get-Item -LiteralPath $Path -ErrorAction SilentlyContinue
    if (-not $item -or -not $item.PSIsContainer) { throw "release gate: $Label is missing: $Path" }
    return $item
}

function Get-ManifestPath([System.IO.FileInfo]$Item) {
    $rootFull = [System.IO.Path]::GetFullPath($root).TrimEnd('\', '/')
    $itemFull = [System.IO.Path]::GetFullPath($Item.FullName)
    $prefix = $rootFull + [System.IO.Path]::DirectorySeparatorChar
    if ($itemFull.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $itemFull.Substring($prefix.Length).Replace('\', '/')
    }
    return $itemFull.Replace('\', '/')
}

if (-not $AllowDirty) {
    $dirty = git status --porcelain
    if ($dirty) {
        throw "release requires a clean worktree; use -AllowDirty only for a local rehearsal"
    }
}

Invoke-Gate "version agreement" { node scripts/check-versions.mjs }
$treeFile = Join-Path $env:TEMP "what-changed-cargo-tree-$PID.txt"
try {
    Invoke-Gate "resolve Windows dependency graph" {
        $treeOutput = @(& cargo tree --locked --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --edges normal 2>&1)
        if ($LASTEXITCODE -ne 0) {
            throw "cargo tree failed with exit $LASTEXITCODE"
        }
        $treeOutput | Set-Content -LiteralPath $treeFile -Encoding utf8
    }
    $env:WHAT_CHANGED_CARGO_TREE_FILE = $treeFile
    Invoke-Gate "core-only target-resolved no-network graph" { node scripts/check-no-network.mjs }
} finally {
    Remove-Item -LiteralPath $treeFile -Force -ErrorAction SilentlyContinue
    Remove-Item Env:WHAT_CHANGED_CARGO_TREE_FILE -ErrorAction SilentlyContinue
}
Invoke-Gate "Rust formatting" { cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check }
Invoke-Gate "Rust lint" { cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --no-default-features -- -D warnings }
Invoke-Gate "Rust tests" { cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features }
Invoke-Gate "frontend typecheck" { bun run check }
Invoke-Gate "generated bindings" { bun run check:bindings }
Invoke-Gate "frontend tests" { bun run test }
Invoke-Gate "frontend build" { bun run build }
Invoke-Gate "frontend bundle budget" { bun run check:bundle }
if (-not $SkipInstaller) {
    Invoke-Gate "reviewed third-party notice payload" {
        node scripts/generate-third-party-notices.mjs --prepare
    }
} else {
    [void]$gateResults.Add([pscustomobject]@{ Label = "reviewed third-party notice payload"; ExitCode = 0; Status = "SKIPPED" })
}
Invoke-Gate "documentation links" { bun run check:docs }
Invoke-Gate "brand assets" { bun run check:brand }
Invoke-Gate "launch copy claims" { bun run check:launch }
if ($IncludePreviewRunner) {
    Invoke-Gate "runner source policy, recipes and protocol" { bun run runner:test }
    Invoke-Gate "runner exact byte inventory and source parity" { bun run check:runner-bundle }
    if (-not $SkipInstaller) {
        Invoke-Gate "reviewed runtime/browser license delta" { bun run check:runner-notices }
        Invoke-Gate "matching browser sources and archive members" {
            node scripts/prepare-runner-source-materials.mjs
        }
    }
    Invoke-Gate "static landing and wiki build" { bun run site:build }
    Invoke-Gate "rendered site and download manifest" { bun run check:site }
    Invoke-Gate "static site tests" { bun run site:test }
    Write-Host '[release] Runner build: core graph does not prove child-process network confinement. Native/clean-Windows/egress/soak/license-delta/pilot evidence and G2/G3 remain required by check:owner-release.' -ForegroundColor Yellow
}

if (-not $SkipPerf) {
    Invoke-Gate "Phase 20 performance matrix (10k/100k/1M)" {
        bun run test:perf
    }
} else {
    [void]$gateResults.Add([pscustomobject]@{ Label = "Phase 20 performance matrix (10k/100k/1M)"; ExitCode = 0; Status = "SKIPPED" })
}

if (-not $SkipInstaller) {
    $installerBuildStartedAt = Get-Date
    Invoke-Gate "Tauri installer build" {
        if ($IncludePreviewRunner) {
            bun run tauri build --config src-tauri/tauri.runner.conf.json
        }
        else { bun run tauri build }
    }
    Invoke-Gate "bundled notice input parity" {
        node scripts/generate-third-party-notices.mjs --check
    }
    $rawDir = Join-Path $targetDir "release/bundle/nsis"
    $nsisScript = Join-Path $targetDir "release/nsis/x64/installer.nsi"
    Invoke-Gate "NSIS install/uninstall behavior surface" {
        node scripts/check-nsis-script.mjs $nsisScript
    }
    if ($IncludePreviewRunner) {
        Invoke-Gate "packaged runtime notice resource parity" {
            node scripts/verify-nsis-notice-copy.mjs $nsisScript release/RUNNER_THIRD_PARTY_NOTICES.txt RUNNER_THIRD_PARTY_NOTICES.txt
        }
    }
    $cargoVersion = Select-String -LiteralPath (Join-Path $root "src-tauri/Cargo.toml") -Pattern '^version\s*=\s*"([^"]+)"' |
        Select-Object -First 1
    if (-not $cargoVersion) { throw "release audit: Cargo.toml version is missing" }
    $expectedRawName = "What Changed_$($cargoVersion.Matches[0].Groups[1].Value)_x64-setup.exe"
    $raw = Get-Item -LiteralPath (Join-Path $rawDir $expectedRawName) -ErrorAction SilentlyContinue
    if (-not $raw) { throw "release audit: no NSIS installer was produced" }
    if ($raw.LastWriteTime -lt $installerBuildStartedAt) {
        throw "release audit: NSIS installer predates this build and may be stale"
    }
    $destination = Join-Path $root "dist-installer/WhatChanged-Setup-x64.exe"
    New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
    Copy-Item -LiteralPath $raw.FullName -Destination $destination -Force
    $published = Get-Item -LiteralPath $destination
    $notices = Resolve-RequiredFile (Join-Path $root "dist-installer/THIRD_PARTY_NOTICES.txt") "reviewed notices"
    $hashes = @($raw, $published, $notices) | ForEach-Object {
        $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
        $relative = Get-ManifestPath $_
        "{0}  {1}" -f $hash.Hash.ToLowerInvariant(), $relative
    }
    $checksumFile = Join-Path $root "dist-installer/SHA256SUMS.txt"
    $hashes | Set-Content -LiteralPath $checksumFile -Encoding utf8
    Write-Host "[release] raw installer: $($raw.Name) ($($raw.Length) bytes)"
    Write-Host "[release] published installer: $destination"

    Invoke-Gate "installer artifact verification" {
        # The packaged browser has its own 1536 MiB distribution budget; core
        # installers retain the existing 25 MiB limit.
        $installerBudgetMb = if ($IncludePreviewRunner) { 1536 } else { 25 }
        & powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-installer.ps1 `
            -RawInstaller $raw.FullName `
            -PublishedInstaller $published.FullName `
            -ChecksumsFile $checksumFile `
            -NoticesFile $notices.FullName `
            -NsisScript $nsisScript `
            -MaximumSizeMb $installerBudgetMb
    }

    if ($IncludePreviewRunner) {
        Invoke-Gate "combined runner installer and source packet" {
            $packetRoot = Resolve-RequiredDirectory (Join-Path $root 'preview-artifacts') 'combined runner packet'
            [void](Resolve-RequiredFile (Join-Path $packetRoot.FullName 'runtime-sources/source-materials-receipt.json') 'verified corresponding-source receipt')
            Copy-Item -LiteralPath $published.FullName -Destination (Join-Path $packetRoot.FullName 'WhatChanged-Setup-x64.exe')
            Copy-Item -LiteralPath $notices.FullName -Destination (Join-Path $packetRoot.FullName 'THIRD_PARTY_NOTICES.txt')
            Copy-Item -LiteralPath (Join-Path $root 'release/RUNNER_THIRD_PARTY_NOTICES.txt') -Destination (Join-Path $packetRoot.FullName 'RUNNER_THIRD_PARTY_NOTICES.txt')
            Copy-Item -LiteralPath (Join-Path $root 'docs/runner-source-build.md') -Destination (Join-Path $packetRoot.FullName 'RUNNER-SOURCE-BUILD.md')
            $packetChecksums = Join-Path $packetRoot.FullName 'SHA256SUMS.txt'
            Get-ChildItem -LiteralPath $packetRoot.FullName -File -Recurse | Where-Object { $_.FullName -ne $packetChecksums } | ForEach-Object {
                $relativePacketPath = $_.FullName.Substring($packetRoot.FullName.Length + 1).Replace('\', '/')
                '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $relativePacketPath
            } | Set-Content -LiteralPath $packetChecksums -Encoding utf8
            Write-Host "[release] distribute the complete installer/source packet together: $($packetRoot.FullName)"
        }
    }

    $releaseBinary = Resolve-RequiredFile (Join-Path $targetDir "release/WhatChanged.exe") "release binary"
    $distDir = Resolve-RequiredDirectory (Join-Path $root "dist") "frontend release directory"
    $env:WHAT_CHANGED_RELEASE_BINARY = $releaseBinary.FullName
    $env:WHAT_CHANGED_DIST_DIR = $distDir.FullName
    Invoke-Gate "release artifact audit" {
        cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --test release_audit -- --include-ignored
    }
    Invoke-Gate "upgrade rehearsal guard" {
        cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --test upgrade_rehearsal -- --include-ignored
    }
    Remove-Item Env:WHAT_CHANGED_RELEASE_BINARY -ErrorAction SilentlyContinue
    Remove-Item Env:WHAT_CHANGED_DIST_DIR -ErrorAction SilentlyContinue
} else {
    [void]$gateResults.Add([pscustomobject]@{ Label = "Tauri installer build"; ExitCode = 0; Status = "SKIPPED" })
    Invoke-Gate "upgrade rehearsal guard" {
        cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --test upgrade_rehearsal -- --include-ignored
    }
}

Write-Host "[release] gate summary" -ForegroundColor Cyan
foreach ($gate in $gateResults) {
    Write-Host ("[release] {0,-36} {1,-7} exit {2}" -f $gate.Label, $gate.Status, $gate.ExitCode)
}
Write-Host "[release] all executed gates completed" -ForegroundColor Green
