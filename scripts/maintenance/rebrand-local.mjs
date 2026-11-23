// Mechanical copy edit of translated product wording; identifiers and legal notices stay intact.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
const roots = [
  "src/ui",
  "src/background/i18n",
  "src/static/widgets",
  "src/static/plugins",
  "src/static/themes/default/i18n",
];
let changed = 0;
function walk(dir) {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const item of entries) {
    const path = join(dir, item.name);
    if (item.isDirectory()) {
      walk(path);
      continue;
    }
    if (!path.endsWith(".yml") || !/[\\/](i18n|translations)[\\/]/.test(path)) continue;
    const original = readFileSync(path, "utf8");
    let next = original.split(/\r?\n/).map((line) => {
      if (/^\s*(attribution|license|copyright|upstream):/.test(line)) return line;
      return line.replace(/MacUi UI/gi, "MAC UI").replace(/\bMacUi\b/g, "MAC UI")
        .replace(/精美工具栏|花式工具栏/g, "顶部栏").replace(/精美工具欄|花式工具欄/g, "頂部列")
        .replace(/Fancy Toolbar/gi, "Top Bar").replace(/停靠栏\/任务栏|停靠栏 \/ 任务栏/g, "Dock 栏");
    }).join("\n");
    if (next !== original.replace(/\r\n/g, "\n")) {
      writeFileSync(path, next);
      changed++;
    }
  }
}
roots.forEach(walk);
console.log("Updated translated product wording in " + changed + " files.");
