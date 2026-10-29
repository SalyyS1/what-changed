[CmdletBinding()]
param(
  [string]$Source,
  [string]$Output,
  [switch]$UpdateTauriIcons
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($Source)) {
  $Source = Join-Path $PSScriptRoot '..\assets\brand'
}
if ([string]::IsNullOrWhiteSpace($Output)) {
  $Output = Join-Path $PSScriptRoot '..\.qa\brand-export'
}

$sourcePath = [IO.Path]::GetFullPath($Source)
$outputPath = [IO.Path]::GetFullPath($Output)
$rasterPath = Join-Path $outputPath 'raster'
$required = @(
  'icon-source.svg',
  'logo-mark.svg',
  'logo-mark-monochrome.svg',
  'logo-wordmark.svg',
  'logo-lockup-horizontal.svg',
  'logo-lockup-stacked.svg',
  'tray-light.svg',
  'tray-dark.svg',
  'tray-high-contrast.svg',
  'social-preview-template.svg',
  'installer-header.svg',
  'installer-sidebar.svg',
  'article-cover-template.svg',
  'brand-guidelines.md'
)

if (-not (Test-Path -LiteralPath $sourcePath -PathType Container)) {
  throw "Brand source directory is missing: $sourcePath"
}
foreach ($name in $required) {
  $path = Join-Path $sourcePath $name
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Required brand source is missing: $name"
  }
}
$designSheet = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\assets\designs\delta-bridge-approval-sheet.svg'))
if (-not (Test-Path -LiteralPath $designSheet -PathType Leaf)) {
  throw "Brand design review sheet is missing: $designSheet"
}

$magick = Get-Command magick -ErrorAction SilentlyContinue
if (-not $magick) {
  throw 'ImageMagick (magick.exe) is required to produce deterministic raster assets.'
}

New-Item -ItemType Directory -Force -Path $outputPath, $rasterPath | Out-Null

function Invoke-Magick([string[]]$Arguments) {
  & $magick.Source @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "ImageMagick failed with exit code ${LASTEXITCODE}: $($Arguments -join ' ')"
  }
}

function Get-Sha256([string]$Path) {
  $hasher = [Security.Cryptography.SHA256]::Create()
  try {
    $stream = [IO.File]::OpenRead($Path)
    try {
      return ([BitConverter]::ToString($hasher.ComputeHash($stream)) -replace '-', '').ToLowerInvariant()
    } finally {
      $stream.Dispose()
    }
  } finally {
    $hasher.Dispose()
  }
}

$sourceSvg = Join-Path $sourcePath 'icon-source.svg'
$sizes = @(16, 20, 24, 30, 32, 44, 48, 50, 64, 71, 89, 107, 128, 142, 150, 256, 284, 310, 512, 1024)
$rasterFiles = [ordered]@{}
$master = Join-Path $rasterPath 'icon-1024.png'
Invoke-Magick @(
  '-background', 'none',
  '-density', '144',
  $sourceSvg,
  '-resize', '1024x1024',
  '-depth', '8',
  '-strip',
  $master
)
foreach ($size in $sizes) {
  $name = "icon-$size.png"
  $destination = Join-Path $rasterPath $name
  if ($size -ne 1024) {
    Invoke-Magick @($master, '-resize', "${size}x${size}", '-depth', '8', '-strip', $destination)
  }
  $rasterFiles[$name] = Get-Sha256 $destination
}

$trayVariants = @('light', 'dark', 'high-contrast')
$traySizes = @(16, 20, 24, 32)
foreach ($variant in $trayVariants) {
  $traySource = Join-Path $sourcePath "tray-$variant.svg"
  foreach ($size in $traySizes) {
    $name = "tray-$variant-$size.png"
    $destination = Join-Path $rasterPath $name
    Invoke-Magick @(
      '-background', 'none', '-density', '384', $traySource,
      '-resize', "${size}x${size}", '-depth', '8', '-strip', $destination
    )
    $rasterFiles[$name] = Get-Sha256 $destination
  }
}

foreach ($art in @(
  @{ Source = 'social-preview-template.svg'; Name = 'social-preview-1280x640.png'; Size = '1280x640' },
  @{ Source = 'article-cover-template.svg'; Name = 'article-cover-1280x720.png'; Size = '1280x720' }
)) {
  $destination = Join-Path $rasterPath $art.Name
  Invoke-Magick @(
    '-background', 'none', '-density', '96',
    (Join-Path $sourcePath $art.Source), '-resize', "$($art.Size)!",
    '-depth', '8', '-strip', $destination
  )
  $rasterFiles[$art.Name] = Get-Sha256 $destination
}

foreach ($art in @(
  @{ Source = 'installer-header.svg'; Name = 'installer-header.bmp'; Size = '150x57' },
  @{ Source = 'installer-sidebar.svg'; Name = 'installer-sidebar.bmp'; Size = '164x314' }
)) {
  $destination = Join-Path $outputPath $art.Name
  Invoke-Magick @(
    '-background', 'white', '-density', '384',
    (Join-Path $sourcePath $art.Source), '-resize', "$($art.Size)!",
    '-depth', '8', '-type', 'TrueColor', "BMP3:$destination"
  )
  $rasterFiles[$art.Name] = Get-Sha256 $destination
}

