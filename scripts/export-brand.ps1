[CmdletBinding()]
param(
  [string]$Source,
  [string]$Output
)

# Compatibility entry point retained for older local notes. The canonical
# exporter is export-brand-assets.ps1 so there is one raster/ICO implementation.
if ([string]::IsNullOrWhiteSpace($Source)) {
  $Source = Join-Path $PSScriptRoot '..\assets\brand\icon-source.svg'
}
if ([string]::IsNullOrWhiteSpace($Output)) {
  $Output = Join-Path $PSScriptRoot '..\.qa\brand-export'
}

$sourceFile = [IO.Path]::GetFullPath($Source)
$sourceDirectory = Split-Path -Parent $sourceFile
& (Join-Path $PSScriptRoot 'export-brand-assets.ps1') -Source $sourceDirectory -Output $Output
exit $LASTEXITCODE
