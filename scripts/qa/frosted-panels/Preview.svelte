<script lang="ts">
  import { onMount } from "svelte";
  import QuickToggle from "src/ui/svelte/quick-settings/components/QuickToggle.svelte";
  import MediaDevices from "src/ui/svelte/flyouts/app/MediaDevices.svelte";
  import Brightness from "src/ui/svelte/flyouts/app/Brightness.svelte";
  import type { MediaDevice, MonitorBrightness } from "@seelen-ui/lib/types";
  import { setMessages, locale } from "libs/ui/svelte/utils/i18n";
  import { invoke, SeelenCommand } from "./native-mock";
  import BackgroundByLayers from "libs/ui/svelte/components/BackgroundByLayers/BackgroundByLayers.svelte";
  import DesktopStack from "src/ui/svelte/desktop-shell/modules/DesktopShell/DesktopStack.svelte";
  import { frostedForeground } from "libs/ui/svelte/utils/frostedContrast";

  let dark = $state(true);
  let adaptiveInk = $state(true);
  let darkWallpaper = $state(false);
  const wallpaperLuminance = $derived.by(() => {
    // Known fixture palette, approximately averaged with its alternating white
    // stripes. This simulates a wallpaper sample; it never captures Windows.
    const stops = darkWallpaper ? [[24, 32, 45], [43, 31, 44], [27, 30, 50]] : [[84, 117, 140], [187, 142, 139], [111, 101, 148]];
    const stripeAlpha = darkWallpaper ? .04 : .12;
    return stops.reduce((sum, rgb) => {
      const linear = rgb.map((channel) => {
        const srgb = channel / 255 * (1 - stripeAlpha) + stripeAlpha;
        return srgb <= .04045 ? srgb / 12.92 : ((srgb + .055) / 1.055) ** 2.4;
      });
      return sum + linear[0]! * .2126 + linear[1]! * .7152 + linear[2]! * .0722;
    }, 0) / stops.length;
  });
  const adaptiveForeground = $derived(adaptiveInk ? frostedForeground(wallpaperLuminance) : undefined);
  let wifi = $state(true);
  let bluetooth = $state(false);
  let failNext = $state(false);
  const toggleRequests = $state({ wifi: 0, bluetooth: 0, hotspot: 0, hdr: 0, appearance: 0, eyeCare: 0 });
  const detailRequests = $state({ wifi: 0, bluetooth: 0 });
  let pendingRequests = $state(0);
  let details = $state("未打开详情");
  let native = $state("正在检查原生边界");
  let gloss = $state(false);
  let showMaterials = $state(true);
  let collapsed = $state(false);
  let stackPosition = $state({ x: 0, y: 0 });
  let stackSize = $state<{ width: number; height: number } | undefined>({ width: 320, height: 280 });
  let output = $state({ id: "qa-output", name: "隔离测试输出", volume: .44, muted: false } as MediaDevice);
  let brightness = $state({ instanceName: "qa-monitor", currentBrightness: 60, availableLevels: [0, 20, 40, 60, 80, 100], levels: 6 } as MonitorBrightness);

  const labels = {
    open_details: "打开{{name}}设置", updating: "正在更新…", action_failed: "操作失败，请重试",
    toggle_mute: "切换静音", volume: "音量", brightness: "亮度",
    desktop_shell: {
      move_hint: "移动分组", move_controls: "位置与尺寸", collapse: "折叠", expand: "展开", resize_hint: "调整尺寸",
      resize_n: "调整上边", resize_s: "调整下边", resize_e: "调整右边", resize_w: "调整左边", resize_ne: "调整右上角", resize_nw: "调整左上角", resize_se: "调整右下角", resize_sw: "调整左下角",
      move_left: "左移", move_right: "右移", move_up: "上移", move_down: "下移", narrower: "窄一点", wider: "宽一点", shorter: "矮一点", taller: "高一点",
    },
  };
  setMessages({ en: labels, "zh-CN": labels });
  void locale.set("zh-CN");

  async function toggle(kind: keyof typeof toggleRequests, action: () => void) {
    toggleRequests[kind] += 1;
    pendingRequests += 1;
    try {
      await new Promise((resolve) => setTimeout(resolve, 500));
      if (failNext) { failNext = false; throw new Error("Expected QA failure"); }
      action();
    } finally {
      pendingRequests -= 1;
    }
  }

  async function openDetails(kind: keyof typeof detailRequests) {
    detailRequests[kind] += 1;
    details = kind === "wifi" ? "已打开 Wi-Fi 详情（测试）" : "已打开蓝牙详情（测试）";
  }

  onMount(() => {
    void invoke(SeelenCommand.SetFrostedRegions).then(() => {
      native = "错误：网页不应该报告原生合成成功";
    }).catch(() => { native = "原生合成不支持：仅 CSS 回退预览，不是 Windows 磨砂验收"; });
    const handler = (event: Event) => {
      const { command, payload } = (event as CustomEvent).detail;
      if (command === SeelenCommand.SetVolumeLevel) output = { ...output, volume: payload.level };
      if (command === SeelenCommand.MediaToggleMute) output = { ...output, muted: !output.muted };
      if (command === SeelenCommand.SetMonitorBrightness) brightness = { ...brightness, currentBrightness: payload.level };
    };
    window.addEventListener("qa-native-command", handler);
    return () => window.removeEventListener("qa-native-command", handler);
  });
