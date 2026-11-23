# C# 桌面管理

桌面管理由 `native/DesktopOrganizer/MacUI.Desktop.csproj` 实现。程序使用 C#、.NET Framework 4.8 和 WPF，输出
`macui-desktop.exe`。桌面窗口、整理盒、文件操作、配置读写和 Explorer 图标恢复均在此进程中执行。

顶部栏、Dock、启动台及设置窗口继续由主程序提供。`src/background/widgets/native_host.rs` 负责启动和停止 C#
进程，并传递设置请求与运行状态。

## 模块

| 文件                                                 | 职责                                                          |
| ---------------------------------------------------- | ------------------------------------------------------------- |
| `EntryPoint.cs`                                      | STA 入口、单实例、父进程退出处理、消息接收                    |
| `DesktopSession.cs`                                  | 窗口生命周期、选择、拖放、剪贴板、文件监视                    |
| `DesktopWindow.cs`                                   | WPF 桌面窗口、整理盒、图标、键盘操作                          |
| `DesktopMenus.cs`                                    | 文件菜单、整理盒编辑及统一设置入口                            |
| `NativeGlass.cs`、`GlassControls.cs`、`GlassMenu.cs` | 原生 Composition 背景模糊、圆角材质、自动隐藏滚动条及玻璃菜单 |
| `DesktopLayout.cs`、`DesktopToolbar.cs`              | 网格与相邻整理盒吸附、可用屏幕区域、桌面控制条                |
| `DesktopModel.cs`                                    | 分类、排序、手动分组、常用文件、显示器布局                    |
| `JsonState.cs`                                       | 配置迁移、原子写入、布局备份、翻译                            |
| `ShellCatalog.cs`                                    | 用户与公共桌面、映射文件夹、Shell 图标                        |
| `FileOperations.cs`                                  | Windows `IFileOperation` 文件操作                             |
| `ShellMenu.cs`、`ShellSelection.cs`                  | Windows 文件菜单及跨文件夹多选                                |
| `ShellDragImage.cs`                                  | Shell 拖放数据对象、跟随指针的原生拖动图像及目标反馈          |
| `ShellTemplates.cs`                                  | Windows 注册的文件模板                                        |
| `MonitorIdentity.cs`                                 | 显示器稳定标识及显示配置                                      |
| `DesktopRecovery.cs`                                 | Explorer 图标可见性和独立恢复进程                             |

## 文件与权限

桌面进程使用 `asInvoker`，随普通权限的 MAC UI 主程序运行。文件复制、移动、重命名、创建和删除调用 Windows
`IFileOperation`，使用系统的冲突、权限和进度界面。

拖动真实文件时通过 `SHDoDragDrop` 提供 Windows `FileDrop` 数据，并用 `IDragSourceHelper`
显示跟随指针的图标。目标程序决定是否接收这些文件；向通讯软件拖入文件后，发送操作仍由用户完成。桌面内部的排序和整理盒分组保存在布局文件中。解散整理盒保留原文件；映射文件夹中的文件继续留在映射目录。

文件操作在专用 STA 线程执行。正常退出等待正在进行的文件操作结束。进程异常终止时，独立恢复进程恢复本次运行隐藏的 Explorer
图标。

## 配置

配置目录为 `%APPDATA%\com.jona.mac-ui\data\macui-desktop-shell`。

- `desktop-shell.json`：当前布局，保留 schema 3 与未识别字段。
- `desktop-shell.before-native-csharp.json`：首次读取已有配置时保存的迁移副本。
- `desktop-shell.backup.json`：上次成功写入前的配置。
- `desktop-shell.manual-backup.json`：用户手动保存的布局。
- `native-desktop-recovery.json`：当前桌面进程的恢复记录，正常恢复后移除。
- `native-desktop.log`：桌面进程错误日志。

原有显示器标识继续使用 Windows `StableMonitorId`。C# 窗口几何信息使用 DIP，并单独存放在
`nativeDesktop.layouts`。原有位置数据按对应显示器 DPI 转换读取。

MAC UI 设置的桌面页面提供整理盒玻璃透明度和磨砂强度，分别保存为 `glassTransparency` 与 `glassBlur`，范围为
0–100，默认值为 88 和 35。设置通过已有消息接口交给 C#
进程保存，并更新整理盒、桌面控制条及菜单材质。图标控件在材质变化和排序时保持复用。

