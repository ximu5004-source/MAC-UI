# 通讯客户端通知 / Messaging client notifications

## 中文

MAC UI 可将微信、企业微信、钉钉、飞书（Lark）客户端发布的通知接入顶栏通知中心与通知弹窗。内容来自三种公开接口：现有
Windows Toast 监听与快照轮询、托盘 `NIF_INFO`、可访问性接口 UIA / MSAA 暴露的可见通知弹窗。

- **真实内容**：Windows Toast 保留系统通知与原有操作；传统客户端的 `NIF_INFO`
  标题、正文及可访问通知弹窗文本由本地通知桥接显示。只有客户端实际公开的内容才会被展示。
- **未读提示**：如果客户端只在托盘提示文字中明确报告“未读”或“新消息”，MAC UI
  显示“有未读消息，请打开应用查看”，不猜测聊天正文或未读数量。微信或企业微信的托盘图像在 6 秒内连续变更三次时，通知中心新增一条可点击的“托盘图标正在闪烁”提醒，并按通知弹窗和勿扰设置显示卡片。一次连续闪烁只提醒一次，正文为空。
- **打开客户端**：点击桥接条目优先调用客户端的原生托盘动作。无托盘的 MSAA 通知使用采集时的进程 ID
  和完整可执行路径寻找、恢复并激活客户端主窗口；客户端已退出时返回失败。不能保证直接定位到具体会话。
- **提示行为**：弹窗遵守 MAC UI
  的通知弹窗开关与系统勿扰状态。启动时建立已有托盘、已有弹窗基线，不把旧状态当作新消息。相同应用、近时间、相同文本的跨接口通知去重；系统
  Toast 保留完整操作。单个通知弹窗持续显示相同内容时只采集一次，隐藏再出现或内容变化可触发新记录。
- **隐私与生命周期**：桥接记录仅保存在当前进程内存中，最多 100 条。关闭或清空桥接提示只移除 MAC UI
  条目，不标记聊天为已读；不读取聊天数据库、进程内存，不发送消息，不自动更改系统隐私权限。UIA / MSAA 采集在独立、无窗口的 MTA
  线程进行，只处理识别到的客户端可见通知弹窗，跳过不可见、屏幕外和非交互会话状态。通知收集器释放时移除 HookManager
  订阅和停止调度。

兼容范围是条件支持，不代表四款客户端的每个版本都能获得正文：

| 客户端      | 可用来源                              | 专用 MSAA 规则                                                                            | 未提供正文时                            |
| ----------- | ------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------- |
| 微信        | 系统 Toast、NIF_INFO、MSAA popup      | `WeChat.exe` / `Weixin.exe` / `WeChatStore.exe`；`TrayNotifyWnd` 中第一个可见静态文本角色 | 仅在明确未读 tooltip 存在时显示未读提示 |
| 企业微信    | 系统 Toast、NIF_INFO、通用 UIA / MSAA | `WXWork.exe` / `WeCom.exe`；要求公开 Alert / LiveRegion，没有臆造专用窗口类                  | 同上，不能从主聊天窗补取正文            |
| 钉钉        | 系统 Toast、NIF_INFO、MSAA popup      | `DingTalk.exe`；`DingtalkMsgComming` 中可见列表项文本                                     | 同上                                    |
| 飞书 / Lark | 系统 Toast、NIF_INFO、通用 UIA / MSAA | `Feishu.exe` / `Lark.exe`；要求公开 Alert / LiveRegion，没有臆造专用窗口类                   | 同上，无法获取内容时不会制造消息卡片    |

通用 MSAA 规则要求顶层窗口具有 `WS_POPUP`，同时具有 `WS_EX_NOACTIVATE` 或 `WS_EX_TOOLWINDOW`，并且在可访问树的
`ROLE_SYSTEM_ALERT` 子树中存在可见静态文本或列表项。UIA 回退保留同一窗口准入限制，并要求 `aria-role=alert/status` 或
`LiveSetting=Polite/Assertive` 区域中的可见 Text / ListItem；不采集 Edit、Document、Menu 子树。普通主窗口不进入采集。遍历限制为最多 64 个 popup、每个树 256
个节点、12
层深度，避免异常可访问树无限递归。UIA 连接和事务超时各为 250ms，单树遍历预算 350ms；它们不保证能取消任意失效的 MSAA 提供程序。
客户端完全自绘且不公开这些接口、关闭消息内容预览、应用权限隔离或系统通知访问未授权时，正文可能不可用。

