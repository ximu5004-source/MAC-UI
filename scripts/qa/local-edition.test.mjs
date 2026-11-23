import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const read = (file) => readFileSync(new URL(`../../${file}`, import.meta.url), "utf8");

test("retired upstream account, backup, updater and presence managers cannot make requests", () => {
  for (
    const file of [
      "src/background/session/application.rs",
      "src/background/backups/application.rs",
      "src/background/utils/updater.rs",
      "src/background/utils/discord.rs",
      "src/background/cli/uri/mod.rs",
      "src/ui/react/settings/state/resources.ts",
    ]
  ) {
    assert.doesNotMatch(read(file), /reqwest|fetch\s*\(|https?:\/\/(?:api\.)?macui\./i, file);
  }
  assert.match(read("src/background/session/application.rs"), /Online accounts are not available/);
  assert.match(read("src/background/utils/updater.rs"), /Ok\(None\)/);
  assert.doesNotMatch(read("src/background/main.rs"), /mod telemetry|on_startup\(\)/);
});

test("MAC UI owns its app identity without deleting license metadata", () => {
  const conf = JSON.parse(read("src/tauri.conf.json"));
  assert.equal(conf.identifier, "com.jona.mac-ui");
  assert.equal(conf.productName, "MAC UI");
  assert.match(conf.bundle.copyright, /AGPL/);
  assert.ok(read("libs/macui-ipc/src/app.rs").includes(String.raw`\pipe\mac-ui-`));
  assert.match(read("src/background/main.rs"), /migrate_legacy_data/);
  assert.match(read("NOTICE.md"), /Seelen UI/);
});

test("local API documentation is self-contained and does not load CDN code", () => {
  const html = read("src/background/server/http/scalar.html");
  assert.doesNotMatch(html, /https?:\/\//);
  assert.match(html, /\/api-doc\/openapi\.json/);
});
