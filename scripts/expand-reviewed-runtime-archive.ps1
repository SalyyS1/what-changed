param(
    [Parameter(Mandatory = $true)][string]$Archive,
    [Parameter(Mandatory = $true)][string]$Destination,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-f0-9]{64}$')][string]$Sha256,
    [Parameter(Mandatory = $true)][string]$ExpectedRoot
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression

function Assert-RegularAncestors([string]$Path) {
    $directory = [System.IO.Path]::GetDirectoryName($Path)
    while ($directory) {
        $item = Get-Item -LiteralPath $directory -Force
        if (-not $item.PSIsContainer -or ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
            throw "Archive ancestor is not a regular directory: $directory"
        }
        $directory = [System.IO.Path]::GetDirectoryName($directory)
    }
}

function Assert-SafeSegment([string]$Segment) {
    if (-not $Segment -or $Segment.Length -gt 255 -or $Segment -in @('.', '..') -or
        $Segment -match '[<>:"/\\|?*\x00-\x1f\x7f]' -or $Segment -match '[ .]$' -or
        $Segment -match '^(?i:CON|PRN|AUX|NUL|CONIN\$|CONOUT\$|COM[0-9\u00b9\u00b2\u00b3]|LPT[0-9\u00b9\u00b2\u00b3])(?:[ .]|$)') {
        throw "Unsafe Windows archive segment: $Segment"
    }
}

Assert-SafeSegment $ExpectedRoot
$archivePath = [System.IO.Path]::GetFullPath($Archive)
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
Assert-RegularAncestors $archivePath
Assert-RegularAncestors $destinationPath
$archiveItem = Get-Item -LiteralPath $archivePath -Force
if ($archiveItem.PSIsContainer -or ($archiveItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
    throw 'Archive must be a regular file'
}
if (Get-Item -LiteralPath $destinationPath -Force -ErrorAction SilentlyContinue) {
    throw 'Archive destination already exists'
}

$stream = $null
$zip = $null
try {
    # Keep the verified archive open without write sharing throughout extraction.
    $stream = [System.IO.File]::Open($archivePath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::Read)
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try { $actualHash = ([System.BitConverter]::ToString($hasher.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $hasher.Dispose() }
    if ($actualHash -cne $Sha256) { throw 'Reviewed archive SHA-256 differs' }
    $stream.Position = 0
    $zip = [System.IO.Compression.ZipArchive]::new($stream, [System.IO.Compression.ZipArchiveMode]::Read, $true)
    if ($zip.Entries.Count -lt 1 -or $zip.Entries.Count -gt 10000) { throw 'Archive entry count exceeds the reviewed limit' }
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $paths = [System.Collections.Generic.Dictionary[string, string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $kinds = [System.Collections.Generic.Dictionary[string, string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    $records = [System.Collections.Generic.List[object]]::new()
    [long]$totalBytes = 0
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName
        if (-not $name -or $name.Contains('\') -or $name.StartsWith('/') -or [System.IO.Path]::IsPathRooted($name)) {
            throw "Unsafe archive path: $name"
        }
        $isDirectory = $name.EndsWith('/')
        $pathName = if ($isDirectory) { $name.Substring(0, $name.Length - 1) } else { $name }
        $segments = $pathName.Split([char]'/')
        foreach ($segment in $segments) { Assert-SafeSegment $segment }
        if ($segments[0] -cne $ExpectedRoot -or ($segments.Length -eq 1 -and -not $isDirectory)) {
            throw 'Archive must contain exactly the expected directory root'
        }
        $unixType = ($entry.ExternalAttributes -shr 16) -band 0xf000
        $dosAttributes = $entry.ExternalAttributes -band 0xffff
        if (($dosAttributes -band 0x448) -or $unixType -notin @(0, 0x4000, 0x8000) -or
            ($unixType -eq 0x4000 -and -not $isDirectory) -or ($unixType -eq 0x8000 -and $isDirectory) -or
            (($dosAttributes -band 0x10) -and -not $isDirectory)) { throw "Nonregular archive entry: $name" }
        if (-not $seen.Add($pathName)) { throw "Duplicate archive entry: $name" }
        for ($index = 0; $index -lt $segments.Length; $index += 1) {
            $prefix = ($segments[0..$index] -join '/')
            $kind = if ($index -lt $segments.Length - 1 -or $isDirectory) { 'directory' } else { 'file' }
            if ($paths.ContainsKey($prefix)) {
                if ($paths[$prefix] -cne $prefix -or $kinds[$prefix] -cne $kind) { throw "Archive path collision: $name" }
            } else { $paths.Add($prefix, $prefix); $kinds.Add($prefix, $kind) }
        }
        if ($entry.Length -lt 0 -or ($isDirectory -and $entry.Length -ne 0)) { throw "Invalid archive entry length: $name" }
        if ($entry.Length -gt 1610612736 - $totalBytes) { throw 'Archive extraction exceeds 1536 MiB' }
        $totalBytes += $entry.Length
        $target = [System.IO.Path]::GetFullPath([System.IO.Path]::Combine($destinationPath, ($pathName.Replace('/', '\'))))
        if (-not $target.StartsWith($destinationPath.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Archive entry escapes destination: $name"
        }
        $records.Add(@{ Entry = $entry; Target = $target; Directory = $isDirectory })
    }
    # No output directory exists until the complete central directory is accepted.
    New-Item -ItemType Directory -Path $destinationPath -ErrorAction Stop | Out-Null
    $buffer = [byte[]]::new(65536)
    foreach ($record in $records) {
        if ($record.Directory) { [System.IO.Directory]::CreateDirectory($record.Target) | Out-Null; continue }
        [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($record.Target)) | Out-Null
        Assert-RegularAncestors $record.Target
        $inputStream = $null
        $outputStream = $null
        try {
            $inputStream = $record.Entry.Open()
            $outputStream = [System.IO.File]::Open($record.Target, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
            [long]$written = 0
            while (($count = $inputStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                if ($count -gt $record.Entry.Length - $written) { throw 'Archive stream exceeds declared entry length' }
                $outputStream.Write($buffer, 0, $count)
                $written += $count
            }
            if ($written -ne $record.Entry.Length) { throw 'Archive stream differs from declared entry length' }
        } finally {
            if ($outputStream) { $outputStream.Dispose() }
            if ($inputStream) { $inputStream.Dispose() }
        }
    }
    @{ destination = $destinationPath; expectedRoot = $ExpectedRoot; entries = $records.Count; bytes = $totalBytes; sha256 = $actualHash } | ConvertTo-Json -Compress
} finally {
    if ($zip) { $zip.Dispose() }
    if ($stream) { $stream.Dispose() }
}
