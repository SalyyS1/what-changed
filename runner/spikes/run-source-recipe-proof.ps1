param([string]$RunRoot)
$ErrorActionPreference='Stop'
$repo=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$bundle=Join-Path $repo '.qa\ownvero-runtime-spike\bundle'
if(-not $RunRoot){$RunRoot=Join-Path $repo ('.qa\ownvero-source-proof-'+[Guid]::NewGuid().ToString('N'))}
$run=[IO.Path]::GetFullPath($RunRoot)
if(-not $run.StartsWith((Join-Path $repo '.qa')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Source proof root must remain in this project QA directory'}
$inventory=Get-Content -LiteralPath (Join-Path $bundle 'bundle-inventory.json') -Raw|ConvertFrom-Json
$node=Join-Path $bundle 'node.exe'
$entry=@($inventory.files|Where-Object path -eq 'node.exe')
if($entry.Count-ne 1 -or (Get-FileHash -LiteralPath $node -Algorithm SHA256).Hash.ToLowerInvariant()-ne $entry[0].sha256){throw 'Packaged Node changed'}
New-Item -ItemType Directory -Path $run -ErrorAction Stop|Out-Null
Add-Type -Path (Join-Path $PSScriptRoot 'windows-owned-process.cs')
$child=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::OrdinalIgnoreCase)
$child['SystemRoot']=$env:SystemRoot;$child['TEMP']=$run;$child['TMP']=$run;$child['USERPROFILE']=$run;$child['LOCALAPPDATA']=$run;$child['PATH']=''
$child['WINDIR']=$env:SystemRoot;$child['SystemDrive']=[IO.Path]::GetPathRoot($env:SystemRoot).TrimEnd('\')
$child['OWNVERO_TEST_BROWSER_BUNDLE']=$bundle;$child['OWNVERO_TEST_RUN_ROOT']=$run
$job=$null
try{
  $job=[OwnveroSpike.OwnedProcess]::Start($node,[string[]]@((Join-Path $PSScriptRoot 'source-recipe-proof.mjs')),$run,$child)
  if(-not $job.WaitForExit(300000)){$job.Terminate();throw 'Current-source synthetic proof timed out'}
  $code=$job.ExitCode
  for($attempt=0;$attempt-lt 20 -and @($job.ActiveProcessIds()).Count-ne 0;$attempt++){Start-Sleep -Milliseconds 100}
  if(@($job.ActiveProcessIds()).Count-ne 0){$job.Terminate();throw 'Synthetic browser descendants survived completion'}
  if(-not (Test-Path -LiteralPath (Join-Path $run 'source-recipe-proof.json'))){throw "Synthetic source exited $code without structured proof. Root: $run"}
  Get-Content -LiteralPath (Join-Path $run 'source-recipe-proof.json') -Raw
  if($code-ne 0){throw 'Synthetic source proof did not match its expected observations'}
}finally{if($null-ne $job){$job.Dispose()}}
