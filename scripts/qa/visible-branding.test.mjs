import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { load } from "js-yaml";
import test from "node:test";

const root = fileURLToPath(new URL("../../", import.meta.url));
const roots = ["src/background/i18n", "src/ui/react/settings/i18n"];
const retired = /(?<![\p{L}\p{N}_])(?:seelen(?:[- ]ui)?|slu|sluhk|macui-slu)(?![\p{L}\p{N}_])|SLU(?=服務|服务)|\.slu\b|MacUi Corp/iu;

function* files(directory) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) yield* files(file);
    else if (/\.ya?ml$/.test(entry.name)) yield file;
  }
}

function inspect(value, keys, file) {
  if (typeof value === "string") {
    if (!keys.some((key) => /^(attribution|license|copyright|upstream)$/.test(key))) {
      assert.doesNotMatch(value, retired, `${file} / ${keys.join(".")}`);
    }
  } else if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) inspect(child, [...keys, key], file);
  }
}

test("all settings and native dialog translations parse and use MAC UI display names", () => {
  let count = 0;
  for (const directory of roots) {
    for (const file of files(path.join(root, directory))) {
      inspect(load(readFileSync(file, "utf8")), [], file);
      count++;
    }
  }
  assert.ok(count >= 75, "The full multilingual translation directories must be checked");
});

test("resource import instructions advertise the current .maui format", () => {
  for (const language of ["zh-CN", "zh-TW", "en"]) {
    const file = path.join(root, "src/ui/react/settings/i18n/translations", `${language}.yml`);
    const translations = load(readFileSync(file, "utf8"));
    assert.match(translations.mac_local.resources_hint, /\.maui\b/);
  }
});

test("the original copyright remains distinct from the new CLI wordmark", () => {
  const en = load(readFileSync(path.join(root, "src/ui/react/settings/i18n/translations/en.yml"), "utf8"));
  assert.match(en.mac_ui.attribution, /Original Seelen UI copyrights/);
  const art = readFileSync(path.join(root, "src/macui-cli/art.rs"), "utf8");
  assert.match(art, /MAC UI \/ JONA/);
  assert.doesNotMatch(art, retired);
  assert.doesNotMatch(art, /MacUi Corp/);
});
