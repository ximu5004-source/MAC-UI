// Loaded only by launchpad-server.mjs; never included by production entry discovery.
(() => {
  const status = text => { document.getElementById("fixture-status").textContent = `隔离测试 · ${text}`; };
  const app = (name, folders = []) => ({ path: `C:\\Fixture\\${folders.join("\\")}\\${name}.lnk`, target: `C:\\Fixture\\${name}.exe`, display_name: name, umid: null, toast_activator: null, start_menu_folder: folders });
  const catalog = [app("Alpha"), app("Beta"), app("绘图", ["工具", "图形"]), app("计算器", ["工具", "图形"]), app("Single", ["单应用文件夹"]), ...Array.from({ length: 76 }, (_, i) => app(`应用 ${String(i + 1).padStart(2, "0")}`))];
  const options = new URLSearchParams(location.search);
  if (options.has("many-folder")) catalog.push(...Array.from({ length: 22 }, (_, i) => app(`工具 ${String(i + 1).padStart(2, "0")}`, ["工具", "图形"])));
  const iconPack = {
    id: "fixture", metadata: { path: "/fixture-icons" }, missing: null,
    entries: catalog.map((item, index) => ({ type: "unique", path: item.path, umid: null, redirect: null,
      icon: { base: `${index}.svg`, light: null, dark: null, mask: null, isAproximatelySquare: true } })),
    remoteEntries: [], downloaded: false,
  };
  // Isolated accessibility branch test only; never changes Windows preferences.
  if (options.get("motion") === "reduce") {
    const matchMedia = window.matchMedia.bind(window);
    window.matchMedia = query => {
      const media = matchMedia(query);
      if (query === "(prefers-reduced-motion: reduce)") Object.defineProperty(media, "matches", { value: true });
      return media;
    };
  }
  // DOM-readable diagnostics for tests; no production entry imports this file.
  const motionStatus = document.createElement("output");
  motionStatus.id = "fixture-motion";
  motionStatus.style.cssText = "position:fixed;left:4px;bottom:16px;font:11px sans-serif;color:white;background:#233047;pointer-events:none";
  document.body.append(motionStatus);
  let peak = 0, maxSnapshots = 0, sawForward = false, sawBackward = false;
  const inspectMotion = () => {
    const motions = document.getAnimations().filter(animation => animation.effect?.target?.classList.contains("launchpad-page-content"));
    peak = Math.max(peak, motions.length);
    for (const motion of motions) {
      const first = motion.effect.getKeyframes()[0]?.transform;
      if (first === "translate3d(100%, 0px, 0px)") sawForward = true;
      if (first === "translate3d(-100%, 0px, 0px)") sawBackward = true;
    }
    const snapshots = document.querySelectorAll(".launchpad-page-snapshots > *").length;
    maxSnapshots = Math.max(maxSnapshots, snapshots);
    motionStatus.textContent = `动画 ${motions.length} · 峰值 ${peak} · 快照 ${snapshots}/${maxSnapshots} · 前进 ${sawForward} · 后退 ${sawBackward}`;
    requestAnimationFrame(inspectMotion);
  };
  requestAnimationFrame(inspectMotion);
  let shown = true, nextCallback = 0;
  const callbacks = new Map();
  const noop = async () => {};
  window.__SLU_WIDGET = { id: "@seelen/apps-menu", settings: [] };
  window.__SLU_WIDGET_INSTANCE = {
    id: "@seelen/apps-menu", decoded: {},
    init: noop, ready: noop, focus: noop, setPosition: noop,
    hide: async () => { shown = false; status("收到关闭窗口请求"); },
    show: async () => { shown = true; },
    onTrigger: () => () => {},
    window: { isVisible: async () => shown, setFocusable: noop },
    webview: { listen: async () => () => {} },
  };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "fixture" }, currentWebview: { label: "fixture" } },
    transformCallback: callback => { callbacks.set(++nextCallback, callback); return nextCallback; },
    unregisterCallback: id => callbacks.delete(id),
    convertFileSrc: path => path.replaceAll("\\", "/"),
    invoke: async (command, args = {}) => {
      if (command.startsWith("plugin:event|")) return 1;
      switch (command) {
        case "get_user_folder_content": return args.folderType === "Documents" ? ["C:\\Fixture\\设计计划.pdf"] : [];
        case "get_start_menu_items": return structuredClone(catalog);
        case "get_native_start_menu": return { pinnedList: [] };
        case "get_native_shell_wallpaper": return "";
        case "get_user": return { name: "Fixture", profilePicturePath: null };
        case "get_connected_monitors": return [{ isPrimary: true, scaleFactor: 1, rect: { left: 0, top: 0, right: innerWidth, bottom: innerHeight } }];
        case "state_get_settings": return { language: options.has("en") ? "en" : "zh-CN", activeIconPacks: ["fixture"], byWidget: {}, monitorsV3: {} };
        case "state_get_icon_packs": return [structuredClone(iconPack)];
        case "set_frosted_regions": throw new Error("Isolated browser cannot supply Windows frost; CSS preview only");
        case "get_frosted_wallpaper_luminance": return (args.rects || []).map(() => options.has("dark") ? .03 : .4);
        case "read_data_file": {
          const response = await fetch(`/fixture-storage?key=${encodeURIComponent(args.filename)}`);
          if (!response.ok) throw new Error("No fixture storage");
          return response.text();
        }
        case "write_data_file": {
          const response = await fetch(`/fixture-storage?key=${encodeURIComponent(args.filename)}`, { method: "POST", body: args.content });
          if (!response.ok) throw new Error("Fixture save failed");
          return;
        }
        case "get_icon": return null;
        case "open_user_folder": status(`收到打开虚构文件夹请求：${args.folder}`); return;
        case "widget_trigger":
        case "trigger_widget": status(`收到组件请求：${args.payload?.id}`); return;
        case "open_file": status(`收到应用启动请求：${args.path}`); return;
        case "trigger_context_menu": status(`右键菜单：${args.menu.items.map(i => i.label || "").join(" · ")}`); return;
        default: throw new Error(`Unexpected native command in isolated fixture: ${command}`);
      }
    },
  };
})();
