param([switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$buildRoot = Join-Path $projectRoot 'var/local-model-build'
$entryPoint = Join-Path $PSScriptRoot 'companion.py'

if (-not $SkipBuild) {
uv run --project $PSScriptRoot --with 'pyinstaller==6.22.3' pyinstaller `
  --onefile `
  --name iTanongMoLocalModel `
  --distpath (Join-Path $buildRoot 'dist') `
  --workpath (Join-Path $buildRoot 'work') `
  --specpath (Join-Path $buildRoot 'spec') `
  --collect-submodules transformers.models.deberta_v2 `
  --collect-submodules transformers.pipelines `
  --collect-data transformers `
  --copy-metadata transformers `
  --copy-metadata huggingface-hub `
  --copy-metadata tokenizers `
  --copy-metadata safetensors `
  --noconfirm `
  $entryPoint
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$executable = Join-Path $buildRoot 'dist/iTanongMoLocalModel.exe'
$file = Get-Item -LiteralPath $executable
if ($file.Length -ge 2GB) { throw 'EXE exceeds the GitHub Release asset limit.' }
$source = [System.IO.File]::OpenRead($executable)
try {
  $digest = [System.Security.Cryptography.SHA256]::Create().ComputeHash($source)
} finally {
  $source.Dispose()
}
$sha256 = [System.BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant()
[pscustomobject]@{ file = $file.FullName; bytes = $file.Length; sha256 = $sha256 } |
  ConvertTo-Json -Compress
