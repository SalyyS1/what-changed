<#
.SYNOPSIS
  Measures the real memory footprint of a running What Changed build.

.DESCRIPTION
  Risk B2 in the plan is that the <80 MB target may be unreachable because
  WebView2 spawns its own process tree. Reporting only the Rust process would
  make the number look good and mean nothing, so this script reports three
  figures and never hides the third:

    backend RSS   - working set of WhatChanged.exe alone
    webview2 sum  - working set of every msedgewebview2.exe descendant
    total         - the number a user sees in Task Manager

  The app must already be running. Start the release build first.

.PARAMETER ProcessName
  Base name of the main binary. Defaults to WhatChanged.

.PARAMETER WaitSeconds
  Poll this long for the process to appear before giving up. Useful right after
  launching the installer build.

.PARAMETER SettleSeconds
  Wait after the process is found, so the measurement is not taken mid-startup.

.PARAMETER Append
  Append the measured row to docs/PERFORMANCE.md.

.PARAMETER Label
  Free-text label for the appended row, e.g. "P01 baseline".
#>
[CmdletBinding()]
param(
  [string]$ProcessName = 'WhatChanged',
  [int]$WaitSeconds = 0,
  [int]$SettleSeconds = 5,
  [switch]$Append,
  [string]$Label = 'baseline'
)

$ErrorActionPreference = 'Stop'

function Get-DescendantProcesses {
  param([int]$RootPid, $AllProcesses)
  $result = New-Object System.Collections.Generic.List[object]
  $frontier = New-Object System.Collections.Generic.Queue[int]
  $frontier.Enqueue($RootPid)
  $seen = New-Object System.Collections.Generic.HashSet[int]
  [void]$seen.Add($RootPid)
  while ($frontier.Count -gt 0) {
    $current = $frontier.Dequeue()
    foreach ($p in $AllProcesses) {
      if ($p.ParentProcessId -eq $current -and -not $seen.Contains([int]$p.ProcessId)) {
        [void]$seen.Add([int]$p.ProcessId)
        $result.Add($p)
        $frontier.Enqueue([int]$p.ProcessId)
      }
    }
  }
  return $result
}

