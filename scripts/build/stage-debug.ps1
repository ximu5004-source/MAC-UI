param(
    [Parameter(Mandatory = $true)][string]$Destination,
    [string]$BuildDirectory = (Join-Path $PSScriptRoot '../../target/debug')
)
$ErrorActionPreference = 'Stop'
$buildRoot = (Resolve-Path -LiteralPath $BuildDirectory).Path
$outputRoot = [IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $outputRoot) { throw 'Choose a new staging directory.' }
$manifest = Join-Path $buildRoot 'SHA256SUMS'
$entries = foreach ($line in Get-Content -LiteralPath $manifest) {
    if ($line -notmatch '^([0-9a-fA-F]{64})  (static[/\\].+)$') { throw ('Invalid resource entry: ' + $line) }
    $expected = $Matches[1]
    $relative = $Matches[2].Replace('/', '\')
    $source = [IO.Path]::GetFullPath((Join-Path $buildRoot $relative))
    if (!$source.StartsWith((Join-Path $buildRoot 'static') + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw ('Resource path is outside the build directory: ' + $relative)
    }
    if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $expected) {
        throw ('Build resource does not match its manifest: ' + $relative)
    }
    [PSCustomObject]@{Relative = $relative; Source = $source}
}
if (!$entries.Count) { throw 'The resource manifest is empty.' }
New-Item -ItemType Directory -Path $outputRoot | Out-Null
foreach ($name in @('macui.exe','macui-service.exe','macui-cli.exe','LICENSE','NOTICE.md','SHA256SUMS','SHA256SUMS.sig')) {
    Copy-Item -LiteralPath (Join-Path $buildRoot $name) -Destination (Join-Path $outputRoot $name)
}
Copy-Item -LiteralPath (Join-Path $buildRoot 'macui_hook.dll') -Destination (Join-Path $outputRoot 'macui-hook.dll')
foreach ($entry in $entries) {
    $target = Join-Path $outputRoot $entry.Relative
    New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($target)) -Force | Out-Null
    Copy-Item -LiteralPath $entry.Source -Destination $target
}
Write-Output ('Staged debug runtime with ' + $entries.Count + ' verified resources: ' + $outputRoot)
