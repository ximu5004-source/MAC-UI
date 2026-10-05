// Real icon component, Dock layout and theme. No user data, IPC or Windows calls.
import esbuild from "esbuild";
import sveltePlugin from "esbuild-svelte";
import { readFile, writeFile, mkdir, copyFile } from "node:fs/promises";
const out = "dist/qa/dock-size";
await mkdir(out, { recursive: true });
await esbuild.build({ entryPoints: ["scripts/qa/dock-size/entry.ts"], outdir: out, bundle: true, format: "esm", target: "esnext", plugins: [sveltePlugin({ compilerOptions: { dev: true } })] });
let theme = await readFile("src/static/themes/default/styles/weg.scss", "utf8");
// The fade mixin only handles mounting; retain all sizing/nesting rules as-is.
const start = theme.indexOf("@mixin fade");
let cursor = theme.indexOf("{", start), depth = 1;
for (++cursor; cursor < theme.length && depth; cursor++) {
  if (theme[cursor] === "{") depth++;
  else if (theme[cursor] === "}") depth--;
}
theme = theme.slice(0, start) + theme.slice(cursor);
theme = theme.replaceAll("@include fade;", "").replace(/^\s*\/\/.*$/gm, "");
theme += await readFile("src/static/themes/liquid-glass/styles/weg.scss", "utf8");
await writeFile(out + "/theme.css", theme);
await copyFile("scripts/qa/dock-size/index.html", out + "/index.html");
console.log("Dock size QA: http://localhost:3579/qa/dock-size/index.html");