$ico = Join-Path $outputPath 'WhatChanged.ico'
$icoInputs = @(
  (Join-Path $rasterPath 'icon-16.png'),
  (Join-Path $rasterPath 'icon-20.png'),
  (Join-Path $rasterPath 'icon-24.png'),
  (Join-Path $rasterPath 'icon-32.png'),
  (Join-Path $rasterPath 'icon-48.png'),
  (Join-Path $rasterPath 'icon-64.png'),
  (Join-Path $rasterPath 'icon-128.png'),
  (Join-Path $rasterPath 'icon-256.png'),
  $ico
)
Invoke-Magick $icoInputs
$rasterFiles['WhatChanged.ico'] = Get-Sha256 $ico

foreach ($name in $required) {
  Copy-Item -LiteralPath (Join-Path $sourcePath $name) -Destination (Join-Path $outputPath $name) -Force
}

if ($UpdateTauriIcons) {
  $tauriIconPath = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\src-tauri\icons'))
  $tauriMap = [ordered]@{
    'icon-32.png' = '32x32.png'
    'icon-64.png' = '64x64.png'
    'icon-128.png' = '128x128.png'
    'icon-256.png' = '128x128@2x.png'
    'icon-512.png' = 'icon.png'
    'icon-107.png' = 'Square107x107Logo.png'
    'icon-142.png' = 'Square142x142Logo.png'
    'icon-150.png' = 'Square150x150Logo.png'
    'icon-284.png' = 'Square284x284Logo.png'
    'icon-30.png' = 'Square30x30Logo.png'
    'icon-310.png' = 'Square310x310Logo.png'
    'icon-44.png' = 'Square44x44Logo.png'
    'icon-71.png' = 'Square71x71Logo.png'
    'icon-89.png' = 'Square89x89Logo.png'
    'icon-50.png' = 'StoreLogo.png'
    'WhatChanged.ico' = 'icon.ico'
    'tray-light-32.png' = 'tray-light.png'
    'tray-dark-32.png' = 'tray-dark.png'
    'tray-high-contrast-32.png' = 'tray-high-contrast.png'
  }
  foreach ($entry in $tauriMap.GetEnumerator()) {
    $generated = if ($entry.Key -eq 'WhatChanged.ico') {
      $ico
    } else {
      Join-Path $rasterPath $entry.Key
    }
    Copy-Item -LiteralPath $generated -Destination (Join-Path $tauriIconPath $entry.Value) -Force
  }
  foreach ($name in @('installer-header.bmp', 'installer-sidebar.bmp')) {
    Copy-Item -LiteralPath (Join-Path $outputPath $name) -Destination (Join-Path $sourcePath $name) -Force
  }
  $launchSocial = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\assets\launch\social\social-preview.svg'))
  Copy-Item -LiteralPath (Join-Path $sourcePath 'social-preview-template.svg') -Destination $launchSocial -Force
  $trackedFiles = [ordered]@{
    'assets/brand/installer-header.bmp' = Get-Sha256 (Join-Path $sourcePath 'installer-header.bmp')
    'assets/brand/installer-sidebar.bmp' = Get-Sha256 (Join-Path $sourcePath 'installer-sidebar.bmp')
    'assets/launch/social/social-preview.svg' = Get-Sha256 $launchSocial
    'assets/designs/delta-bridge-approval-sheet.svg' = Get-Sha256 $designSheet
  }
  foreach ($name in $required) {
    $trackedFiles["assets/brand/$name"] = Get-Sha256 (Join-Path $sourcePath $name)
  }
  foreach ($entry in $tauriMap.GetEnumerator()) {
    $trackedFiles["src-tauri/icons/$($entry.Value)"] = Get-Sha256 (Join-Path $tauriIconPath $entry.Value)
  }
  [ordered]@{
    schema = 1
    source = 'assets/brand/icon-source.svg'
    source_sha256 = Get-Sha256 $sourceSvg
    rasterizer = 'ImageMagick'
    rasterizer_version = ((& $magick.Source -version | Select-Object -First 1).Trim())
    files = $trackedFiles
  } | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $sourcePath 'generated-assets-manifest.json') -Encoding utf8
  Write-Output "Updated Tauri icons under $tauriIconPath"
}

$manifest = [ordered]@{
  schema = 2
  source = 'assets/brand/icon-source.svg'
  source_sha256 = Get-Sha256 $sourceSvg
  rasterizer = 'ImageMagick'
  rasterizer_version = ((& $magick.Source -version | Select-Object -First 1).Trim())
  sizes = $sizes
  tray_sizes = $traySizes
  files = $rasterFiles
}
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $outputPath 'manifest.json') -Encoding utf8
Write-Output "Brand assets exported deterministically to $outputPath"
