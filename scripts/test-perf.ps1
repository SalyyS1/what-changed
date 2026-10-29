<#
.SYNOPSIS
  Runs the opt-in Phase 20 performance test matrix.

.DESCRIPTION
  Invoked by `bun run test:perf` and by the default release gate. Checks the
  SQLite query plans on the canonical 10k corpus, including actual page and
  complete checkpoint comparison reads, then runs the query budget and database
  size/vacuum tests separately at 10k, 100k, and 1M events. A separate 10k
  size gate uses an opt-in, checked-in collector-payload sensitivity profile;
  it is not a production-history representativeness claim. It is intentionally
  not enabled at 100k or 1M pending review of the 10k result. A separate 10k
  gate exercises production retention and incremental reclaim without a full
  VACUUM. A failing test or Cargo invocation terminates the matrix nonzero.
  Most plan-shape SQL remains copied from production, so the canonical fixture
  and captured page/checkpoint reads do not yet prove every §61 service path.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-perf.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$rowsWasSet = Test-Path Env:WHAT_CHANGED_PERF_ROWS
$previousRows = if ($rowsWasSet) { $env:WHAT_CHANGED_PERF_ROWS } else { $null }
$payloadWasSet = Test-Path Env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY
$previousPayload = if ($payloadWasSet) { $env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY } else { $null }
$jobsWasSet = Test-Path Env:CARGO_BUILD_JOBS
$previousJobs = if ($jobsWasSet) { $env:CARGO_BUILD_JOBS } else { $null }
$tempWasSet = Test-Path Env:TEMP
$previousTemp = if ($tempWasSet) { $env:TEMP } else { $null }
$tmpWasSet = Test-Path Env:TMP
$previousTmp = if ($tmpWasSet) { $env:TMP } else { $null }

function Invoke-PerfTest([string]$Label, [string[]]$CargoArguments) {
    Write-Host "[perf] $Label" -ForegroundColor Cyan
    & cargo @CargoArguments
    $exitCode = [int]$LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "Phase 20 performance gate failed: $Label (cargo exit $exitCode)"
    }
}

Push-Location $root
try {
    # Keep the full-size corpus and helper temporary files on the workspace
    # volume. Resolve and check the exact parent before handing it to Cargo;
    # a junction at .perf-temp must not redirect the write to another drive.
    $workspace = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $root).ProviderPath)
    $expectedTemp = [IO.Path]::GetFullPath((Join-Path $workspace '.perf-temp'))
    if (-not (Test-Path -LiteralPath $expectedTemp)) {
        New-Item -ItemType Directory -Path $expectedTemp -ErrorAction Stop | Out-Null
    }
    $tempItem = Get-Item -LiteralPath $expectedTemp -ErrorAction Stop
    $resolvedTemp = [IO.Path]::GetFullPath((Resolve-Path -LiteralPath $expectedTemp).ProviderPath)
    if (-not $tempItem.PSIsContainer -or
        (($tempItem.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) -or
        -not [StringComparer]::OrdinalIgnoreCase.Equals($resolvedTemp, $expectedTemp) -or
        -not [StringComparer]::OrdinalIgnoreCase.Equals(
            [IO.Path]::GetPathRoot($resolvedTemp), [IO.Path]::GetPathRoot($workspace))) {
        throw 'Performance temp parent must be a real directory on the workspace volume'
    }
    $env:TEMP = $resolvedTemp
    $env:TMP = $resolvedTemp
    # Default to the build parallelism proven safe on the release host, while
    # respecting an explicit caller override.
    if (-not $jobsWasSet) { $env:CARGO_BUILD_JOBS = '1' }
    Remove-Item Env:WHAT_CHANGED_PERF_ROWS -ErrorAction SilentlyContinue
    Remove-Item Env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY -ErrorAction SilentlyContinue
    Invoke-PerfTest 'Canonical 10k SQLite query-plan shapes' @(
        'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
        '--features', 'devtools', '--test', 'perf_query_plans',
        '--', '--include-ignored', '--nocapture'
    )
    Invoke-PerfTest 'Canonical 10k actual first and tenth page plans' @(
        'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
        '--features', 'devtools', '--test', 'page_plan_capture_profile',
        'canonical_10k_actual_first_and_tenth_page_plans',
        '--', '--exact', '--ignored', '--nocapture'
    )
    Invoke-PerfTest 'Canonical 10k actual complete checkpoint comparison plans' @(
        'test', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
        '--features', 'devtools', '--test', 'checkpoint_window_plan_profile',
        'canonical_10k_actual_complete_checkpoint_compare_plans',
        '--', '--exact', '--ignored', '--nocapture'
    )

    foreach ($rows in @(10000, 100000, 1000000)) {
        $env:WHAT_CHANGED_PERF_ROWS = [string]$rows
        Invoke-PerfTest "$rows events: eight query shapes and database size/vacuum" @(
            'test', '--release', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
            '--features', 'devtools', '--test', 'perf_queries', '--test', 'perf_db_size',
            '--', '--include-ignored', '--nocapture'
        )
        if ($rows -eq 10000) {
            $env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY = '1'
            Invoke-PerfTest '10000 events: collector-payload sensitivity database size/vacuum' @(
                'test', '--release', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
                '--features', 'devtools', '--test', 'perf_db_size',
                '--', '--include-ignored', '--nocapture'
            )
            Remove-Item Env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY -ErrorAction SilentlyContinue
        }
    }
    Remove-Item Env:WHAT_CHANGED_PERF_ROWS -ErrorAction SilentlyContinue
    Invoke-PerfTest 'Canonical 10k routine retention and incremental reclaim' @(
        'test', '--release', '--locked', '--manifest-path', 'src-tauri/Cargo.toml',
        '--features', 'devtools', '--test', 'maintenance_reclaim',
        '--', '--include-ignored', '--nocapture'
    )
    Write-Host '[perf] all executed matrix gates passed' -ForegroundColor Green
} finally {
    if ($rowsWasSet) {
        $env:WHAT_CHANGED_PERF_ROWS = $previousRows
    } else {
        Remove-Item Env:WHAT_CHANGED_PERF_ROWS -ErrorAction SilentlyContinue
    }
    if ($payloadWasSet) {
        $env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY = $previousPayload
    } else {
        Remove-Item Env:WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY -ErrorAction SilentlyContinue
    }
    if ($jobsWasSet) {
        $env:CARGO_BUILD_JOBS = $previousJobs
    } else {
        Remove-Item Env:CARGO_BUILD_JOBS -ErrorAction SilentlyContinue
    }
    if ($tempWasSet) {
        $env:TEMP = $previousTemp
    } else {
        Remove-Item Env:TEMP -ErrorAction SilentlyContinue
    }
    if ($tmpWasSet) {
        $env:TMP = $previousTmp
    } else {
        Remove-Item Env:TMP -ErrorAction SilentlyContinue
    }
    Pop-Location
}
