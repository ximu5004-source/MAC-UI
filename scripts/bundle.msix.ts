import { execSync } from "child_process";
import fs from "fs";
import os from "os";
import path from "path";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";

import packageJson from "../package.json";
import process from "node:process";

async function getArgs() {
  const argv = await yargs(hideBin(process.argv)).option("target", {
    type: "string",
    description: "target to get the files from",
    default: "release",
  }).argv;
  return {
    target: argv.target,
  };
}

const [major, minor, patch, pre, _build_number] = packageJson.version.split(/[\.\+\-]/);
if (major === undefined || minor === undefined || patch === undefined) {
  throw new Error("Invalid package version");
}

const { target } = await getArgs();
// MSIX identity must belong to the MAC UI publisher; never reuse an upstream certificate.
const publisher = process.env.MAC_UI_MSIX_PUBLISHER;
if (!publisher || !publisher.startsWith("CN=")) {
  throw new Error("Set MAC_UI_MSIX_PUBLISHER to your own certificate subject. NSIS remains the supported distribution.");
}
if (!["release", "x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"].includes(target)) {
  throw new Error("Unsupported build target");
}

// Determine architecture for MSIX naming
const archMap: Record<string, string> = {
  "x86_64-pc-windows-msvc": "x64",
  "aarch64-pc-windows-msvc": "arm64",
};
const arch = archMap[target] || "x64";

const outDir = target === "release" ? "target/release" : `target/${target}/release`;

console.info(`Building MSIX for ${arch}...`);
const buildFolder = `${outDir}/msix`;
const bundleFolder = `${outDir}/bundle/msix`;

fs.rmSync(buildFolder, { recursive: true, force: true }); // clean up
fs.mkdirSync(buildFolder, { recursive: true });
fs.mkdirSync(bundleFolder, { recursive: true });

// we skip revision here because greater numbers than 65535 are not supported on msix
const appxPackageVersion = `${major}.${minor}.${patch}.0`;
const fileVersion = pre ? packageJson.version : appxPackageVersion;
const installer_msix_path = path.resolve(`${bundleFolder}/MAC-UI_${fileVersion}_${arch}.msix`);

// Add manifest
const manifest = fs
  .readFileSync("src/templates/AppxManifest.xml", "utf-8")
  .replace("{{version}}", appxPackageVersion)
  .replace("{{architecture}}", arch)
  .replace("{{publisher}}", publisher.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;").replaceAll(">", "&gt;"));
fs.writeFileSync(`${buildFolder}/AppxManifest.xml`, manifest);

// Add binaries
fs.copyFileSync(`${outDir}/macui.exe`, `${buildFolder}/macui.exe`);
fs.copyFileSync(`${outDir}/macui-service.exe`, `${buildFolder}/macui-service.exe`);
fs.copyFileSync(`${outDir}/macui-cli.exe`, `${buildFolder}/macui-cli.exe`);

// Optional symbols belong only in a pre-release package.
if (pre && fs.existsSync(`${outDir}/macui.pdb`)) {
  fs.copyFileSync(`${outDir}/macui.pdb`, `${buildFolder}/macui.pdb`);
}

// dlls
fs.copyFileSync(`${outDir}/macui_hook.dll`, `${buildFolder}/macui-hook.dll`);

// integrity files
fs.copyFileSync(`${outDir}/SHA256SUMS`, `${buildFolder}/SHA256SUMS`);
fs.copyFileSync(`${outDir}/SHA256SUMS.sig`, `${buildFolder}/SHA256SUMS.sig`);

// Preserve the license and attribution in every distribution format.
fs.copyFileSync("LICENSE", `${buildFolder}/LICENSE`);
fs.copyFileSync("NOTICE.md", `${buildFolder}/NOTICE.md`);

// Add resources
fs.cpSync("src/static", `${buildFolder}/static`, { recursive: true });

if (os.platform() !== "win32") {
  throw new Error("MSIX bundling is only supported on Windows");
}

const sdkBinRoot = "C:\\Program Files (x86)\\Windows Kits\\10\\bin";
const sdkVersion = fs
  .readdirSync(sdkBinRoot)
  .filter((d) => /^\d+\.\d+\.\d+\.\d+$/.test(d))
  .sort((a, b) => {
    const toNum = (v: string) =>
      v
        .split(".")
        .map(Number)
        .reduce((acc, n) => acc * 100000 + n, 0);
    return toNum(b) - toNum(a);
  })[0];

if (!sdkVersion) {
  throw new Error(
    "Windows SDK not found. Install it from https://developer.microsoft.com/windows/downloads/windows-sdk/",
  );
}

const sdkBin = path.join(sdkBinRoot, sdkVersion, "x64");
const makeappx = path.join(sdkBin, "makeappx.exe");

console.info(`Using Windows SDK ${sdkVersion} at ${sdkBin}`);

try {
  // create installer bundle
  let out = execSync(`"${makeappx}" pack /d "${buildFolder}" /p "${installer_msix_path}" /nv /o`);
  console.info(out.toString());

  console.info("Unsigned MAC UI MSIX created. Sign separately with your own matching certificate; no certificate was loaded.");
} catch (error) {
  console.error("\n\n", error?.toString());
  process.exit(1);
}