图标大小、水平与垂直间距、整理盒边缘吸附、网格和布局锁定均在同一设置页面调整。桌面菜单中的设置入口打开 MAC UI
的桌面设置页。`snapBoxes` 控制整理盒边缘吸附，默认开启；拖动时在 12 DIP 范围内吸附相邻边缘，盒间间距为 12 DIP。 按住
Shift 可临时关闭吸附。整理盒按鼠标位置移动，松手后保留当前位置；新建整理盒选择邻近空位。
跨显示器拖动使用目标显示器的缩放比例与可用区域，折叠状态下保留展开高度。

整理盒使用显示器实际可用区域，并预留顶部栏位置。新建整理盒为空盒，映射文件夹由盒内菜单设置。桌面空白区域使用 Windows
桌面菜单，并附加 MAC UI 桌面子菜单。窄滚动条在滚动、拖动或进入操作区域时显示，操作结束约一秒后淡出。

滚动条覆盖在内容右侧，不占据图标布局宽度。桌面背景、整理盒和控制条保持在普通应用窗口下方，不显示在任务栏与 Alt+Tab
中。玻璃使用均匀的透明底色与原生背景模糊；按钮的键盘焦点使用圆角实线轮廓。

## 主程序通信

标准输入和标准输出传递逐行 UTF-8 JSON。主程序请求包括 `settings`、`controls`、`refresh` 和 `stop`；桌面回复包括
`ready`、`controls`、`changed`、`open-settings` 和 `error`。`controls` 使用请求 `id` 返回结果。`open-settings`
由主程序打开 `/desktop` 设置页。

父进程退出或输入管道关闭时，桌面进程请求正常退出。安装程序通过 `Local\macui.desktop.stop.<pid>`
事件请求桌面进程退出，并等待恢复进程结束。

## 构建与验证

开发环境需要 Windows 10 SDK、.NET Framework 4.8、Node.js 和项目依赖。

```powershell
node scripts/build/desktop-native.mjs
dotnet msbuild native/DesktopOrganizer/MacUI.Desktop.csproj /t:Build /p:Configuration=Debug
& ./scripts/tests/native-desktop.ps1
& ./scripts/tests/native-desktop-host.ps1 -Executable ./src/static/native/macui-desktop.exe -StopMethod Pipe
& ./scripts/tests/native-desktop-host.ps1 -Executable ./src/static/native/macui-desktop.exe -StopMethod EndOfInput
node --test scripts/qa/runtime-install-identity.test.mjs
```

原生交互检查包含连续拖动、跨整理盒拖动、拖入文件夹、拖入通讯软件但不发送、解散整理盒、正常退出和异常退出恢复。

`macui-native-tests.exe --recovery <macui-desktop.exe 的路径>` 使用临时文件和配置验证 Explorer
图标隐藏、正常退出恢复及进程异常终止恢复。执行前退出 MAC UI；测试结束后还原测试前的图标可见状态。

隔离预览使用独立文件目录和配置目录：

```powershell
./src/static/native/macui-desktop.exe --preview --root "E:\NativeDesktopTest\Files" --data "E:\NativeDesktopTest\Config" --locale zh-CN
```

隔离预览只显示指定目录，不接管 Explorer 图标。完整桌面测试需要先正常退出当前 MAC UI，防止同时运行两套桌面管理。

主程序集成构建顺序：先执行 `npm run build:ui -- --production`，完成后再执行
`cargo build -p macui --bin macui --features custom-protocol`。

整理独立测试目录时，执行
`./scripts/build/stage-debug.ps1 -Destination <新目录>`。该脚本按当前构建的资源清单复制文件并核对完整性，主程序启动时继续执行正常的资源校验。

## 回退

修改前的源码、Git 历史和用户配置备份位于
`E:\换皮\backups\MAC-UI-before-native-desktop-20261122`。先正常退出测试程序，再恢复该目录中的配置备份，运行已保留的
4.2.0 安装包。源码可从 `source-4.2.0.zip` 解压到独立目录，或从 `repository.bundle` 恢复 Git 仓库。

保留项目根目录的许可证、版权和第三方署名文件。
