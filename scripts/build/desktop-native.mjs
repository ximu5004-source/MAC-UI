import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildDesktopLocales } from "./desktop-locales.mjs";

const root = fileURLToPath(new URL("../../", import.meta.url));
const framework = path.join(process.env.SystemRoot, "Microsoft.NET/Framework64/v4.0.30319");
const source = path.join(root, "native/DesktopOrganizer");
const output = process.env.MACUI_NATIVE_OUTPUT || path.join(root, "src/static/native");
mkdirSync(output, { recursive: true });
buildDesktopLocales(output);
const release = process.env.MACUI_NATIVE_CONFIGURATION === "Release";
const sdkRoot = path.join(process.env["ProgramFiles(x86)"], "Windows Kits/10/UnionMetadata");
const facadeRoot = path.join(process.env.SystemRoot, "Microsoft.NET/assembly/GAC_MSIL/System.Runtime");
const facade = readdirSync(facadeRoot).find((name) => name.startsWith("v4.0_4.0.0.0_"));
if (!facade) throw new Error(".NET Framework 4.8 is required to build the native desktop.");
const sdk = readdirSync(sdkRoot).filter((name) => /^10\./.test(name)).sort((a, b) =>
  a.localeCompare(b, undefined, { numeric: true })
).at(-1);
if (!sdk) throw new Error("The Windows 10 SDK is required to build the native desktop.");
const testing = process.argv.includes("--test");
const executable = testing ? "macui-native-tests.exe" : "macui-desktop.exe";
copyFileSync(path.join(source, "App.config"), path.join(output, executable + ".config"));
execFileSync(path.join(framework, "csc.exe"), [
  "/nologo",
  testing ? "/target:exe" : "/target:winexe",
  "/platform:x64",
  release ? "/optimize+" : "/debug+",
  "/utf8output",
  testing ? "/main:NativeTests" : "/main:MacUI.Desktop.Native.EntryPoint",
  "/out:" + path.join(output, executable),
  "/win32manifest:" + path.join(root, "native/DesktopOrganizer/app.manifest"),
  "/win32icon:" + path.join(root, "src/static/icons/mac-ui/icon.ico"),
  "/reference:System.Web.Extensions.dll",
  "/reference:System.Xaml.dll",
  "/reference:System.Windows.Forms.dll",
  "/reference:System.Numerics.dll",
  "/reference:Microsoft.CSharp.dll",
  "/reference:" +
  path.join(
    process.env.SystemRoot,
    "Microsoft.NET/assembly/GAC_MSIL/System.Numerics.Vectors/v4.0_4.0.0.0__b03f5f7f11d50a3a/System.Numerics.Vectors.dll",
  ),
  "/reference:" + path.join(sdkRoot, sdk, "Windows.winmd"),
  "/reference:" + path.join(framework, "System.Runtime.WindowsRuntime.dll"),
  "/reference:" + path.join(facadeRoot, facade, "System.Runtime.dll"),
  ...["WindowsBase", "PresentationCore", "PresentationFramework"].map((name) =>
    "/reference:" + path.join(framework, "WPF", name + ".dll")
  ),
  ...readdirSync(source).filter((name) => name.endsWith(".cs")).map((name) => path.join(source, name)),
  ...(testing
    ? readdirSync(path.join(source, "Tests")).filter((name) => name.endsWith(".cs")).map((name) =>
      path.join(source, "Tests", name)
    )
    : []),
], { stdio: "inherit", windowsHide: true });
if (release) rmSync(path.join(output, path.basename(executable, ".exe") + ".pdb"), { force: true });
