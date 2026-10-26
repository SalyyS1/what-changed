<#
.SYNOPSIS
  Verifies the native build toolchain and records the chosen MSVC install.

.DESCRIPTION
  Three things can silently break a Rust MSVC build on this class of machine:

    1. More than one Visual Studio / Build Tools install providing VC.Tools.x86.x64,
       so "the linker" is ambiguous.
    2. A POSIX `link` (Git Bash / MSYS2 coreutils) winning the PATH lookup ahead of
       MSVC's `link.exe`. The failure looks like a linker bug, not a PATH bug.
    3. A missing Windows SDK, which surfaces as LNK1181 on kernel32.lib.

  This script reports all three and exits non-zero when the build cannot be
  trusted, so it is usable as a gate.

.PARAMETER Json
  Emit a machine-readable summary instead of the human report.
#>
[CmdletBinding()]
param(
  [switch]$Json
)

$ErrorActionPreference = 'Stop'
$problems = New-Object System.Collections.Generic.List[string]
$warnings = New-Object System.Collections.Generic.List[string]

function Test-CommandVersion {
  param([string]$Name, [string]$Exe, [string[]]$VersionArgs)
  $resolved = Get-Command $Exe -ErrorAction SilentlyContinue
  if (-not $resolved) { return [pscustomobject]@{ Name = $Name; Found = $false; Version = $null; Path = $null } }
  $raw = & $Exe @VersionArgs 2>&1 | Select-Object -First 1
  return [pscustomobject]@{ Name = $Name; Found = $true; Version = "$raw".Trim(); Path = $resolved.Source }
}

# --- Rust ---------------------------------------------------------------------
$rustc = Test-CommandVersion -Name 'rustc' -Exe 'rustc' -VersionArgs @('--version')
$cargo = Test-CommandVersion -Name 'cargo' -Exe 'cargo' -VersionArgs @('--version')
$rustHost = $null
if ($rustc.Found) {
  $hostMatch = & rustc -vV 2>&1 | Select-String -Pattern '^host:' | Select-Object -First 1
  if ($hostMatch) { $rustHost = "$hostMatch".Replace('host:', '').Trim() }
  if ($rustHost -ne 'x86_64-pc-windows-msvc') {
    $problems.Add("rustc host is '$rustHost', expected 'x86_64-pc-windows-msvc'")
  }
} else {
  $problems.Add('rustc not found on PATH (see docs/DECISIONS.md D-01)')
}

# --- MSVC installs ------------------------------------------------------------
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$msvcInstalls = @()
if (Test-Path $vswhere) {
  $paths = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
  foreach ($p in $paths) {
    if ([string]::IsNullOrWhiteSpace($p)) { continue }
    $toolsRoot = Join-Path $p 'VC\Tools\MSVC'
    $toolsets = @()
    if (Test-Path $toolsRoot) {
      $toolsets = Get-ChildItem $toolsRoot -Directory | Select-Object -ExpandProperty Name | Sort-Object -Descending
    }
    $msvcInstalls += [pscustomobject]@{
      InstallPath = $p
      Toolsets    = $toolsets
      Newest      = ($toolsets | Select-Object -First 1)
      VcVars      = Join-Path $p 'VC\Auxiliary\Build\vcvars64.bat'
    }
  }
} else {
  $problems.Add("vswhere.exe not found at $vswhere")
}

if ($msvcInstalls.Count -eq 0) {
  $problems.Add('no Visual Studio install provides Microsoft.VisualStudio.Component.VC.Tools.x86.x64')
}

# Newest toolset across every install wins; ties are impossible because the
# version string is unique per install directory.
$chosen = $msvcInstalls | Sort-Object -Property Newest -Descending | Select-Object -First 1
if ($msvcInstalls.Count -gt 1) {
  $warnings.Add("$($msvcInstalls.Count) MSVC installs present; using toolset $($chosen.Newest) from $($chosen.InstallPath)")
}
if ($chosen -and -not (Test-Path $chosen.VcVars)) {
  $problems.Add("chosen install has no vcvars64.bat at $($chosen.VcVars)")
}

# --- link.exe shadow check ----------------------------------------------------
# `Get-Command link` walks PATH exactly like a build tool would.
$linkCandidates = @(Get-Command link -All -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source)
$firstLink = $linkCandidates | Select-Object -First 1
$msvcLink = $null
if ($chosen -and $chosen.Newest) {
  $msvcLink = Join-Path $chosen.InstallPath "VC\Tools\MSVC\$($chosen.Newest)\bin\Hostx64\x64\link.exe"
  if (-not (Test-Path $msvcLink)) {
    $problems.Add("MSVC link.exe missing at $msvcLink")
    $msvcLink = $null
  }
}
$linkShadowed = $false
if ($firstLink -and $firstLink -notlike '*\VC\Tools\MSVC\*') {
  $linkShadowed = $true
  # Quotes are doubled, not backslash-escaped: PowerShell has no backslash escape
  # inside a double-quoted string, so \" would end the string and break parsing.
  $warnings.Add("PATH resolves 'link' to '$firstLink' (not MSVC). rustc locates link.exe through its own MSVC detection, so this is usually harmless; if a build fails with LNK1181 or ""linker not found"", re-run it through scripts/dev-shell.cmd.")
}

