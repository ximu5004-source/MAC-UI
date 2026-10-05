import { strict as assert } from "node:assert";
import { test } from "node:test";
import { communicationApp, hasUnreadHint } from "./communication.ts";

test("messaging identity distinguishes QQ browser and enterprise WeChat", () => {
  assert.equal(communicationApp("C:\\Apps\\QQ\\QQ.exe"), "QQ");
  assert.equal(communicationApp("C:\\Apps\\QQBrowser.exe"), null);
  assert.equal(communicationApp("企业微信 新消息"), "WeCom");
  assert.equal(communicationApp("C:\\Apps\\Weixin.exe"), "WeChat");
  assert.equal(communicationApp("Windows Security"), null);
});

test("zero unread and an ordinary running tooltip are not notifications", () => {
  assert.equal(hasUnreadHint("Telegram (0 unread)"), false);
  assert.equal(hasUnreadHint("微信 未读：0"), false);
  assert.equal(hasUnreadHint("微信"), false);
  assert.equal(hasUnreadHint("微信 有新消息"), true);
  assert.equal(hasUnreadHint("Telegram (2 unread)"), true);
});