# --- locate the app -----------------------------------------------------------
$deadline = (Get-Date).AddSeconds($WaitSeconds)
$main = $null
do {
  $main = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Sort-Object StartTime | Select-Object -First 1
  if ($main) { break }
  if ((Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
} while ((Get-Date) -lt $deadline)

if (-not $main) {
  Write-Host "not running: no process named '$ProcessName'." -ForegroundColor Red
  Write-Host 'Start the release build first, then re-run: bun run mem'
  exit 1
}

if ($SettleSeconds -gt 0) {
  Write-Host "found $ProcessName (pid $($main.Id)); settling ${SettleSeconds}s before measuring..."
  Start-Sleep -Seconds $SettleSeconds
  $main.Refresh()
}

# --- measure ------------------------------------------------------------------
$allCim = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, WorkingSetSize
$descendants = Get-DescendantProcesses -RootPid $main.Id -AllProcesses $allCim

$backendBytes = [int64]$main.WorkingSet64
$webviewProcs = @($descendants | Where-Object { $_.Name -like 'msedgewebview2*' })
$otherProcs = @($descendants | Where-Object { $_.Name -notlike 'msedgewebview2*' })
$webviewBytes = [int64]0
foreach ($p in $webviewProcs) { $webviewBytes += [int64]$p.WorkingSetSize }
$otherBytes = [int64]0
foreach ($p in $otherProcs) { $otherBytes += [int64]$p.WorkingSetSize }

# Working set counts every mapped page, including the Chromium binaries WebView2
# shares with every other WebView2/Edge process on the machine. Private working
# set is the part that exists *because this app is running* — the figure that
# actually changes when the app is optimized. Report both: working set is what
# Task Manager shows the user, private is what the app costs the machine.
$perf = Get-CimInstance Win32_PerfRawData_PerfProc_Process -Property IDProcess, WorkingSetPrivate
$privateOf = {
  param($processId)
  $row = $perf | Where-Object { $_.IDProcess -eq $processId } | Select-Object -First 1
  if ($row) { [int64]$row.WorkingSetPrivate } else { [int64]0 }
}
$backendPrivBytes = & $privateOf $main.Id
$webviewPrivBytes = [int64]0
foreach ($p in $webviewProcs) { $webviewPrivBytes += & $privateOf $p.ProcessId }
$otherPrivBytes = [int64]0
foreach ($p in $otherProcs) { $otherPrivBytes += & $privateOf $p.ProcessId }

$toMb = { param($b) [math]::Round($b / 1MB, 1) }
$backendMb = & $toMb $backendBytes
$webviewMb = & $toMb $webviewBytes
$privateTotalMb = & $toMb ($backendPrivBytes + $webviewPrivBytes + $otherPrivBytes)
$totalMb = & $toMb ($backendBytes + $webviewBytes + $otherBytes)

# `git rev-parse HEAD` fails on a repo with no commits, and a native command's
# stderr surfaces as a NativeCommandError under ErrorActionPreference=Stop, so
# 2>$null alone is not enough. Capture both streams and judge by exit code.
$commit = 'uncommitted'
try {
  $revOutput = (& git rev-parse --short HEAD 2>&1)
  if ($LASTEXITCODE -eq 0 -and $revOutput) { $commit = ($revOutput | Select-Object -First 1).ToString().Trim() }
} catch {
  # no repo, no git, or no commits yet — 'uncommitted' is the honest label
}
$stamp = (Get-Date).ToString('yyyy-MM-dd HH:mm')
$target = 80.0
if ($totalMb -le $target) {
  $verdict = 'within target'
  $verdictColor = 'Green'
} else {
  $verdict = "over target by $([math]::Round($totalMb - $target,1)) MB"
  $verdictColor = 'Yellow'
}

Write-Host ''
Write-Host 'What Changed - memory measurement' -ForegroundColor Cyan
Write-Host '---------------------------------'
Write-Host ("backend RSS      : {0,7} MB  ({1} pid {2})" -f $backendMb, $main.ProcessName, $main.Id)
Write-Host ("WebView2 sum     : {0,7} MB  ({1} process(es))" -f $webviewMb, $webviewProcs.Count)
if ($otherProcs.Count -gt 0) {
  Write-Host ("other children   : {0,7} MB  ({1} process(es))" -f (& $toMb $otherBytes), $otherProcs.Count)
}
Write-Host ("TOTAL            : {0,7} MB  <- the number the user sees" -f $totalMb)
Write-Host ("private WS total : {0,7} MB  <- what this app actually costs (shared Chromium pages excluded)" -f $privateTotalMb)
Write-Host ("target           : {0,7} MB  ({1})" -f $target, $verdict) -ForegroundColor $verdictColor
Write-Host ''
foreach ($p in ($webviewProcs | Sort-Object -Property WorkingSetSize -Descending)) {
  Write-Host ("  {0,-24} pid {1,-8} {2,7} MB ws  {3,7} MB private" -f $p.Name, $p.ProcessId, (& $toMb $p.WorkingSetSize), (& $toMb (& $privateOf $p.ProcessId)))
}
Write-Host ''

$row = "| {0} | {1} | {2} | {3} MB | {4} MB | {5} MB | {6} MB | {7} |" -f $stamp, $commit, $Label, $backendMb, $webviewMb, $totalMb, $privateTotalMb, $verdict
Write-Host 'markdown row:'
Write-Host $row

if ($Append) {
  $repoRoot = Split-Path -Parent $PSScriptRoot
  $doc = Join-Path $repoRoot 'docs\PERFORMANCE.md'
  if (-not (Test-Path $doc)) {
    Write-Host "cannot append: $doc does not exist" -ForegroundColor Red
    exit 1
  }
  Add-Content -Path $doc -Value $row -Encoding UTF8
  Write-Host "appended to $doc" -ForegroundColor Green
}

exit 0
