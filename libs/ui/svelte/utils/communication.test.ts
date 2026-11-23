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
  assert.equal(hasUnreadHint("微信 0 条未读消息"), false);
  assert.equal(hasUnreadHint("微信 暂无新消息"), false);
  assert.equal(hasUnreadHint("WeCom no unread messages"), false);
  assert.equal(hasUnreadHint("微信 10 条未读消息"), true);
  assert.equal(hasUnreadHint("微信"), false);
  assert.equal(hasUnreadHint("微信 有新消息"), true);
  assert.equal(hasUnreadHint("Telegram (2 unread)"), true);
});

test("DingTalk and Feishu labels are matched without treating arbitrary lark substrings as Feishu", () => {
  assert.equal(communicationApp("C:\\Apps\\DingTalk.exe"), "DingTalk");
  assert.equal(communicationApp("钉钉 新消息"), "DingTalk");
  assert.equal(communicationApp("C:\\Apps\\Feishu.exe"), "Feishu");
  assert.equal(communicationApp("Lark new message"), "Feishu");
  assert.equal(communicationApp("飞书 新消息"), "Feishu");
  assert.equal(communicationApp("ScalarKeyFrameAnimation"), null);
});