</script>

<div class="fixture" class:dark-wallpaper={darkWallpaper} style:color-scheme={dark ? "dark" : "light"}>
  <header class="qa-toolbar">
    <strong>真实组件隔离预览</strong>
    <button onclick={() => dark = !dark}>{dark ? "切换浅色" : "切换深色"}</button>
    <label><input type="checkbox" bind:checked={failNext} />下次开关请求失败</label>
    <label><input type="checkbox" bind:checked={gloss} />Dock 光泽 100%</label>
    <label><input type="checkbox" bind:checked={showMaterials} />显示测试材质</label>
    <label><input type="checkbox" bind:checked={adaptiveInk} />模拟壁纸采样自动文字</label>
    <label><input type="checkbox" bind:checked={darkWallpaper} />暗壁纸样本</label>
    <span>模拟亮度 {wallpaperLuminance.toFixed(3)} → {adaptiveForeground ?? "跟随主题"}（非原生采样）</span>
    <span role="status">{native}</span>
  </header>
  <div class="qa-layout">
    <section class="slu-std-popover mac-panel mac-frosted-surface quick-settings" data-frosted-ink={adaptiveForeground}>
      <h1 class="mac-panel-heading">控制中心</h1>
      <div class="quick-toggle-grid">
        <QuickToggle icon="IoWifiSharp" label="Wi-Fi" status={wifi ? "启用" : "禁用"} enabled={wifi}
          onToggle={() => toggle("wifi", () => wifi = !wifi)} onDetails={() => openDetails("wifi")} />
        <QuickToggle icon="IoBluetooth" label="Bluetooth" status={bluetooth ? "启用" : "禁用"} enabled={bluetooth}
          onToggle={() => toggle("bluetooth", () => bluetooth = !bluetooth)} onDetails={() => openDetails("bluetooth")} />
        <QuickToggle icon="MdWifiTethering" label="热点" status="禁用" enabled={false} onToggle={() => toggle("hotspot", () => {})} />
        <QuickToggle icon="TbHdr" label="高动态范围" status="启用" enabled={true} onToggle={() => toggle("hdr", () => {})} />
        <QuickToggle icon="IoMoon" label="深色模式" status="外观" enabled={true} onToggle={() => toggle("appearance", () => {})} />
        <QuickToggle icon="IoEye" label="护眼模式" status="禁用" enabled={false} onToggle={() => toggle("eyeCare", () => {})} />
      </div>
      <p class="details-result" role="status">{details}</p>
      <output class="qa-request-counts" data-testid="quick-toggle-invocations">
        开关请求：Wi-Fi {toggleRequests.wifi} · Bluetooth {toggleRequests.bluetooth} · 热点 {toggleRequests.hotspot} · HDR {toggleRequests.hdr} · 外观 {toggleRequests.appearance} · 护眼 {toggleRequests.eyeCare}
        <br />详情请求：Wi-Fi {detailRequests.wifi} · Bluetooth {detailRequests.bluetooth}
        <br />进行中请求：{pendingRequests}（仅隔离预览，不调用 Windows）
      </output>
    </section>
    <section class="qa-osds" aria-label="实际音量和亮度组件">
      <div class="flyout mac-frosted-surface" data-frosted-ink={adaptiveForeground} data-placement="top" data-showing="true"><MediaDevices {output} orientation="horizontal" /></div>
      <div class="flyout mac-frosted-surface" data-frosted-ink={adaptiveForeground} data-placement="top" data-showing="true"><Brightness {brightness} orientation="horizontal" /></div>
      <div class="flyout mac-frosted-surface" data-frosted-ink={adaptiveForeground} data-placement="right" data-showing="true"><MediaDevices {output} orientation="vertical" /></div>
    </section>
    <section class="qa-materials" aria-label="实际 Dock 与桌面分组材质">
      <div class="qa-dock mac-dock-material" data-mac-material="dock" style:--mac-dock-alpha=".1" style:--mac-dock-gloss={gloss ? 1 : 0} style:opacity={showMaterials ? 1 : 0}>
        <BackgroundByLayers />
        <span class="qa-dock-caption">Dock 内部材质</span>
      </div>
      <div class="qa-stack-stage" style:opacity={showMaterials ? 1 : 0}>
        <DesktopStack title="图片与文件" count={4} {collapsed} position={stackPosition} dimensions={stackSize} bounds={{ width: 340, height: 320 }}
          active={false} snap={false} onmove={(point) => stackPosition = point} onresize={(point, size) => { stackPosition = point; stackSize = size; }}
          oninteraction={() => {}} oncollapse={() => collapsed = !collapsed} onactivate={() => {}}>
          {#snippet children()}
            {#each ["设计稿", "图片", "文件夹", "文档"] as item}<span class="qa-stack-item">{item}</span>{/each}
          {/snippet}
        </DesktopStack>
      </div>
    </section>
  </div>
  <p class="qa-note">验收：Tab / Enter / Space 操作；开关等待时不可重复提交；失败文字；音量拖动与方向键；静音后保持音量；背景边缘与圆角。此页面不会修改 Windows。</p>
</div>

<style>
  :global(html), :global(body) { width: 100%; height: auto; margin: 0; font: 13px "Segoe UI", "Microsoft YaHei UI", sans-serif; }
  :global(#root) { width: auto; height: auto; }
  :global(button), :global(input) { font: inherit; }
  :global(button) { color: inherit; }
  .fixture {
    min-height: 100vh;
    padding: 24px;
    box-sizing: border-box;
    color: light-dark(#202020, #f4f4f4);
    background: repeating-linear-gradient(0deg, transparent 0 44px, rgb(255 255 255 / .24) 44px 88px), linear-gradient(135deg, #54758c, #bb8e8b 48%, #6f6594);
  }
  .fixture.dark-wallpaper {
    background: repeating-linear-gradient(0deg, transparent 0 44px, rgb(255 255 255 / .08) 44px 88px), linear-gradient(135deg, #18202d, #2b1f2c 48%, #1b1e32);
  }
  .qa-toolbar { display: flex; align-items: center; flex-wrap: wrap; gap: 16px; background: Canvas; color: CanvasText; padding: 12px; border-radius: 12px; }
  .qa-toolbar button { min-height: 32px; border: 1px solid; border-radius: 7px; background: Canvas; }
  .qa-layout { display: flex; flex-wrap: wrap; align-items: flex-start; gap: 32px; margin-top: 50px; }
  .qa-osds { display: flex; flex-direction: column; align-items: flex-start; gap: 14px; }
  .qa-materials { display: flex; flex-direction: column; gap: 22px; width: 340px; margin: 10px; }
  .qa-dock { position: relative; isolation: isolate; width: 320px; height: 72px; border-radius: 22px; }
  .qa-dock-caption { position: absolute; inset: 24px 50px; text-align: center; font-size: 13px; }
  .qa-stack-stage { position: relative; width: 340px; height: 320px; }
  .qa-stack-item { display: grid; place-items: center; height: 76px; border-radius: 16px; background: rgb(255 255 255 / .18); color: inherit; }
  .details-result { margin: 0; min-height: 20px; color: var(--mac-secondary); font-size: 12px; }
  .qa-request-counts { display: block; font-size: 11px; line-height: 1.5; color: var(--mac-secondary); overflow-wrap: anywhere; }
  .qa-note { max-width: 650px; margin: 28px 10px; padding: 10px; background: Canvas; color: CanvasText; border-radius: 8px; line-height: 1.6; }
</style>
