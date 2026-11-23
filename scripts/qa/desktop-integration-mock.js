// Browser-only fixture. Every native command is intercepted; no real files or messages are accessed.
(() => {
  const params = new URLSearchParams(location.search);
  const panel = location.pathname.split("/")[2];
  const callbacks = new Map();
  const events = new Map();
  const dragListeners = new Set();
  let modifiers = [false, false, false];
  let lastPoint = { x: 0, y: 0 };
  let pendingDrag = null;
  let nativeMenuResolve = null;
  let callbackId = 0;
  const storeKey = params.get("case") ? `macui-desktop-correction-${params.get("case")}` : params.has("fresh") ? "macui-desktop-integration-fresh-v3" : "macui-desktop-integration-fixture-v2";
  const files = new Map(Object.entries(JSON.parse(localStorage.getItem(storeKey) || "{}")));
  const entry = (name, folder = false, parent = "C:\\Fixture\\Desktop") => ({
    name, path: `${parent}\\${name}`, isDirectory: folder,
    isApplication: name.endsWith(".exe"), extension: folder ? null : name.split(".").at(-1),
    size: folder ? null : name.length * 1024, modifiedAt: Date.now() - name.length * 3600000,
    typeName: folder ? "文件夹" : name.endsWith(".txt") ? "文本文档" : `${name.split(".").at(-1).toUpperCase()} 文件`,
    isDesktopEntry: parent === "C:\\Fixture\\Desktop",
  });
  const entryState = JSON.parse(localStorage.getItem(`${storeKey}:entries`) || "null");
  const entries = entryState?.desktop || [entry("项目资料", true), entry("需求说明.pdf"), entry("工作记录.txt"), entry("示意图.png"), entry("工具.exe"), entry("示例应用.lnk"), { ...entry("早期记录.txt"), modifiedAt: Date.now() - 20 * 86400000 }];
  if (params.has("namespace") && !entries.some((item) => item.path.startsWith("::{"))) entries.push({ ...entry("此电脑", true), path: "::{20D04FE0-3AEA-1069-A2D8-08002B30309D}", size: null, modifiedAt: null, typeName: "系统文件夹" });
  const mappings = new Map(Object.entries(entryState?.mappings || {}));
  const persistEntries = () => localStorage.setItem(`${storeKey}:entries`, JSON.stringify({ desktop: entries, mappings: Object.fromEntries(mappings) }));
  const folderEntries = (path) => {
    if (!mappings.has(path)) mappings.set(path, [entry("映射文件.md", false, path), entry("子目录", true, path)]);
    return mappings.get(path);
  };
  const allEntries = () => [...entries, ...[...mappings.values()].flat()];
  const diskList = (path) => path === "C:\\Fixture\\Desktop" ? entries : folderEntries(path);
  const removeEntry = (path) => {
    for (const list of [entries, ...mappings.values()]) {
      const index = list.findIndex((item) => item.path.toLowerCase() === path.toLowerCase());
      if (index >= 0) list.splice(index, 1);
    }
  };
  const uniqueName = (list, name) => {
    const dot = name.lastIndexOf("."), stem = dot > 0 ? name.slice(0, dot) : name, extension = dot > 0 ? name.slice(dot) : "";
    let result = name, counter = 2;
    while (list.some((item) => item.name.toLowerCase() === result.toLowerCase())) result = `${stem} (${counter++})${extension}`;
    return result;
  };
  const emitDrag = async (payload) => { for (const handler of dragListeners) await handler({ payload }); };
  const fixtureIcon = (name) => ({ base: `${["file", "folder", "pdf", "txt", "png", "exe"].includes(name) ? name : "file"}.svg`, light: null, dark: null, mask: null, isAproximatelySquare: false });
  let notifications = ["WeChat", "WeCom", "DingTalk", "Feishu"].map((app, index) => ({
    id: `fixture-${app}`, app, source: index === 1 ? "UnreadHint" : "AccessiblePopup",
    trayId: null, iconPath: null, date: Date.now() - index * 60000,
    title: index === 1 ? "" : "隔离测试通知", body: index === 1 ? "" : `${app} 提示窗暴露的虚构内容。`,
  }));
  const record = (command, args) => {
    document.getElementById("fixture-status").textContent = `${command} ${JSON.stringify(args)}`;
  };
  const emit = (name, payload) => {
    for (const id of events.get(name) || []) callbacks.get(id)?.({ event: name, payload, id });
  };
  const noop = async () => {};
  const widget = {
    id: `@macui/${panel}`, settings: [], decoded: {}, windowId: 100,
    init: noop, ready: noop, focus: noop, hide: noop, onTrigger: () => noop,
    window: { setResizable: noop, onFocusChanged: async () => noop,
      onDragDropEvent: async (handler) => { dragListeners.add(handler); return () => dragListeners.delete(handler); } },
    webview: { listen: async (name, handler) => {
      callbacks.set(++callbackId, handler);
      const id = callbackId;
      events.set(name, [...(events.get(name) || []), id]);
      return () => { callbacks.delete(id); };
    } },
  };
  function showMenu(menu, onSelect) {
    document.getElementById("fixture-menu")?.remove();
    const element = document.createElement("div");
    element.id = "fixture-menu";
    element.setAttribute("role", "menu");
    const items = (list, depth = 0) => {
      for (const item of list) {
        if (item.type === "Submenu") { items(item.items, depth + 1); continue; }
        if (item.type !== "Item") continue;
        const button = document.createElement("button");
        button.textContent = `${"· ".repeat(depth)}${item.label}`;
        button.disabled = item.disabled;
        button.onclick = () => {
          element.remove();
          if (onSelect) onSelect(item.key);
          else emit(item.callbackEvent, { key: item.key, meta: menu.meta, checked: !item.checked });
        };
        element.append(button);
      }
    };
    items(menu.items);
    const close = document.createElement("button");
    close.textContent = "关闭测试菜单";
    close.onclick = () => { element.remove(); onSelect?.(null); };
    element.append(close);
    document.body.append(element);
  }
  window.__MACUI_WIDGET = widget;
  window.__MACUI_WIDGET_INSTANCE = widget;
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "fixture" }, currentWebview: { label: "fixture" } },
    transformCallback: (handler) => { callbacks.set(++callbackId, handler); return callbackId; },
    unregisterCallback: (id) => callbacks.delete(id),
    convertFileSrc: (path) => path.replaceAll("\\", "/"),
    invoke: async (command, args = {}) => {
      if (command === "plugin:event|listen") {
        events.set(args.event, [...(events.get(args.event) || []), args.handler]); return args.handler;
      }
      if (command.startsWith("plugin:event|")) return;
      if (command === "plugin:dialog|open") return "C:\\Fixture\\Mapped";
      switch (command) {
        case "state_get_settings": return { language: params.has("en") ? "en" : "zh-CN", activeIconPacks: ["@fixture/icons"], byWidget: {}, monitorsV3: {} };
        case "state_get_icon_packs": return [{ id: "@fixture/icons", metadata: { path: "/fixture-icons" }, missing: fixtureIcon("file"), entries: entries.map((item) => ({ type: "unique", path: item.path, umid: null, icon: fixtureIcon(item.isDirectory ? "folder" : item.extension) })), remoteEntries: [], downloaded: true }];
        case "get_icon": return null;
        case "get_connected_monitors": return [{ id: "fixture-monitor", name: "测试显示器", isPrimary: true, scaleFactor: 1,
          rect: { left: 0, top: 0, right: innerWidth, bottom: innerHeight } }];
        case "get_desktop_entries": return structuredClone(entries);
        case "get_desktop_folder_entries": return structuredClone(folderEntries(args.path));
        case "get_desktop_new_file_types": return [];
        case "get_notifications": return [];
        case "get_communication_notifications": return structuredClone(notifications);
        case "get_notifications_mode": return "All";
        case "get_system_tray_icons": return [];
        case "desktop_is_pointer_over_window": return true;
        case "desktop_show_background_menu": {
          // Browser verification covers organizer callbacks only. The real
          // Windows background menu is built and tested by the native host.
          const result = (action) => ({ action, shellCommand: false, shellVerb: null, iconSize: null,
            iconsVisible: null, autoArrange: null, snapToGrid: null, sort: null, sortDescending: null });
          record(command, { organizerLabel: args.organizerLabel });
          return new Promise((resolve) => {
            nativeMenuResolve = () => resolve(result(null));
            showMenu(args.menu, (action) => { nativeMenuResolve = null; resolve(result(action)); });
            const label = document.createElement("span");
            label.textContent = "整理子菜单适配测试；Windows 主菜单由原生宿主生成";
            const menu = document.getElementById("fixture-menu");
            menu?.prepend(label);
            if (params.has("qa")) {
              for (const [label, changes] of [
                ["模拟 Windows：关闭自动排列", { autoArrange: false, snapToGrid: false }],
                ["模拟 Windows：启用自动排列", { autoArrange: true, snapToGrid: true }],
                ["模拟 Windows：图标对齐网格", { autoArrange: false, snapToGrid: true }],
              ]) {
                const button = document.createElement("button"); button.textContent = label;
                button.onclick = () => { menu.remove(); nativeMenuResolve = null; resolve({ ...result(null), shellCommand: true, ...changes }); };
                menu?.append(button);
              }
            }
          });
        }
        case "desktop_show_context_menu": {
          record(command, args);
          return new Promise((resolve) => {
            nativeMenuResolve = resolve;
            document.getElementById("fixture-menu")?.remove();
            const menu = document.createElement("div"); menu.id = "fixture-menu"; menu.setAttribute("role", "menu");
            const label = document.createElement("span"); label.textContent = "模拟 Windows 原生菜单"; menu.append(label);
            for (const [name, result] of [["重命名", args.paths[0]], ["属性", null], ["关闭原生菜单", null]]) {
              const button = document.createElement("button"); button.textContent = name;
              button.onclick = () => { menu.remove(); nativeMenuResolve = null; resolve(result); }; menu.append(button);
            }
            document.body.append(menu);
          });
        }
        case "rename_desktop_entry": {
          if (args.path.startsWith("::{")) throw new Error("Fixture rejected filesystem rename of namespace item");
          const source = allEntries().find((item) => item.path === args.path);
          if (!source) throw new Error("Fixture source missing");
          const parent = args.path.slice(0, args.path.lastIndexOf("\\"));
          const renamed = { ...source, name: args.newName, path: `${parent}\\${args.newName}` };
          removeEntry(args.path); diskList(parent).push(renamed); persistEntries(); record(command, args); return renamed.path;
        }
        case "desktop_delete_files": {
          if (args.paths.some((path) => path.startsWith("::{"))) throw new Error("Fixture rejected filesystem deletion of namespace item");
          for (const path of args.paths) removeEntry(path); persistEntries(); record(command, args); return;
        }
        case "desktop_copy_files":
        case "desktop_paste_files":
        case "desktop_file_properties": record(command, args); return;
        case "desktop_get_drop_modifiers": return [...modifiers];
        case "desktop_start_file_drag": {
          record(command, args);
          return new Promise((resolve) => {
            pendingDrag = { paths: args.paths, finish: resolve };
            void emitDrag({ type: "enter", paths: args.paths, position: lastPoint });
          });
        }
        case "desktop_transfer_files": {
          const destination = args.destination || "C:\\Fixture\\Desktop", list = diskList(destination), result = [];
          for (const path of args.paths) {
            if (path.startsWith("::{")) throw new Error("Fixture rejected namespace transfer");
            const source = allEntries().find((item) => item.path === path) || entry(path.split("\\").at(-1), false, path.slice(0, path.lastIndexOf("\\")));
            const copy = args.copy ?? path.slice(0, 2).toLowerCase() !== destination.slice(0, 2).toLowerCase();
            if (!copy && path.slice(0, path.lastIndexOf("\\")).toLowerCase() === destination.toLowerCase()) { result.push(path); continue; }
            const name = uniqueName(list, source.name), next = { ...source, name, path: `${destination}\\${name}`, isDesktopEntry: destination === "C:\\Fixture\\Desktop" };
            if (!copy) removeEntry(path); list.push(next); result.push(next.path);
          }
          persistEntries(); record(command, { ...args, actualPaths: result }); return result;
        }
        case "dismiss_communication_notification":
          notifications = notifications.filter((item) => item.id !== args.id);
          emit("communication-notifications::changed", structuredClone(notifications)); break;
        case "clear_communication_notifications": notifications = []; emit("communication-notifications::changed", []); break;
        case "read_data_file": {
          if (files.has(args.filename)) return files.get(args.filename);
          if (params.has("fresh")) throw new Error("No fixture preferences yet");
          return '{"arrangeMode":"category"}';
        }
        case "write_data_file": files.set(args.filename, args.content); localStorage.setItem(storeKey, JSON.stringify(Object.fromEntries(files))); break;
        case "trigger_context_menu": showMenu(args.menu); break;
        case "get_frosted_wallpaper_luminance": return (args.rects || []).map(() => params.has("light-wallpaper") ? .9 : .08);
        case "set_frosted_regions": throw new Error("Browser fixture has no native blur");
        case "set_desktop_regions": return;
      }
      record(command, args);
    },
  };
  const updatePointer = (event) => {
    modifiers = [event.ctrlKey, event.shiftKey, event.altKey];
    lastPoint = { x: event.clientX * devicePixelRatio, y: event.clientY * devicePixelRatio };
  };
  window.addEventListener("pointermove", (event) => {
    updatePointer(event);
    if (pendingDrag) void emitDrag({ type: "over", position: lastPoint });
  });
  window.addEventListener("pointerup", (event) => {
    updatePointer(event);
    const drag = pendingDrag;
    if (!drag) return;
    pendingDrag = null;
    void emitDrag({ type: "drop", paths: drag.paths, position: lastPoint }).finally(drag.finish);
  });
  window.addEventListener("keydown", (event) => {
    modifiers = [event.ctrlKey, event.shiftKey, event.altKey];
    if (event.key === "Escape" && nativeMenuResolve) {
      document.getElementById("fixture-menu")?.remove(); nativeMenuResolve(null); nativeMenuResolve = null;
    }
    if (event.key === "Escape" && pendingDrag) {
      const drag = pendingDrag; pendingDrag = null;
      void emitDrag({ type: "leave" }).finally(drag.finish);
    }
  });
  window.addEventListener("keyup", (event) => { modifiers = [event.ctrlKey, event.shiftKey, event.altKey]; });

  if (panel === "desktop-shell" && params.has("qa")) {
    window.addEventListener("keydown", (event) => {
      console.debug("fixture key", JSON.stringify({ key: event.key, ctrl: event.ctrlKey, shift: event.shiftKey, alt: event.altKey, meta: event.metaKey }));
    }, true);
    const controls = document.createElement("details"); controls.id = "fixture-controls";
    controls.style.cssText = "position:fixed;bottom:24px;right:8px;z-index:12000;max-width:350px;padding:8px;border-radius:10px;background:#fff;color:#17243a;font:12px sans-serif";
    const summary = document.createElement("summary"); summary.textContent = "隔离夹具控制"; controls.append(summary);
    const targets = document.createElement("select"); targets.setAttribute("aria-label", "夹具拖放目标");
    targets.onfocus = () => {
      const previous = targets.value; targets.replaceChildren();
      for (const node of document.querySelectorAll("[data-icon-sort-scope]")) {
        const option = document.createElement("option"); option.value = node.dataset.iconSortScope;
        option.textContent = node.getAttribute("aria-label") || option.value; targets.append(option);
      }
      if ([...targets.options].some((option) => option.value === previous)) targets.value = previous;
    };
    controls.append(targets);
    const drop = document.createElement("button"); drop.textContent = "模拟外部文件落入";
    drop.onclick = async () => {
      const target = [...document.querySelectorAll("[data-icon-sort-scope]")].find((node) => node.dataset.iconSortScope === targets.value);
      if (!target) return;
      target.scrollIntoView({ block: "nearest" }); const rect = target.getBoundingClientRect();
      const position = { x: (rect.left + rect.width / 2) * devicePixelRatio, y: (rect.top + Math.min(rect.height / 2, 60)) * devicePixelRatio };
      modifiers = [false, false, false];
      const paths = ["D:\\ExternalFixture\\外部报告.txt"];
      await emitDrag({ type: "enter", paths, position }); await emitDrag({ type: "drop", paths, position });
    };
    controls.append(drop);
    const inspect = document.createElement("button"); inspect.textContent = "查看夹具状态";
    const output = document.createElement("pre"); output.id = "fixture-state"; output.style.cssText = "max-height:260px;overflow:auto;white-space:pre-wrap;font:10px monospace";
    inspect.onclick = () => { output.textContent = JSON.stringify({ entries, mappings: Object.fromEntries(mappings), files: Object.fromEntries(files) }, null, 2); };
    controls.append(inspect, output); document.body.append(controls);
    const marqueeCheck = document.createElement("button"); marqueeCheck.textContent = "验证盒内框选事件";
    marqueeCheck.onclick = async () => {
      const root = document.querySelector(".desktop-shell");
      const box = document.querySelector('[data-icon-sort-scope="documents"] .stack-items');
      if (!root || !box) return;
      box.scrollIntoView({ block: "center" });
      const rows = [];
      const recordSelection = (phase) => rows.push({ phase, selected: [...box.querySelectorAll('[aria-pressed="true"]')].map(e => e.getAttribute("aria-label")), marquee: !!document.querySelector(".desktop-marquee") });
      const settle = () => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      // This is an event-adapter test, not a claim of physical pointer capture.
      const originals = [root.setPointerCapture, root.hasPointerCapture, root.releasePointerCapture];
      let captured = false;
      root.setPointerCapture = () => { captured = true; };
      root.hasPointerCapture = () => captured;
      root.releasePointerCapture = () => { captured = false; };
      try {
        root.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true })); await settle();
        const r = box.getBoundingClientRect();
        const items = box.querySelectorAll('[data-desktop-icon-path]');
        const last = items[1].getBoundingClientRect();
        const pointer = (type, target, x, y) => target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 77, isPrimary: true, button: 0, buttons: type === "pointerup" ? 0 : 1, clientX: x, clientY: y }));
        pointer("pointerdown", box, r.left + 3, r.top + 3); await settle(); recordSelection("down");
        pointer("pointermove", root, last.right + 2, last.bottom + 2); await settle(); recordSelection("move");
        window.dispatchEvent(new Event("blur")); await settle(); recordSelection("captured-blur");
        pointer("pointerup", root, last.right + 2, last.bottom + 2); await settle(); recordSelection("up");
        root.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true })); await settle(); recordSelection("click");
      } finally {
        [root.setPointerCapture, root.hasPointerCapture, root.releasePointerCapture] = originals;
        output.textContent = JSON.stringify({ marqueeEventCheck: rows }, null, 2);
      }
    };
    controls.append(marqueeCheck);
  }
})();
