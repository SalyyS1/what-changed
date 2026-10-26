<#
.SYNOPSIS
  Builds an internal diagnostic executable with the frontend embedded.
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$manifestPath = Join-Path $repoRoot 'src-tauri/Cargo.toml'
$frontendIndex = Join-Path $repoRoot 'dist/index.html'

function Test-PathInTargetDirectory([string]$Path, [string]$TargetDirectory) {
    $candidate = [IO.Path]::GetFullPath($Path)
    $directoryPrefix = [IO.Path]::GetFullPath($TargetDirectory).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
    return $candidate.StartsWith($directoryPrefix, [StringComparison]::OrdinalIgnoreCase)
}

function Assert-TargetNotRunning([string]$Path, [switch]$ExactFile) {
    foreach ($process in Get-Process -Name 'what-changed' -ErrorAction SilentlyContinue) {
        try {
            if ($process.HasExited) { continue }
            $runningPath = $process.Path
            if ([string]::IsNullOrWhiteSpace($runningPath)) {
                throw "Cannot verify executable path for what-changed PID $($process.Id); stop before building."
            }
            $conflicts = if ($ExactFile) {
                [string]::Equals([IO.Path]::GetFullPath($runningPath), [IO.Path]::GetFullPath($Path), [StringComparison]::OrdinalIgnoreCase)
            } else { Test-PathInTargetDirectory $runningPath $Path }
            if ($conflicts) {
                throw "Standalone target is running (PID $($process.Id)): $runningPath. Quit it before building; no process was stopped."
            }
        } finally { $process.Dispose() }
    }
}

function Read-CargoArtifact([string]$Line, [string]$PackageId, [Collections.Generic.List[string]]$Executables) {
    $message = ConvertFrom-Json -InputObject $Line
    if ($message.reason -eq 'compiler-artifact' -and $message.package_id -eq $PackageId -and
        $message.target.name -eq 'what-changed' -and 'bin' -in $message.target.kind -and
        -not [string]::IsNullOrWhiteSpace($message.executable)) {
        [void]$Executables.Add($message.executable)
    } elseif ($message.reason -eq 'compiler-message' -and $message.message.rendered) {
        Write-Host -NoNewline $message.message.rendered
    }
}

function Resolve-StandaloneExecutable([Collections.Generic.List[string]]$Executables, [string]$TargetDirectory) {
    if ($Executables.Count -ne 1) { throw "Expected exactly one what-changed executable from Cargo; received $($Executables.Count)." }
    if (-not [IO.Path]::IsPathRooted($Executables[0])) { throw 'Cargo executable path is not absolute.' }
    $path = [IO.Path]::GetFullPath($Executables[0])
    if (-not (Test-PathInTargetDirectory $path $TargetDirectory) -or
        [IO.Path]::GetFileName($path) -ne 'what-changed.exe' -or
        -not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Cargo returned an unexpected or missing standalone executable: $path"
    }
    return $path
}

function Get-ExecutableSha256([string]$Path) {
    $stream = $null
    $hasher = $null
    try {
        $stream = [IO.File]::OpenRead($Path)
        $hasher = [Security.Cryptography.SHA256]::Create()
        return [BitConverter]::ToString($hasher.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
    } finally {
        if ($null -ne $hasher) { $hasher.Dispose() }
        if ($null -ne $stream) { $stream.Dispose() }
    }
}

Write-Host 'INTERNAL DIAGNOSTIC NOT INSTALLER/PUBLICATION'
Push-Location -LiteralPath $repoRoot
try {
    # Cargo owns target-directory resolution, including relative CARGO_TARGET_DIR values.
    $metadataOutput = & cargo metadata --manifest-path $manifestPath --format-version 1 --no-deps --offline --locked `
        --no-default-features --features tauri/custom-protocol
    if ($LASTEXITCODE -ne 0) { throw "Cargo metadata failed (exit $LASTEXITCODE)." }
    $metadata = ConvertFrom-Json -InputObject ($metadataOutput -join [Environment]::NewLine)
    if ([string]::IsNullOrWhiteSpace($metadata.target_directory)) { throw 'Cargo metadata did not return a target directory.' }
    $targetDirectory = [IO.Path]::GetFullPath($metadata.target_directory)
    $packages = @($metadata.packages | Where-Object { [IO.Path]::GetFullPath($_.manifest_path) -eq $manifestPath })
    if ($packages.Count -ne 1) { throw 'Cargo metadata did not identify exactly one application package.' }
    Assert-TargetNotRunning $targetDirectory

    & bun run build
    if ($LASTEXITCODE -ne 0) { throw "Frontend build failed (exit $LASTEXITCODE)." }
    if (-not (Test-Path -LiteralPath $frontendIndex -PathType Leaf)) {
        throw "Frontend build succeeded but the entry point is missing: $frontendIndex"
    }

    Assert-TargetNotRunning $targetDirectory
    $executables = New-Object 'Collections.Generic.List[string]'
    # Diagnostics remain live on stderr; stdout supplies the exact executable for any configured target.
    & cargo build --manifest-path $manifestPath --release --locked --no-default-features `
        --features tauri/custom-protocol --bin what-changed --message-format=json-render-diagnostics |
        ForEach-Object { Read-CargoArtifact -Line $_ -PackageId $packages[0].id -Executables $executables }
    if ($LASTEXITCODE -ne 0) { throw "Standalone Cargo build failed (exit $LASTEXITCODE)." }
    $executablePath = Resolve-StandaloneExecutable $executables $targetDirectory
    Assert-TargetNotRunning $executablePath -ExactFile

    # A successful cached Cargo build is valid even when the executable timestamp is unchanged.
    $hash = Get-ExecutableSha256 $executablePath
    Write-Output "Executable: $executablePath"
    Write-Output "SHA-256: $hash"
} finally { Pop-Location }
