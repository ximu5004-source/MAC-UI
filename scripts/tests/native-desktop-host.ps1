param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [ValidateSet('Pipe', 'EndOfInput', 'Installer')][string]$StopMethod = 'Pipe'
)
$ErrorActionPreference = 'Stop'
$binary = (Resolve-Path -LiteralPath $Executable).Path
$testRoot = Join-Path $env:TEMP ('macui-desktop-host-' + [Guid]::NewGuid().ToString('N'))
$files = Join-Path $testRoot 'files'
$config = Join-Path $testRoot 'config'
New-Item -ItemType Directory -Path $files, $config | Out-Null
Set-Content -LiteralPath (Join-Path $files 'test.txt') -Value 'MAC UI native host test'
$info = [Diagnostics.ProcessStartInfo]::new($binary)
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
$info.RedirectStandardInput = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($argument in @('--preview', '--root', $files, '--data', $config, '--locale', 'zh-CN', '--parent', [string]$PID)) { $info.ArgumentList.Add($argument) }
$child = [Diagnostics.Process]::Start($info)
function Read-Response([string]$Type) {
    do {
        $line = $child.StandardOutput.ReadLineAsync().WaitAsync([TimeSpan]::FromSeconds(20)).GetAwaiter().GetResult()
        if ($null -eq $line) { throw ('Desktop exited before responding: ' + $child.StandardError.ReadToEnd()) }
        $message = $line | ConvertFrom-Json
        if ($message.type -eq 'error') { throw ($message.payload | ConvertTo-Json -Compress) }
    } while ($message.type -ne $Type)
    return $message.payload
}
function Send-Request($Message) {
    $child.StandardInput.WriteLine(($Message | ConvertTo-Json -Compress))
    $child.StandardInput.Flush()
}
try {
    $ready = Read-Response 'ready'
    if ($ready.pid -ne $child.Id -or !$ready.windows.Count) { throw 'Native windows did not report ready' }
    Send-Request @{type='controls'; id='size'; patch=@{iconSize=64}}
    $response = Read-Response 'controls'
    if ($response.id -ne 'size' -or $response.controls.iconSize -ne 64) { throw 'Settings response did not match' }
    Send-Request @{type='controls'; id='glass'; patch=@{glassTransparency=96; glassBlur=65}}
    $response = Read-Response 'controls'
    if ($response.id -ne 'glass' -or $response.controls.glassTransparency -ne 96 -or $response.controls.glassBlur -ne 65) { throw 'Glass settings response did not match' }
    Send-Request @{type='controls'; id='invalid'; patch=@{iconSize=999}}
    $response = Read-Response 'controls'
    if ($response.id -ne 'invalid' -or !$response.error) { throw 'Invalid settings request was accepted' }
    Send-Request @{type='settings'; locale='en'; contrast=$true}
    if ($StopMethod -eq 'Installer') {
        $installDirectory = [IO.Directory]::GetParent([IO.Path]::GetDirectoryName($binary)).Parent.FullName
        $expected = Join-Path $installDirectory 'static/native/macui-desktop.exe'
        if (![string]::Equals($binary, $expected, [StringComparison]::OrdinalIgnoreCase)) { throw 'Installer tests require an isolated static/native installation structure' }
        & (Join-Path $PSScriptRoot '../../src/templates/installer-processes.ps1') -InstallDirectory $installDirectory
    } elseif ($StopMethod -eq 'EndOfInput') {
        $child.StandardInput.Close()
    } else { Send-Request @{type='stop'} }
    if (!$child.WaitForExit(15000) -or $child.ExitCode -ne 0) { throw 'Native desktop did not stop normally' }
    $saved = Get-Content -Raw -LiteralPath (Join-Path $config 'desktop-shell.json') | ConvertFrom-Json
    if ($saved.iconSize -ne 64) { throw 'Settings were not persisted' }
    if ($saved.glassTransparency -ne 96 -or $saved.glassBlur -ne 65) { throw 'Glass settings were not persisted' }
    Write-Output ('PASS: native WPF startup, settings transport, validation, persistence and ' + $StopMethod + ' shutdown')
} finally {
    if (!$child.HasExited) { $child.Kill(); $child.WaitForExit() }
    $child.Dispose()
}
