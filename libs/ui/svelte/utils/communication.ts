/** Only classify known messaging clients. Never infer unread counts from icon changes. */
export function communicationApp(value: string): string | null {
  const name = value.toLowerCase();
  const apps: [RegExp, string][] = [
    [/wechat|weixin|微信/, "WeChat"],
    [/wxwork|wecom|企业微信/, "WeCom"],
    [/(?:^|[\\/\s])qq(?:\.exe|\s|$)|腾讯qq/, "QQ"],
    [/dingtalk|钉钉/, "DingTalk"],
    [/feishu|lark|飞书/, "Feishu"],
    [/teams/, "Teams"],
    [/telegram/, "Telegram"],
    [/whatsapp/, "WhatsApp"],
    [/discord/, "Discord"],
    [/slack/, "Slack"],
    [/(?:^|[\\/\s])signal(?:\.exe|\s|$)/, "Signal"],
    [/skype/, "Skype"],
  ];
  // Enterprise WeChat must win over the more general Chinese WeChat name.
  if (/wxwork|wecom|企业微信/.test(name)) return "WeCom";
  return apps.find(([pattern]) => pattern.test(name))?.[1] ?? null;
}

export function hasUnreadHint(tooltip: string): boolean {
  if (/\b0\s*(?:unread|new messages?)|(?:未读|新消息)\s*[:：]?\s*0(?:\D|$)/i.test(tooltip)) return false;
  return /未读|新消息|新信息|\bunread\b|\bnew messages?\b/i.test(tooltip);
}