# --- Windows SDK --------------------------------------------------------------
$sdkRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\Lib'
$sdkVersions = @()
if (Test-Path $sdkRoot) {
  # @(...) is required: with a single SDK installed the pipeline returns a bare
  # string, and $sdkVersions[0] would then index its first character ("1").
  $sdkVersions = @(Get-ChildItem $sdkRoot -Directory | Select-Object -ExpandProperty Name | Sort-Object -Descending)
}
if ($sdkVersions.Count -eq 0) { $problems.Add("no Windows SDK libraries under $sdkRoot") }

# --- Frontend toolchain -------------------------------------------------------
$bun = Test-CommandVersion -Name 'bun' -Exe 'bun' -VersionArgs @('--version')
if (-not $bun.Found) { $problems.Add('bun not found on PATH') }
$git = Test-CommandVersion -Name 'git' -Exe 'git' -VersionArgs @('--version')

# --- WebView2 -----------------------------------------------------------------
$wv2Key = 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$wv2 = (Get-ItemProperty $wv2Key -ErrorAction SilentlyContinue).pv
if (-not $wv2) { $problems.Add('WebView2 Evergreen runtime not detected') }

# --- Report -------------------------------------------------------------------
# Values are precomputed: Windows PowerShell 5.1 cannot use `if` as an
# expression inside a hashtable literal, and `powershell.exe` is what
# `bun run toolchain` invokes.
$installLines = @()
foreach ($i in $msvcInstalls) { $installLines += "$($i.InstallPath) [$($i.Toolsets -join ', ')]" }
$chosenPath = $null
$chosenToolset = $null
if ($chosen) {
  $chosenPath = $chosen.InstallPath
  $chosenToolset = $chosen.Newest
}
$newestSdk = $null
if ($sdkVersions.Count -gt 0) { $newestSdk = $sdkVersions[0] }

$summary = [pscustomobject]@{
  RustcVersion   = $rustc.Version
  RustcHost      = $rustHost
  CargoVersion   = $cargo.Version
  MsvcInstalls   = $installLines
  MsvcChosen     = $chosenPath
  MsvcToolset    = $chosenToolset
  MsvcLink       = $msvcLink
  PathLink       = $firstLink
  LinkShadowed   = $linkShadowed
  WindowsSdk     = $newestSdk
  BunVersion     = $bun.Version
  GitVersion     = $git.Version
  WebView2       = $wv2
  Warnings       = $warnings
  Problems       = $problems
  Ok             = ($problems.Count -eq 0)
}

if ($Json) {
  $summary | ConvertTo-Json -Depth 4
} else {
  Write-Host ''
  Write-Host 'What Changed - toolchain verification' -ForegroundColor Cyan
  Write-Host '-------------------------------------'
  Write-Host ("rustc          : {0}  (host {1})" -f $summary.RustcVersion, $summary.RustcHost)
  Write-Host ("cargo          : {0}" -f $summary.CargoVersion)
  Write-Host ("MSVC chosen    : {0}  toolset {1}" -f $summary.MsvcChosen, $summary.MsvcToolset)
  foreach ($i in $summary.MsvcInstalls) { Write-Host ("  candidate    : {0}" -f $i) }
  Write-Host ("MSVC link.exe  : {0}" -f $summary.MsvcLink)
  Write-Host ("PATH 'link'    : {0}" -f $summary.PathLink)
  Write-Host ("Windows SDK    : {0}" -f $summary.WindowsSdk)
  Write-Host ("bun            : {0}" -f $summary.BunVersion)
  Write-Host ("git            : {0}" -f $summary.GitVersion)
  Write-Host ("WebView2       : {0}" -f $summary.WebView2)
  foreach ($w in $warnings) { Write-Host ("WARN  {0}" -f $w) -ForegroundColor Yellow }
  foreach ($p in $problems) { Write-Host ("FAIL  {0}" -f $p) -ForegroundColor Red }
  Write-Host ''
  if ($summary.Ok) { Write-Host 'toolchain OK' -ForegroundColor Green }
}

if (-not $summary.Ok) { exit 1 }
exit 0
