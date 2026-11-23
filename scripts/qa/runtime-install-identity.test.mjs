import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import test from "node:test";

const read = (file) => readFileSync(new URL(`../../${file}`, import.meta.url), "utf8");

test("MSIX executable names, execution aliases and resource association match runtime names", () => {
  const xml = read("src/templates/AppxManifest.xml");
  for (const executable of ["macui.exe", "macui-cli.exe", "macui-service.exe"]) {
    assert.ok(xml.includes(`Executable="${executable}"`), executable);
  }
  assert.match(xml, /Alias="macui-cli\.exe"/);
  assert.match(xml, /Alias="macui-service\.exe"/);
  assert.match(xml, /<uap:FileType>\.maui<\/uap:FileType>/);
  assert.doesNotMatch(xml, /(?:Executable|Alias)="(?:slu|seelen|mac-ui)/);
  const script = read("scripts/bundle.msix.ts");
  assert.match(script, /macui_hook\.dll.*macui-hook\.dll/);
  assert.match(script, /fs\.copyFileSync\("LICENSE"/);
  assert.match(script, /fs\.copyFileSync\("NOTICE\.md"/);
  assert.doesNotMatch(script, /pre \|\| target === "release"/);
  assert.match(read("src/background/cli/svc_pipe.rs"), /WindowsApps\\\\macui-service\.exe/);
});

test("installer upgrade stops only known processes in its installation directory", () => {
  const file = fileURLToPath(new URL("../../src/templates/installer-processes.ps1", import.meta.url));
  const command = `
    . '${file.replaceAll("'", "''")}' -InstallDirectory 'C:/Apps/MAC UI' -ListOnly
    $cases = @(
      @{ Path='C:/Apps/MAC UI/macui.exe'; Expected=$true },
      @{ Path='C:/Apps/MAC UI/macui-service.exe'; Expected=$true },
      @{ Path='C:/Apps/MAC UI/seelen-ui.exe'; Expected=$true },
      @{ Path='C:/Apps/Other/seelen-ui.exe'; Expected=$false },
      @{ Path='C:/Apps/MAC UI/sub/macui.exe'; Expected=$false },
      @{ Path='C:/Apps/MAC UI/not-macui.exe'; Expected=$false },
      @{ Path=''; Expected=$false }
    )
    foreach ($case in $cases) {
      $actual = Test-MacUiInstallerProcess -ProcessPath $case.Path -Directory 'C:/Apps/MAC UI'
      if ($actual -ne $case.Expected) { throw ('Incorrect ownership: ' + $case.Path) }
    }
    if (!(Test-MacUiNativeDesktopProcess -ProcessPath 'C:/Apps/MAC UI/static/native/macui-desktop.exe' -Directory 'C:/Apps/MAC UI')) { throw 'Native desktop ownership was not recognized' }
    foreach ($path in @('C:/Apps/Other/static/native/macui-desktop.exe', 'C:/Apps/MAC UI/macui-desktop.exe', 'C:/Apps/MAC UI/static/native/other.exe')) {
      if (Test-MacUiNativeDesktopProcess -ProcessPath $path -Directory 'C:/Apps/MAC UI') { throw ('Unrelated native process was included: ' + $path) }
    }
    Write-Output 'ownership passed'
  `;
  const result = spawnSync("powershell.exe", [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    command,
  ], {
    encoding: "utf8",
    windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /ownership passed/);
  const hooks = read("src/templates/installer-hooks.nsh");
  assert.doesNotMatch(hooks, /taskkill.*\/IM/i);
  assert.match(hooks, /macui-installer-processes\.ps1/);
});

test("loopback API and logs use MAC UI names without an external docs dependency", () => {
  assert.match(read("src/background/server/http/mod.rs"), /OpenApi::new\("MAC UI Local API"/);
  assert.doesNotMatch(read("src/background/server/http/scalar.html"), /https?:\/\//);
  assert.match(read("src/background/logger.rs"), /join\("macui\.log"\)/);
  assert.match(read("src/service/logger.rs"), /join\("macui-service\.log"\)/);
  assert.doesNotMatch(read("src/templates/installer.nsi"), /discord\.seelen\.io/);
});

test("upgrade stops the watchdog first and catches a GUI relaunched during shutdown", () => {
  const file = fileURLToPath(new URL("../../src/templates/installer-processes.ps1", import.meta.url));
  const command = `
    $global:live = @(
      [pscustomobject]@{ProcessId=11;Name='macui.exe';ExecutablePath='C:/Apps/MAC UI/macui.exe'},
      [pscustomobject]@{ProcessId=22;Name='macui-service.exe';ExecutablePath='C:/Apps/MAC UI/macui-service.exe'}
    )
    $global:events = [Collections.Generic.List[string]]::new()
    function Get-CimInstance { param($ClassName,$Filter)
      if ($Filter) { return @($global:live | Where-Object { ('ProcessId=' + $_.ProcessId) -eq $Filter }) }
      return $global:live
    }
    function Start-Process { param($FilePath,$ArgumentList,$WindowStyle,[switch]$PassThru)
      if ($ArgumentList -ne 'stop' -or $WindowStyle -ne 'Hidden') { throw 'Wrong stop request' }
      $global:events.Add('service-stop')
      $global:live = @($global:live | Where-Object ProcessId -ne 22)
      $global:live += [pscustomobject]@{ProcessId=33;Name='macui.exe';ExecutablePath='C:/Apps/MAC UI/macui.exe'}
      $client = [pscustomobject]@{}
      $client | Add-Member ScriptMethod WaitForExit { param($timeout) return $true }
      return $client
    }
    function Get-Process { param($Id,$ErrorAction)
      if (!($global:live | Where-Object ProcessId -eq $Id)) { return $null }
      $process = [pscustomobject]@{HasExited=$false}
      $process | Add-Member ScriptMethod CloseMainWindow { return $false }
      return $process
    }
    function Stop-Process { param($Id,[switch]$Force)
      $global:events.Add('kill-' + $Id)
      $global:live = @($global:live | Where-Object ProcessId -ne $Id)
    }
    function Wait-Process { param($Id,$Timeout,$ErrorAction) }
    & '${file.replaceAll("'", "''")}' -InstallDirectory 'C:/Apps/MAC UI'
    if ($global:events[0] -ne 'service-stop' -or (($global:events | Select-Object -Skip 1 | Sort-Object) -join ',') -ne 'kill-11,kill-33') { throw ($global:events -join ',') }
    if ($global:live.Count) { throw 'Owned processes survived' }
    Write-Output 'upgrade quiescence passed'
  `;
  const result = spawnSync("powershell.exe", [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-Command",
    command,
  ], {
    encoding: "utf8",
    windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /upgrade quiescence passed/);
});
