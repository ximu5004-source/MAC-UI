import { render } from "preact";
import { useState } from "react";
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { load } from "js-yaml";
import zh from "../../../src/ui/react/settings/i18n/translations/zh-CN.yml";
import { GlassAppearanceControls } from "../../../src/ui/react/settings/modules/resources/Widget/DesktopGlass";
import { DockSizeControl } from "../../../src/ui/react/settings/modules/resources/Widget/seelenweg/DockSizeControl";
import { DEFAULT_GLASS_APPEARANCE, normalizeGlassAppearance } from "../../../libs/ui/utils/glassAppearance";
import "../../../src/ui/react/settings/styles/variables.css";
import "./preview.css";

await i18next.use(initReactI18next).init({ lng: "zh", resources: { zh: { translation: load(zh) as any } } });
function Preview() {
  const [dockSize, setDockSize] = useState(40);
  const [saved, setSaved] = useState(() => normalizeGlassAppearance(JSON.parse(localStorage.getItem("mac-ui-glass-qa") || "null")));
  const [value, setValue] = useState(saved);
  const [dark, setDark] = useState(false);
  const [status, setStatus] = useState("隔离预览 · 不调用 Windows，不修改真实设置");
  return <main style={{ colorScheme: dark ? "dark" : "light" }}>
    <header><h1>桌面管理</h1><div>
      <button onClick={() => setDark(!dark)}>切换浅色/深色</button>
      <button onClick={() => { setValue(saved); setStatus("已撤销测试更改"); }}>撤销更改</button>
      <button onClick={() => { localStorage.setItem("mac-ui-glass-qa", JSON.stringify(value)); setSaved(value); setStatus("测试设置已保存"); }}>保存测试设置</button>
    </div></header>
    <p role="status">{status}</p>
    <DockSizeControl value={dockSize} onChange={setDockSize} />
    <GlassAppearanceControls value={value} onChange={setValue} />
    <button onClick={() => { setValue({ ...DEFAULT_GLASS_APPEARANCE }); setSaved({ ...DEFAULT_GLASS_APPEARANCE }); localStorage.removeItem("mac-ui-glass-qa"); setStatus("测试数据已清空"); }}>清空测试数据</button>
  </main>;
}
render(<Preview />, document.getElementById("root")!);
