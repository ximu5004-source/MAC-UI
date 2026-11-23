import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import yaml from "js-yaml";

export function buildDesktopLocales(output) {
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const locales = {};
  function flatten(value, prefix = "", result = {}) {
    for (const [key, item] of Object.entries(value)) {
      const name = prefix ? `${prefix}.${key}` : key;
      if (item && typeof item === "object") flatten(item, name, result);
      else result[name] = String(item);
    }
    return result;
  }
  const translations = path.join(root, "native/DesktopOrganizer/Translations");
  for (const name of readdirSync(translations).filter((name) => name.endsWith(".yml"))) {
    locales[path.basename(name, ".yml")] = flatten(yaml.load(readFileSync(path.join(translations, name), "utf8")));
  }
  const native = path.join(root, "native/DesktopOrganizer/Locales");
  for (const name of readdirSync(native).filter((name) => name.endsWith(".json"))) {
    const locale = path.basename(name, ".json");
    locales[locale] = {
      ...locales.en,
      ...locales[locale],
      ...JSON.parse(readFileSync(path.join(native, name), "utf8")),
    };
  }
  mkdirSync(output, { recursive: true });
  writeFileSync(path.join(output, "desktop-locales.json"), JSON.stringify(locales), "utf8");
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (!process.argv[2]) throw new Error("Specify the native desktop output directory.");
  buildDesktopLocales(path.resolve(process.argv[2]));
}
