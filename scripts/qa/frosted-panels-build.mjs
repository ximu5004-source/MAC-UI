// Isolated CSS/interaction fixture. No desktop state is loaded or changed.
import esbuild from "esbuild";
import svelte from "esbuild-svelte";
import { sveltePreprocess } from "svelte-preprocess";
import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const output = "target/frosted-panels-qa";
await mkdir(`${output}/translations`, { recursive: true });
await esbuild.build({
  entryPoints: ["scripts/qa/frosted-panels/main.ts"], outdir: output,
  bundle: true, format: "esm", platform: "browser", target: "esnext", conditions: ["svelte"],
  plugins: [
    { name: "no-native-commands", setup(build) {
      build.onResolve({ filter: /^@seelen-ui\/lib$/ }, () => ({ path: resolve("scripts/qa/frosted-panels/native-mock.ts") }));
      build.onResolve({ filter: /^libs\/ui\/svelte\/components\/Icon$/ }, () => ({ path: resolve("scripts/qa/frosted-panels/icon-only.ts") }));
    } },
    svelte({ cache: false, preprocess: sveltePreprocess({ postcss: { plugins: [] } }) }),
  ],
});
const defaultStyles = await Promise.all([
  "src/static/themes/default/styles/quick-settings.scss",
  "src/static/themes/default/styles/flyouts.css",
].map(async (file) => (await readFile(file, "utf8")).replace(/^\s*\/\/.*$/gm, "")));
await writeFile(`${output}/theme.css`, defaultStyles.join("\n"));
await copyFile("scripts/qa/frosted-panels/index.html", `${output}/index.html`);
await copyFile("src/ui/svelte/quick-settings/i18n/translations/en.yml", `${output}/translations/en.yml`);
console.info("Run node scripts/qa/frosted-panels-server.mjs and open http://127.0.0.1:3584/index.html");
