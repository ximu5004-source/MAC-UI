param(
    [Parameter(Mandatory = $true)]
    [string]$InstallDirectory,
    [switch]$ListOnly
)
$ErrorActionPreference = 'Stop'

function Test-MacUiInstallerProcess {
    param([string]$ProcessPath, [string]$Directory)
    if ([string]::IsNullOrWhiteSpace($ProcessPath)) { return $false }
    $knownNames = @('macui.exe', 'macui-service.exe', 'mac-ui.exe', 'mac-ui-service.exe', 'seelen-ui.exe', 'slu-service.exe')
    try {
        $file = [IO.Path]::GetFullPath($ProcessPath)
        $parent = [IO.Path]::GetDirectoryName($file).TrimEnd([char[]]'\/')
        $target = [IO.Path]::GetFullPath($Directory).TrimEnd([char[]]'\/')
        return ($knownNames -contains [IO.Path]::GetFileName($file)) -and
            [string]::Equals($parent, $target, [StringComparison]::OrdinalIgnoreCase)
    } catch { return $false }
}

function Test-MacUiNativeDesktopProcess {
    param([string]$ProcessPath, [string]$Directory)
    if ([string]::IsNullOrWhiteSpace($ProcessPath)) { return $false }
    $expected = [IO.Path]::GetFullPath((Join-Path $Directory 'static/native/macui-desktop.exe'))
    return [string]::Equals([IO.Path]::GetFullPath($ProcessPath), $expected, [StringComparison]::OrdinalIgnoreCase)
}

$installPath = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd([char[]]'\/')
if ([string]::Equals($installPath, [IO.Path]::GetPathRoot($installPath).TrimEnd([char[]]'\/'), [StringComparison]::OrdinalIgnoreCase)) {
    throw 'An installation directory cannot be a drive root.'
}

# Dot-sourcing exposes the ownership predicate to the non-mutating regression check.
if ($MyInvocation.InvocationName -eq '.') { return }

$owned = @(Get-CimInstance Win32_Process | Where-Object {
    Test-MacUiInstallerProcess -ProcessPath $_.ExecutablePath -Directory $installPath
})
if ($ListOnly) {
    $owned | Select-Object ProcessId, Name, ExecutablePath
    Get-CimInstance Win32_Process | Where-Object {
        Test-MacUiNativeDesktopProcess -ProcessPath $_.ExecutablePath -Directory $installPath
    } | Select-Object ProcessId, Name, ExecutablePath
    return
}

# Stop the watchdog before its GUI. Otherwise it can relaunch the old GUI while
# the installer is replacing it. Give the service time to restore native state.
foreach ($service in @($owned | Where-Object { $_.Name -match 'service' })) {
    $live = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $service.ProcessId)
    if (!$live -or !(Test-MacUiInstallerProcess -ProcessPath $live.ExecutablePath -Directory $installPath)) { continue }
    $client = Start-Process -FilePath $live.ExecutablePath -ArgumentList 'stop' -WindowStyle Hidden -PassThru
    if (!$client.WaitForExit(5000)) { $client.Kill(); $client.WaitForExit() }
    $instance = Get-Process -Id $service.ProcessId -ErrorAction SilentlyContinue
    if ($instance) { $null = $instance.WaitForExit(5000) }
}

$native = @(Get-CimInstance Win32_Process | Where-Object {
    Test-MacUiNativeDesktopProcess -ProcessPath $_.ExecutablePath -Directory $installPath
})
foreach ($desktop in @($native | Where-Object { $_.CommandLine -notmatch '--watchdog' })) {
    $instance = Get-Process -Id $desktop.ProcessId -ErrorAction SilentlyContinue
    if (!$instance) { continue }
    $stop = [Threading.EventWaitHandle]::OpenExisting('Local\macui.desktop.stop.' + $desktop.ProcessId)
    try { $null = $stop.Set() } finally { $stop.Dispose() }
    if (!$instance.WaitForExit(120000)) { throw 'Desktop file operations are still running. Finish or cancel them before updating MAC UI.' }
}
foreach ($watchdog in @($native | Where-Object { $_.CommandLine -match '--watchdog' })) {
    $instance = Get-Process -Id $watchdog.ProcessId -ErrorAction SilentlyContinue
    if ($instance -and !$instance.WaitForExit(15000)) { throw 'Native desktop recovery is still running. The installation files have been preserved.' }
}

# Refresh after the graceful stop: the watchdog may have relaunched a GUI just
# before acknowledging Stop. Revalidate ownership before every termination.
$owned = @(Get-CimInstance Win32_Process | Where-Object {
    Test-MacUiInstallerProcess -ProcessPath $_.ExecutablePath -Directory $installPath
})
foreach ($process in ($owned | Sort-Object { $_.Name -notmatch 'service' })) {
    $instance = Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue
    if (!$instance) { continue }
    if ($instance.CloseMainWindow()) {
        $null = $instance.WaitForExit(1500)
    }
    if (!$instance.HasExited) {
        # Re-check the live executable path to avoid acting on a reused process ID.
        $live = Get-CimInstance Win32_Process -Filter ('ProcessId=' + $process.ProcessId)
        if ($live -and (Test-MacUiInstallerProcess -ProcessPath $live.ExecutablePath -Directory $installPath)) {
            Stop-Process -Id $process.ProcessId -Force
            Wait-Process -Id $process.ProcessId -Timeout 10 -ErrorAction SilentlyContinue
        }
    }
}

$remaining = @(Get-CimInstance Win32_Process | Where-Object {
    (Test-MacUiInstallerProcess -ProcessPath $_.ExecutablePath -Directory $installPath) -or
    (Test-MacUiNativeDesktopProcess -ProcessPath $_.ExecutablePath -Directory $installPath)
})
if ($remaining.Count) { throw 'The previous installation has not exited; refusing to replace running binaries.' }
