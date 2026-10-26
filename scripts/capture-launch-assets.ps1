[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$Commit,
  [string]$Output = "$PSScriptRoot\..\assets\launch"
)

$ErrorActionPreference = 'Stop'
if ($Commit -notmatch '^[0-9a-fA-F]{7,40}$') { throw 'Commit must be a git SHA.' }
$outputPath = [IO.Path]::GetFullPath($Output)
$screenshots = Join-Path $outputPath 'screenshots'
$demo = Join-Path $outputPath 'demo'
$social = Join-Path $outputPath 'social'
foreach ($path in @($screenshots, $demo, $social)) { New-Item -ItemType Directory -Force -Path $path | Out-Null }
$manifestPath = Join-Path $outputPath 'capture-manifest.json'
if (Test-Path -LiteralPath $manifestPath) {
  $existing = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
  if ($existing.commit -ne $Commit) { throw "Refusing to overwrite launch captures from commit $($existing.commit)." }
}
$shotNames = @('01-home','02-timeline','03-details','04-investigation','05-checkpoint','06-map-2d','07-privacy','08-diagnostics','09-tray')
$shots = foreach ($name in $shotNames) {
  [ordered]@{ name = $name; status = 'capture-required-on-RC'; synthetic_data = $true; viewport = '1280x820'; dpi = 100; theme = 'dark' }
}
[ordered]@{
  schema = 1
  version = $Version
  commit = $Commit
  captured_at = (Get-Date).ToUniversalTime().ToString('o')
  source = 'tagged release candidate with deterministic synthetic fixture'
  screenshots = $shots
  demo_seconds = 75
  demo_transcript = 'demo/demo-transcript.md'
  social_source = 'social/social-preview.svg'
} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding utf8
Write-Output "Launch capture manifest written: $manifestPath"
