// QA only. No native APIs, real files, user notifications or account information.
(() => {
  const params = new URLSearchParams(location.search);
  const empty = params.has("empty");
  const panel = location.pathname.split("/")[2];
  const callbacks = new Map(), events = new Map();
  let nextCallback = 0, operations = 0;
  const noop = async () => {};
  const status = (command, args) => {
    document.getElementById("fixture-status").textContent = `${++operations} · ${command} ${JSON.stringify(args)}`;
  };
  const emit = (name, payload) => {
    for (const handler of events.get(name) || []) callbacks.get(handler)?.({ event: name, payload, id: handler });
  };
  // Production appends ?hash= to icon paths. HTTP paths keep that cache key
  // out of the SVG text; appending it to a data URI corrupts the document.
  const icon = (letter, color = "#1872d9") =>
    `/fixture-icon/${encodeURIComponent(letter)}/${color.replace("#", "")}.svg`;
  // Bluetooth's empty fixture keeps a working radio so the empty-device state
  // is distinct from the no-adapter case. Other panels keep their existing QA.
  const bluetoothPanel = panel === "bluetooth-popup";
  const radios = (empty && !bluetoothPanel) || params.has("no-adapter") ? [] : [
    { id: "wifi", kind: "WiFi", name: "Test Wi-Fi", isEnabled: true },
    { id: "bt", kind: "Bluetooth", name: "Test Bluetooth", isEnabled: !params.has("disabledRadio") && !params.has("disabled-radio") },
  ];
  const mediaDevice = (id, name, volume, active = true) => ({
    id,
    name,
    volume,
    muted: false,
    isDefaultMultimedia: active,
    isDefaultCommunications: active,
    sessions: [],
  });
  const media = empty ? [[], []] : [[mediaDevice("input", "Studio Microphone", .54)], [
    mediaDevice("output", "Studio Speakers", .36),
    mediaDevice("display", "External Display Audio · Long Device Name", .5, false),
  ]];
  let notes = empty ? [] : [1, 2, 3].map((id) => ({
    id,
    appUmid: "fixture-chat",
    appName: id === 1 ? "信息" : "日历",
    date: Date.now() - id * 300_000,
    content: {
      "@launch": "open",
      "@activationType": "Foreground",
      visual: {
        binding: {
          "@template": "ToastGeneric",
          $value: [
            { text: { $value: id === 1 ? "设计讨论" : "稍后安排" } },
            {
              text: {
                $value: id === 1
                  ? "这是一条虚构通知，用于检查正文、圆角和回复输入框。不会读取或发送真实消息。"
                  : "检查长文本是否自然换行，滚动时底部设置入口仍然可见。",
              },
            },
          ],
        },
      },
      actions: {
        $value: [
          { input: { "@type": "Text", "@id": "reply", "@placeHolderContent": "回复测试消息" } },
          { action: { "@content": "回复", "@arguments": "reply", "@activationType": "Background" } },
        ],
      },
    },
  }));
  let notificationsMode = "All";
  const langs = [{
    name: "简体中文（中国大陆）",
    keyboardLayouts: [
      { id: "pinyin", handle: "1", displayName: "微软拼音", active: true },
      { id: "sogou", handle: "2", displayName: "搜狗拼音输入法", active: false },
    ],
  }];
  const btDevices = [
    {
      id: "mouse",
      name: "Studio Mouse",
      paired: true,
      connected: true,
      isLowEnergy: true,
      appearance: { category: "HumanInterfaceDevice", subcategory: "Mouse" },
    },
    {
      id: "audio",
      name: "Studio Headphones",
      paired: true,
      connected: false,
      isLowEnergy: false,
      appearance: { category: "WearableAudioDevice" },
    },
    {
      id: "keyboard",
      name: "Wireless Keyboard",
      paired: false,
      connected: false,
      isLowEnergy: true,
      appearance: { category: "HumanInterfaceDevice", subcategory: "Keyboard" },
    },
  ];
  if (params.has("long")) {
    btDevices.push(...Array.from({ length: 27 }, (_, i) => ({
      id: `fixture-bluetooth-${i}`,
      name: `虚构蓝牙设备 ${i} · Long Device Name`,
      paired: false,
      connected: false,
      isLowEnergy: true,
      appearance: { category: "HumanInterfaceDevice", subcategory: i % 2 ? "Mouse" : "Keyboard" },
    })));
  }
  const bt = empty ? [] : btDevices.map((device, i) => ({
    address: 0xAABBCCDD0000 + i,
    majorServiceClasses: [],
    class: { major: "Uncategorized" },
    classRaw: null,
    appearanceRaw: null,
    canPair: !device.paired,
    canConnect: device.paired && !device.connected,
    canDisconnect: device.connected,
    ...device,
  }));
  const networkNames = params.has("long") ? Array.from({ length: 30 }, (_, i) => `测试网络 ${i} · Long SSID Name`) : ["Studio Wi-Fi", "Guest Network", "Cafe Network"];
  const networks = empty ? [] : networkNames.map((ssid, i) => ({
    ssid,
    bssid: `fixture-${i}`,
    known: i === 0,
    connected: i === 0,
    secured: true,
    signal: Math.max(5, 90 - i * 8),
    channelFrequency: 5_200_000,
    auth: "WPA2",
    channel: 40,
  }));
  const hotspot = empty
    ? null
    : { ssid: "MAC UI Hotspot", state: "off", password: "fixture-only", clients: [], band: "Auto" };
  const tray = empty ? [] : Array.from({ length: params.has("long") ? 40 : 13 }, (_, i) => ({
    stable_id: { guid: `fixture-${i}` },
    tooltip: i === 0 ? "微信" : `后台应用 ${i}`,
    executable: i === 0 ? "WeChat.exe" : `Fixture${i}.exe`,
    icon_path: icon(i === 0 ? "W" : String(i), i === 0 ? "#13b762" : "#5570a1"),
    icon_image_hash: "fixture",
    is_visible: i < 4,
    notification: null,
  }));
  window.__SLU_WIDGET = { id: `@seelen/${panel}`, settings: [] };
  window.__SLU_WIDGET_INSTANCE = {
    id: `@seelen/${panel}`,
    decoded: {},
    init: noop,
    ready: async () => {
      emit("wlan-scanned", networks);
    },
    focus: noop,
    setPosition: noop,
    hide: async () => status("hide", {}),
    onTrigger: () => () => {},
    window: { onFocusChanged: async () => () => {}, isVisible: async () => true, setResizable: noop },
    webview: { listen: async () => () => {} },
  };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "fixture" }, currentWebview: { label: "fixture" } },
    transformCallback: (callback) => {
      callbacks.set(++nextCallback, callback);
      return nextCallback;
    },
    unregisterCallback: (id) => callbacks.delete(id),
    convertFileSrc: (path) => path,
    invoke: async (command, args = {}) => {
      if (command === "plugin:event|listen") {
        const list = events.get(args.event) || [];
        list.push(args.handler);
        events.set(args.event, list);
        return args.handler;
      }
      if (command.startsWith("plugin:event|")) return;
      if (command.startsWith("plugin:window|")) {
        return {
          name: "Fixture",
          scaleFactor: params.has("dpi") ? 2 : 1,
          position: { x: 0, y: 0 },
          size: { width: 1920, height: 1080 },
          workArea: { position: { x: 0, y: 0 }, size: { width: 1920, height: params.has("short") ? 480 : 1040 } },
        };
      }
      if (command.startsWith("plugin:path|")) return "C:\\Fixture";
      switch (command) {
        case "set_frosted_regions":
          throw new Error("Browser QA has no Windows backdrop composition; CSS preview only");
        case "get_frosted_wallpaper_luminance":
          return (args.rects || []).map(() => params.has("dark-wallpaper") ? .03 : .4);
        case "state_get_settings":
          return {
            language: params.has("en") ? "en" : "zh-CN",
            activeIconPacks: [],
            byWidget: {},
            monitorsV3: {},
            devTools: false,
            startOfWeek: "Monday",
          };
        case "state_get_icon_packs":
          return [];
        case "get_icon":
          return null;
        case "get_user":
          return { name: "JONA", profilePicturePath: icon("J", "#5d61c8") };
        case "get_user_folder_content":
          return empty ? [] : ["C:\\Fixture\\设计方案.pdf", "C:\\Fixture\\灵感.png"];
        case "get_radios":
          return structuredClone(radios);
        case "get_network_hotspot":
          return hotspot;
        case "get_bluetooth_devices":
          return structuredClone(bt);
        case "get_media_devices":
          return structuredClone(media);
        case "get_media_sessions":
          return empty ? [] : [{
            umid: "player",
            title: "Evening Light",
            author: "Fixture Artist",
            thumbnail: icon("♫", "#496e85"),
            owner: { name: "Fixture Music" },
            playing: false,
            timeline: { start: 0, position: 80e9, end: 230e9 },
          }];
        case "get_all_monitors_brightness":
          return [];
        case "get_connected_monitors":
          return [{ id: "fixture-monitor", hdr: true, isPrimary: true, scaleFactor: 1,
            rect: { left: 0, top: 0, right: innerWidth, bottom: innerHeight } }];
        case "get_system_dark_mode":
          return params.has("dark");
        case "get_system_night_light_enabled":
          return false;
        case "get_notifications":
          return structuredClone(notes);
        case "get_notifications_mode":
          return notificationsMode;
        case "get_system_languages":
          return structuredClone(langs);
        case "get_system_tray_icons":
          return tray;
        case "set_radios_state":
          radios.forEach((r) => {
            if (r.kind === args.kind) r.isEnabled = args.enabled;
          });
          emit("radio::changed", structuredClone(radios));
          break;
        case "set_volume_level":
          media.flat().forEach((d) => {
            if (d.id === args.deviceId) d.volume = args.level;
          });
          emit("media::devices", structuredClone(media));
          break;
        case "set_system_keyboard_layout":
          langs[0].keyboardLayouts.forEach((k) => {
            k.active = k.id === args.id;
          });
          emit("system::languages-changed", structuredClone(langs));
          break;
        case "wlan_connect":
          status(command, args);
          if (params.has("fail-connect")) return false;
          networks.forEach((network) => { network.connected = network.ssid === args.ssid; });
          emit("wlan-scanned", structuredClone(networks));
          return true;
        case "wlan_disconnect":
          networks.forEach((network) => { network.connected = false; });
          emit("wlan-scanned", structuredClone(networks));
          break;
        case "set_notifications_mode":
          notificationsMode = args.mode;
          emit("notifications::mode-changed", notificationsMode);
          break;
        case "notifications_close":
          notes = notes.filter((n) => n.id !== args.id);
          emit("notifications", structuredClone(notes));
          break;
        case "notifications_close_all":
          notes = [];
          emit("notifications", []);
          break;
        default:
          break;
      }
      status(command, args); // Only records an action. Does not call the OS or persist anything.
    },
  };
})();
