param([string]$OutputDirectory = (Join-Path $env:TEMP 'macui-native-test-bin'))
$ErrorActionPreference = 'Stop'
$repository = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$env:MACUI_NATIVE_OUTPUT = $OutputDirectory
& node (Join-Path $repository 'scripts/build/desktop-native.mjs') --test
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$testArguments = @((Join-Path $OutputDirectory 'desktop-locales.json'))
& (Join-Path $OutputDirectory 'macui-native-tests.exe') @testArguments
exit $LASTEXITCODE
