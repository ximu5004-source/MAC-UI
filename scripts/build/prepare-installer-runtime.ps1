param([string]$Redistributable)
$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$destination = Join-Path $projectRoot 'src/installer-runtime'
New-Item -ItemType Directory -Path $destination -Force | Out-Null
$package = Join-Path $destination 'vc_redist.x64.exe'
if ($Redistributable) {
    $source = (Resolve-Path -LiteralPath $Redistributable).Path
    if (![string]::Equals($source, $package, [StringComparison]::OrdinalIgnoreCase)) {
        Copy-Item -LiteralPath $source -Destination $package
    }
} elseif (!(Test-Path -LiteralPath $package)) {
    Invoke-WebRequest -Uri 'https://aka.ms/vc14/vc_redist.x64.exe' -OutFile $package
}
$signature = Get-AuthenticodeSignature -LiteralPath $package
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') {
    throw 'The Visual C++ runtime must have a valid Microsoft signature.'
}
$version = (Get-Item -LiteralPath $package).VersionInfo.ProductVersion
if ($version -notmatch '^14\.\d+\.\d+\.\d+$') { throw 'The Visual C++ runtime version is invalid.' }
Set-Content -LiteralPath (Join-Path $destination 'runtime.nsh') -Value ('!define MACUI_VC_VERSION "' + $version + '"') -Encoding utf8
Write-Output ('Prepared Microsoft Visual C++ runtime ' + $version)
