// Isolated output: never shipped in dist or the installer.
import esbuild from "esbuild";
import svelte from "esbuild-svelte";
import { sveltePreprocess } from "svelte-preprocess";
import { copyFile, readFile, writeFile } from "node:fs/promises";
const output = "target/icon-edges-qa";
await esbuild.build({
  entryPoints: ["scripts/qa/icon-edges/main.ts"], outdir: output,
  bundle: true, format: "esm", platform: "browser", target: "esnext", conditions: ["svelte"],
  plugins: [svelte({ cache: false, preprocess: sveltePreprocess({ postcss: { plugins: [] } }) })],
});
const theme = await Promise.all([
  "src/static/themes/liquid-glass/shared/index.scss",
  "src/static/themes/liquid-glass/styles/weg.scss",
].map(async file => (await readFile(file, "utf8")).replace(/^\s*\/\/.*$/gm, "")));
await writeFile(`${output}/theme.css`, `@layer theme { ${theme.join("\n")} }`);
await copyFile("scripts/qa/icon-edges/index.html", `${output}/index.html`);
console.info("node scripts/qa/launchpad-server.mjs target/icon-edges-qa 3583 → /index.html");