Windows Toast 每 500ms 复核一次实际系统快照，避免仅依赖在非打包程序中可能失败或漏报的 `NotificationChanged`。
`NotificationModeChanged` 注册失败不会中断通知分发；模式变化同时由轮询补充，勿扰仍有效。通知内容临时读取失败会保留后续重试机会。

微信、钉钉的专用规则依据 MyDockFinder 5.9.9.92 静态分析：适配函数 `0x140325bc0`、MSAA 入口 `0x140177820`、角色递归
`0x140177a60`。样本仅有企业微信默认规则
`wxwork.exe`，没有证明企业微信或飞书的专用内容读取器，因此这里对两者使用通用公开接口。实现是依据行为重新编写的本地模块。

## English

MAC UI integrates client-published WeChat, WeCom, DingTalk and Feishu/Lark notifications into the top-bar notification
center and notification flyouts.

- **Actual content**: Existing Windows Toasts retain their native actions. The bridge also accepts real `NIF_INFO`
  title/body and visible notification-popup text exposed through UIA/MSAA. It never reconstructs private chat contents.
- **Unread hints**: An explicit unread/new-message tray tooltip produces a localized “unread messages; open the app to
  view” prompt, without inventing message text or a count. For WeChat and WeCom, three tray image transitions within six seconds generate a single content-free activity card per uninterrupted burst. The card follows the notification flyout and Do Not Disturb settings.
- **Activation**: Clicking a bridged entry uses the client's tray action when available. A popup without a tray icon can
  activate a real main window matching its captured process ID and full executable path. Opening the exact conversation
  is not guaranteed; an exited client produces an activation failure.
- **Display behavior**: Flyouts respect MAC UI's notification setting and system Do Not Disturb. Startup baselines
  existing tray and popup state. Matching app/text/time across sources is deduplicated; a popup with unchanged text is
  not repeatedly collected.
- **Privacy and lifetime**: Bridged records are in-memory only, bounded to 100 entries. Dismiss/clear removes MAC UI
  entries without marking chats as read. The integration does not read chat databases or process memory, send messages,
  or automatically change privacy permissions.

WeChat's `TrayNotifyWnd` static text and DingTalk's `DingtalkMsgComming` list items use dedicated rules evidenced in the
analyzed sample. WeCom and Feishu have no evidenced dedicated adapter: their generic accessibility path requires a transient
native popup and an MSAA Alert or explicit UIA Alert/Status/LiveRegion containing visible text/list items. UIA runs
on a separate windowless MTA with bounded provider timeouts and tree traversal; editable document/menu trees are excluded. The integration never scans a main chat
window. Versions that expose no Toast, NIF_INFO or accessible popup text cannot supply message bodies. Explicit unread
tooltips can still produce a content-free unread hint. Client-side preview privacy settings remain effective.

System Toast snapshots are reconciled every 500ms even when WinRT accepts event registration. A failure to subscribe
to quiet-mode events no longer prevents the notification dispatcher from receiving polling results. Transient
content failures remain eligible for retry. These are delivery-path fixes, not proof of compatibility with every
current WeChat/WeCom popup version; real-client acceptance still requires an actual newly received notification.

## Focused validation

Automated tests cover exact executable identities, dedicated/generic popup admission, startup baselines, changed text,
cross-source duplicate arrival orders, optional tray activation targets, bounded history, and all four apps in Toast
merging and flyout suppression.

For a local runtime check, enable a client's notification preview, receive a newly generated notification, and verify
the displayed text matches that popup or Windows Toast. Repeat with preview disabled, Do Not Disturb, a minimized
client, and after exiting the client. Test WeCom/Feishu both with system Toast enabled and disabled: an inaccessible
proprietary popup must not be reported as captured content. Restart MAC UI with an existing notification visible to
verify there is no startup replay. Native-client runtime coverage depends on the installed version and is separate from
automated tests.

## Windows API references

- [Notification listener](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/notification-listener)
- [NOTIFYICONDATAW and NIF_INFO](https://learn.microsoft.com/windows/win32/api/shellapi/ns-shellapi-notifyicondataw)
- [AccessibleObjectFromWindow](https://learn.microsoft.com/windows/win32/api/oleacc/nf-oleacc-accessibleobjectfromwindow)
- [AccessibleChildren](https://learn.microsoft.com/windows/win32/api/oleacc/nf-oleacc-accessiblechildren)
- [MSAA object roles](https://learn.microsoft.com/windows/win32/winauto/object-roles)
- [UI Automation threading](https://learn.microsoft.com/windows/win32/winauto/uiauto-threading)
- [UIA connection timeout](https://learn.microsoft.com/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomation2-put_connectiontimeout)
