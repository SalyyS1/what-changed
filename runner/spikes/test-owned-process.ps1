[CmdletBinding()]
param(
    [string]$NodeExecutable,
    [ValidateSet('', 'ParentWait')][string]$WorkerMode = '',
    [string]$CaseDirectory
)

# Feasibility only. Generated fixtures stay in the synthetic QA root.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$invocationId = [guid]::NewGuid().ToString('D')
$evidence = [ordered]@{
    schema = 'ownvero-owned-process-v1'; invocationId = $invocationId
    utc = [DateTime]::UtcNow.ToString('o'); status = 'started'; exitCode = $null
    results = @(); freshMachineProof = $false; passed = $false
}
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$testRoot = Join-Path $repository '.qa/ownvero-runtime-spike/owned-process-test'
$helper = Join-Path $PSScriptRoot 'windows-owned-process.cs'

function Assert-Spike([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Assert-QaPath([string]$Path) {
    $resolved = [IO.Path]::GetFullPath($Path)
    Assert-Spike ($resolved -eq $testRoot -or $resolved.StartsWith($testRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) 'Write path is outside the synthetic QA root.'
    # Inspect every existing component, including ancestors outside the root, before any write.
    $cursor = $resolved
    while ($cursor) {
        try {
            $attributes = [IO.File]::GetAttributes($cursor)
            Assert-Spike (($attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) 'QA path contains a reparse point.'
        }
        catch [IO.FileNotFoundException] { }
        catch [IO.DirectoryNotFoundException] { }
        $parent = [IO.Directory]::GetParent($cursor)
        $cursor = if ($parent) { $parent.FullName } else { $null }
    }
    return $resolved
}

function New-QaDirectory([string]$Path) {
    $resolved = Assert-QaPath $Path
    $null = [IO.Directory]::CreateDirectory($resolved)
    $null = Assert-QaPath $resolved
}

function Initialize-QaRoot {
    New-QaDirectory $testRoot
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = Get-Acl -LiteralPath $testRoot
    Assert-Spike ($acl.GetOwner([Security.Principal.SecurityIdentifier]).Value -eq $sid.Value) 'Synthetic QA root is not owned by the current user.'
    $privateAcl = [Security.AccessControl.DirectorySecurity]::new()
    $privateAcl.SetAccessRuleProtection($true, $false)
    $privateAcl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    [IO.FileSystemAclExtensions]::SetAccessControl([IO.DirectoryInfo]::new($testRoot), $privateAcl)
    $null = Assert-QaPath $testRoot
}

function Write-QaFileAtomically([string]$Path, [string]$Content) {
    $resolved = Assert-QaPath $Path
    $temporary = $resolved + '.' + $invocationId + '.' + [guid]::NewGuid().ToString('N') + '.tmp'
    $null = Assert-QaPath $temporary
    try {
        $stream = [IO.FileStream]::new($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
        try {
            $bytes = [Text.UTF8Encoding]::new($false).GetBytes($Content)
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush($true)
        }
        finally { $stream.Dispose() }
        $null = Assert-QaPath $resolved
        if ([IO.File]::Exists($resolved)) { [IO.File]::Replace($temporary, $resolved, [System.Management.Automation.Language.NullString]::Value) }
        else { [IO.File]::Move($temporary, $resolved) }
    }
    finally {
        $null = Assert-QaPath $temporary
        if ([IO.File]::Exists($temporary)) { [IO.File]::Delete($temporary) }
    }
}

function Initialize-SpikeRuntime {
    if (-not $script:NodeExecutable) { $script:NodeExecutable = Join-Path $repository '.qa/ownvero-runtime-spike/bundle/node.exe' }
    $script:NodeExecutable = [IO.Path]::GetFullPath($script:NodeExecutable)
    Assert-Spike (Test-Path -LiteralPath $script:NodeExecutable -PathType Leaf) 'Packaged Node executable is required.'
    if (-not ('OwnveroSpike.OwnedProcess' -as [type])) { Add-Type -Path $helper }
}

function New-ChildEnvironment([string]$Directory) {
    $environment = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
    $windows = [Environment]::GetFolderPath([Environment+SpecialFolder]::Windows)
    $environment.Add('SystemRoot', $windows)
    $environment.Add('WINDIR', $windows)
    $environment.Add('TEMP', $Directory)
    $environment.Add('TMP', $Directory)
    $environment.Add('USERPROFILE', $Directory)
    $environment.Add('PATH', '')
    return ,$environment
}

function Read-Tree([string]$Directory) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while ($timer.ElapsedMilliseconds -lt 15000) {
        $files = @('root', 'child', 'grandchild') | ForEach-Object { Join-Path $Directory ($_ + '.json') }
        if (@($files | Where-Object { -not (Test-Path -LiteralPath $_ -PathType Leaf) }).Count -eq 0) {
            return @($files | ForEach-Object { Get-Content -LiteralPath $_ -Raw | ConvertFrom-Json })
        }
        Start-Sleep -Milliseconds 50
    }
    throw 'Synthetic tree did not become ready within 15 seconds.'
}

function Open-TreeReferences($Records) {
    foreach ($record in $Records) {
        $reference = [Diagnostics.Process]::GetProcessById([int]$record.pid)
        $null = $reference.Handle # Retain the actual process handle, so PID reuse cannot change this observation.
        Assert-Spike (-not $reference.HasExited) 'Synthetic process exited before observation.'
        $reference
    }
}

function Wait-AllGone($References) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while ($timer.ElapsedMilliseconds -lt 8000) {
        if (@($References | Where-Object { -not $_.HasExited }).Count -eq 0) { return $timer.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 25
    }
    throw 'An owned synthetic descendant survived the 8-second cleanup deadline.'
}

function Assert-JobMembers([int[]]$Expected, [int[]]$Members) {
    Assert-Spike (@($Expected | Where-Object { $Members -notcontains $_ }).Count -eq 0) 'Job did not contain every known synthetic process.'
    foreach ($memberId in @($Members | Where-Object { $Expected -notcontains $_ })) {
        $reference = [Diagnostics.Process]::GetProcessById($memberId)
        try { Assert-Spike ($reference.ProcessName -eq 'conhost') 'Unexpected auxiliary process in the owned job.' }
        finally { $reference.Dispose() }
    }
}

function Assert-Tree($Records, $Owned) {
    $expectedKeys = @('PATH', 'SystemRoot', 'TEMP', 'TMP', 'USERPROFILE', 'WINDIR') | Sort-Object
    foreach ($record in $Records) {
        Assert-Spike $record.argumentIntegrity 'Windows argument round trip failed.'
        Assert-Spike $record.parentCanariesAbsent 'Parent environment was inherited.'
        Assert-Spike ((@($record.environmentKeys | Sort-Object) -join '|') -eq ($expectedKeys -join '|')) 'Child environment contains unexpected entries.'
        Assert-Spike $record.pathEmpty 'Child PATH was populated.'
    }
    $ids = @($Records | ForEach-Object { [int]$_.pid } | Sort-Object)
    $members = @($Owned.ActiveProcessIds() | Sort-Object)
    Assert-JobMembers $ids $members
    Assert-Spike (-not $Owned.WaitForExit(0)) 'WaitForExit should time out for the live synthetic root.'
    $runningExitCodeRejected = $false
    # Invoke explicitly because PowerShell property access can suppress a getter exception.
    try { $null = $Owned.get_ExitCode() }
    catch { $runningExitCodeRejected = $true }
    Assert-Spike $runningExitCodeRejected 'ExitCode must reject a process that is still running.'
    return $members
}

function Start-Tree([string]$Directory) {
    $arguments = [string[]]@((Join-Path (Split-Path $Directory -Parent) 'tree.cjs'), 'root', $Directory,
        '', 'space value', 'quote"value', 'backslash\end\', '$literal`characters', 'unicode-π')
    return [OwnveroSpike.OwnedProcess]::Start($NodeExecutable, $arguments, $Directory, (New-ChildEnvironment $Directory))
}

if ($WorkerMode -eq 'ParentWait') {
    $resolvedCase = Assert-QaPath $CaseDirectory
    Assert-Spike ($resolvedCase -ne $testRoot) 'Worker must use a dedicated case directory.'
    Initialize-SpikeRuntime
    $owned = Start-Tree $resolvedCase
    try {
        $records = Read-Tree $resolvedCase
        $members = @(Assert-Tree $records $owned)
        $ready = [ordered]@{ pid = $PID; inJob = [OwnveroSpike.OwnedProcess]::CurrentProcessIsInJob()
            elevated = [OwnveroSpike.OwnedProcess]::CurrentProcessIsElevated(); jobMembers = $members
        } | ConvertTo-Json -Depth 5
        Write-QaFileAtomically (Join-Path $resolvedCase 'parent-ready.json') $ready
        # The controller forcibly terminates only this known owner. Its inner job handle stays solely here.
        while ($true) { Start-Sleep -Milliseconds 100 }
    }
    finally { $owned.Dispose() }
    exit
}

$runDirectory = Join-Path $testRoot $invocationId
$latestInitialized = $false
$savedEnvironment = @{}
$parentCanaries = @{ OWNVERO_SPIKE_PARENT_ONLY = 'synthetic-parent-canary'; NODE_OPTIONS = '--invalid-ownvero-spike-canary'; HTTP_PROXY = 'http://127.0.0.1:9'; HTTPS_PROXY = 'http://127.0.0.1:9' }
try {
    Initialize-QaRoot
    # Invalidate older success before executable lookup, compilation or per-run setup.
    Write-QaFileAtomically (Join-Path $testRoot 'latest-result.json') ($evidence | ConvertTo-Json -Depth 8)
    $latestInitialized = $true
    New-QaDirectory $runDirectory
    Write-QaFileAtomically (Join-Path $runDirectory 'result.json') ($evidence | ConvertTo-Json -Depth 8)
    Initialize-SpikeRuntime
    $evidence.platform = [Environment]::OSVersion.VersionString
    $evidence.pointerBytes = [IntPtr]::Size
    $evidence.helperSha256 = (Get-FileHash -LiteralPath $helper -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence.testSha256 = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence.nodeSha256 = (Get-FileHash -LiteralPath $NodeExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence.elevated = [OwnveroSpike.OwnedProcess]::CurrentProcessIsElevated()
    $evidence.harnessInJob = [OwnveroSpike.OwnedProcess]::CurrentProcessIsInJob()
$treeSource = @'
const fs = require('node:fs');
const path = require('node:path');
const { spawn } = require('node:child_process');
const [role, directory, ...args] = process.argv.slice(2);
const expected = ['', 'space value', 'quote"value', 'backslash\\end\\', '$literal`characters', 'unicode-π'];
const next = role === 'root' ? 'child' : role === 'child' ? 'grandchild' : null;
if (next) spawn(process.execPath, [__filename, next, directory, ...args], {
  cwd: directory, env: process.env, stdio: 'ignore', windowsHide: true, shell: false
});
const record = {
  pid: process.pid, parentPid: process.ppid, role,
  argumentIntegrity: JSON.stringify(args) === JSON.stringify(expected),
  environmentKeys: Object.keys(process.env).sort(), pathEmpty: !process.env.PATH,
  parentCanariesAbsent: ['OWNVERO_SPIKE_PARENT_ONLY', 'NODE_OPTIONS', 'HTTP_PROXY', 'HTTPS_PROXY']
    .every(key => process.env[key] === undefined)
};
const destination = path.join(directory, `${role}.json`);
fs.writeFileSync(`${destination}.tmp`, JSON.stringify(record));
fs.renameSync(`${destination}.tmp`, destination);
if (role === 'exit') process.exit(23);
setInterval(() => {}, 1000);
'@
    Write-QaFileAtomically (Join-Path $runDirectory 'tree.cjs') $treeSource
foreach ($name in $parentCanaries.Keys) {
    $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
    [Environment]::SetEnvironmentVariable($name, $parentCanaries[$name], 'Process')
}
    Assert-Spike (-not $evidence.elevated) 'Spike must run with a non-elevated token.'
    $directory = Join-Path $runDirectory 'normal-exit'
    New-QaDirectory $directory
    $arguments = [string[]]@((Join-Path $runDirectory 'tree.cjs'), 'exit', $directory)
    $owned = [OwnveroSpike.OwnedProcess]::Start($NodeExecutable, $arguments, $directory, (New-ChildEnvironment $directory))
    try {
        Assert-Spike ($owned.WaitForExit(5000)) 'Normal synthetic process did not exit.'
        Assert-Spike ($owned.ExitCode -eq 23) 'Nonzero exit code was not preserved.'
        Assert-Spike (Test-Path -LiteralPath (Join-Path $directory 'exit.json')) 'Normal child did not write its fixed result file.'
        $evidence.results += [ordered]@{ check = 'normal-exit'; passed = $true; exitCode = $owned.ExitCode }
    }
    finally { $owned.Dispose() }
    foreach ($mode in @('cancel', 'dispose')) {
        $directory = Join-Path $runDirectory $mode
        New-QaDirectory $directory
        $owned = $null
        $references = @()
        try {
            $owned = Start-Tree $directory
            $records = Read-Tree $directory
            $members = @(Assert-Tree $records $owned)
            $references = @(Open-TreeReferences @($members | ForEach-Object { @{ pid = $_ } }))
            if ($mode -eq 'cancel') {
                $owned.Terminate()
                Assert-Spike ($owned.WaitForExit(5000)) 'Cancelled root did not exit.'
            }
            else { $owned.Dispose() }
            $elapsed = Wait-AllGone $references
            if ($mode -eq 'cancel') { Assert-Spike ($owned.ActiveProcessIds().Length -eq 0) 'Cancelled job is not empty.' }
            $evidence.results += [ordered]@{ check = $mode; passed = $true; ownedProcessIds = $members
                nodeDescendantCount = 2; auxiliaryConsoleCount = $members.Count - 3
                cleanupObservedMilliseconds = $elapsed; environmentScrubbed = $true; argumentIntegrity = $true }
        }
        finally {
            if ($owned) { $owned.Dispose() }
            foreach ($reference in $references) { $reference.Dispose() }
        }
    }

    $directory = Join-Path $runDirectory 'parent-crash'
    New-QaDirectory $directory
    $powerShellExecutable = Join-Path $PSHOME 'pwsh.exe'
    Assert-Spike (Test-Path -LiteralPath $powerShellExecutable -PathType Leaf) 'PowerShell 7 executable is required for the parent-crash helper.'
    $outer = $null
    $parentReference = $null
    $references = @()
    try {
        $arguments = [string[]]@('-NoLogo', '-NoProfile', '-NonInteractive', '-File', $PSCommandPath,
            '-NodeExecutable', $NodeExecutable, '-WorkerMode', 'ParentWait', '-CaseDirectory', $directory)
        $outer = [OwnveroSpike.OwnedProcess]::Start($powerShellExecutable, $arguments, $directory, (New-ChildEnvironment $directory))
        $parentReference = [Diagnostics.Process]::GetProcessById($outer.ProcessId)
        $null = $parentReference.Handle
        $readyFile = Join-Path $directory 'parent-ready.json'
        $timer = [Diagnostics.Stopwatch]::StartNew()
        while (-not (Test-Path -LiteralPath $readyFile -PathType Leaf) -and $timer.ElapsedMilliseconds -lt 20000) { Start-Sleep -Milliseconds 50 }
        Assert-Spike (Test-Path -LiteralPath $readyFile -PathType Leaf) 'Nested parent did not become ready.'
        $ready = Get-Content -LiteralPath $readyFile -Raw | ConvertFrom-Json
        Assert-Spike ($ready.inJob -and -not $ready.elevated) 'Nested parent job/token invariant failed.'
        $records = Read-Tree $directory
        $outerMembers = @($outer.ActiveProcessIds())
        Assert-JobMembers @($records.pid) @($ready.jobMembers)
        Assert-JobMembers @($outer.ProcessId; $ready.jobMembers) $outerMembers
        $references = @(Open-TreeReferences @($outerMembers | ForEach-Object { @{ pid = $_ } }))
        $parentReference.Kill() # Abrupt owner death; deliberately keep the outer job handle open.
        Assert-Spike ($outer.WaitForExit(5000)) 'Terminated helper parent did not exit.'
        $elapsed = Wait-AllGone $references
        Assert-Spike ($outer.ActiveProcessIds().Length -eq 0) 'Outer job still has survivors after inner owner death.'
        $evidence.results += [ordered]@{ check = 'abrupt-parent-death-and-nested-job'; passed = $true
            helperParentId = $outer.ProcessId; ownedProcessIds = $outerMembers; nestedJobProcessIds = @($ready.jobMembers)
            cleanupObservedMilliseconds = $elapsed; outerJobHeldOpenDuringObservation = $true; nestedJobAssignedBeforeResume = $true }
    }
    finally {
        if ($outer) { $outer.Dispose() }
        if ($parentReference) { $parentReference.Dispose() }
        foreach ($reference in $references) { $reference.Dispose() }
    }
    $evidence.passed = $true
    $evidence.status = 'passed'
    $evidence.exitCode = 0
}
catch {
    $evidence.status = 'failed'
    $evidence.exitCode = 1
    $evidence.failure = [ordered]@{ type = $_.Exception.GetType().Name; message = $_.Exception.Message }
}
finally {
    foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], 'Process') }
    $evidence.completedUtc = [DateTime]::UtcNow.ToString('o')
    if ($latestInitialized) {
        try {
            New-QaDirectory $runDirectory
            $json = $evidence | ConvertTo-Json -Depth 8
            Write-QaFileAtomically (Join-Path $runDirectory 'result.json') $json
            Write-QaFileAtomically (Join-Path $testRoot 'latest-result.json') $json
        }
        catch {
            $evidence.passed = $false; $evidence.status = 'failed'; $evidence.exitCode = 1
            $evidence.evidenceWriteFailure = $_.Exception.GetType().Name
            # The earlier atomic started record remains passed:false if storage becomes unwritable.
        }
    }
    Write-Output ($evidence | ConvertTo-Json -Depth 8)
}
exit $evidence.exitCode
