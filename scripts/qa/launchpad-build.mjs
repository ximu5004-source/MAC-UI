// Build only Launchpad into an isolated target directory, without touching a running dist.
import esbuild from "esbuild";
import svelte from "esbuild-svelte";
import { sveltePreprocess } from "svelte-preprocess";
import { cp } from "node:fs/promises";
await esbuild.build({
  entryPoints: ["src/ui/svelte/apps-menu/index.ts"],
  bundle: true, format: "esm", platform: "browser", target: "esnext",
  outdir: "target/launchpad-motion-qa", outbase: "src/ui",
  conditions: ["svelte"], loader: { ".yml": "text" },
  plugins: [svelte({ cache: false, preprocess: sveltePreprocess({ postcss: { plugins: [] } }) })],
});
await cp("src/ui/svelte/apps-menu/i18n/translations", "target/launchpad-motion-qa/svelte/apps-menu/translations", { recursive: true });
