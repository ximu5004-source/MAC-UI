param(
    [Parameter(Mandatory = $true)]
    [string]$RuntimeCab,
    [Parameter(Mandatory = $false)]
    [ValidateSet("x64", "arm64")]
    [string]$Architecture = "x64",
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$')]
    [string]$Version
)
$ErrorActionPreference = "Stop"
# Obtain the CAB directly from Microsoft; this script never downloads from a mirror.
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$cabPath = (Resolve-Path -LiteralPath $RuntimeCab).Path
$expectedName = "Microsoft.WebView2.FixedVersionRuntime.$Version.$Architecture.cab"
if ((Split-Path -Leaf $cabPath) -ne $expectedName) {
    throw "Expected CAB filename: $expectedName"
}
$runtimeRoot = Join-Path $projectRoot "src/runtime"
$runtimeTarget = Join-Path $runtimeRoot $Version
if (Test-Path -LiteralPath $runtimeTarget) {
    throw "Runtime target already exists; nothing was replaced: $runtimeTarget"
}
$stage = Join-Path $runtimeRoot ("extract-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $stage -Force | Out-Null
& expand.exe $cabPath "-f:*" $stage
if ($LASTEXITCODE -ne 0) { throw "CAB extraction failed. Staging files retained at $stage" }
$extracted = Join-Path $stage ([IO.Path]::GetFileNameWithoutExtension($expectedName))
if (!(Test-Path -LiteralPath (Join-Path $extracted "msedgewebview2.exe"))) {
    throw "Invalid runtime layout. Staging files retained at $stage"
}
# All move targets are resolved under this exact runtime root; no recursive deletion is used.
$resolvedExtracted = [IO.Path]::GetFullPath($extracted)
$resolvedTarget = [IO.Path]::GetFullPath($runtimeTarget)
$allowedPrefix = [IO.Path]::GetFullPath($runtimeRoot) + [IO.Path]::DirectorySeparatorChar
if (!$resolvedExtracted.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase) -or
    !$resolvedTarget.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Runtime paths escaped their staging root"
}
Move-Item -LiteralPath $resolvedExtracted -Destination $resolvedTarget
$configPath = Join-Path $projectRoot "src/tauri.conf.json"
Copy-Item -LiteralPath $configPath -Destination ($configPath + ".runtime-backup-" + [guid]::NewGuid().ToString())
$config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
$config.bundle.windows | Add-Member -Force -MemberType NoteProperty -Name "webviewInstallMode" -Value ([PSCustomObject]@{
    type = "fixedRuntime"
    path = "runtime/$Version"
})
$config | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $configPath -Encoding UTF8
Write-Host "Configured local Microsoft WebView2 runtime: $runtimeTarget"
