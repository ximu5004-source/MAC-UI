// Real settings controls and material CSS; no Windows commands or user settings.
import esbuild from "esbuild";
import CssModulesPlugin from "esbuild-css-modules-plugin";
import { resolve } from "node:path";
import { copyFile, mkdir } from "node:fs/promises";
const qa = resolve("scripts/qa/glass-appearance");
const out = resolve("dist/qa/glass-appearance");
await esbuild.build({
  entryPoints: [qa + "/preview.tsx"], outfile: out + "/preview.js",
  bundle: true, format: "esm", target: "esnext", jsx: "automatic", loader: { ".yml": "text" },
  alias: { react: resolve("node_modules/preact/compat"), "react/jsx-runtime": resolve("node_modules/preact/jsx-runtime"), "react-dom": resolve("node_modules/preact/compat") },
  plugins: [
    { name: "isolate-settings", setup(build) {
      build.onResolve({ filter: /^(\.\/application|\.\/seelenweg\/application)$/ }, args => args.importer.replaceAll("\\", "/").endsWith("Widget/DesktopGlass.tsx") ? { path: qa + "/mock.ts" } : undefined);
    } },
    CssModulesPlugin({ localsConvention: "camelCase", pattern: "qa-[local]-[hash]", targets: {} }),
  ],
});
await mkdir(out, { recursive: true });
await copyFile(qa + "/index.html", out + "/index.html");
console.info("Glass settings QA: http://localhost:3579/qa/glass-appearance/index.html");
