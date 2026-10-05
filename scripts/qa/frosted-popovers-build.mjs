// Real popup entries, isolated fake native bridge served by mac-panels-server.
// No user settings, Windows actions, or production bundles are changed.
import esbuild from "esbuild";
import svelte from "esbuild-svelte";
import { sveltePreprocess } from "svelte-preprocess";
import { mkdir, copyFile } from "node:fs/promises";

const panels = ["keyboard-selector", "network-popup", "bluetooth-popup", "notifications", "system-tray"];
const output = "target/frosted-popovers-qa";
await esbuild.build({
  entryPoints: panels.map((panel) => `src/ui/svelte/${panel}/index.ts`),
  outdir: output, outbase: "src/ui", bundle: true, format: "esm",
  platform: "browser", target: "esnext", conditions: ["svelte"], loader: { ".yml": "text" },
  plugins: [svelte({ cache: false, preprocess: sveltePreprocess({ postcss: { plugins: [] } }) })],
});
for (const panel of panels) {
  await mkdir(`${output}/svelte/${panel}/translations`, { recursive: true });
  for (const locale of ["en", "zh-CN"]) {
    await copyFile(`src/ui/svelte/${panel}/i18n/translations/${locale}.yml`, `${output}/svelte/${panel}/translations/${locale}.yml`);
  }
}
console.info(`Run node scripts/qa/mac-panels-server.mjs ${output} 3585 ${panels.join(",")}`);
