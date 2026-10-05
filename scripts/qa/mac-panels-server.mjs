// Real built popovers + fake native bridge. Never reads user data or calls Windows APIs.
// npm run build:ui
// npx --yes --package=sass@1.93.2 sass scripts/qa/mac-panels-theme.scss target/qa/mac-panels-theme.css --no-source-map
// node scripts/qa/mac-panels-server.mjs
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";

const availablePanels = [
  "user-menu",
  "system-tray",
  "keyboard-selector",
  "bluetooth-popup",
  "network-popup",
  "media-popup",
  "notifications",
  "quick-settings",
  "calendar-popup",
  "power-menu",
];
const panels = process.argv[4]?.split(",").filter((panel) => availablePanels.includes(panel)) || availablePanels;
const port = Number(process.argv[3] || 3582);
const dist = resolve(process.argv[2] || "dist");
const iconRoot = resolve("dist/icons");
const theme = await readFile("target/qa/mac-panels-theme.css", "utf8");
const mock = await readFile(new URL("./mac-panels-mock.js", import.meta.url), "utf8");
const mime = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".yml": "text/plain" };
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://127.0.0.1:${port}`);
    res.setHeader("Cache-Control", "no-store");
    // Virtual icons contain fixture labels only. Never read native icon paths
    // or user files, and tolerate the production icon cache's ?hash= query.
    const fixtureIcon = /^\/fixture-icon\/([^/]+)\/([a-f\d]{6})\.svg$/i.exec(url.pathname);
    if (fixtureIcon) {
      const label = decodeURIComponent(fixtureIcon[1]).slice(0, 8)
        .replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;").replaceAll("'", "&apos;");
      res.setHeader("Content-Type", "image/svg+xml");
      res.end(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="16" fill="#${fixtureIcon[2]}"/><text x="32" y="43" text-anchor="middle" font-family="sans-serif" font-size="32" fill="white">${label}</text></svg>`);
      return;
    }
    if (url.pathname === "/") {
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(
        `<!doctype html><html lang="zh-CN"><head><title>MAC UI popover QA</title><style>body{font:14px 'Segoe UI',sans-serif;background:#263754;color:#fff;padding:20px}nav{display:flex;gap:24px;flex-wrap:wrap}a{color:#bcdcff}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(400px,1fr));gap:16px}iframe{border:0;width:400px;height:750px;border-radius:20px}h2{font-size:15px}</style></head><body><h1>MAC UI · ${panels.length} 个面板隔离测试</h1><p>全部为虚构数据，所有操作仅记录，不调用 Windows；Windows 原生磨砂不在网页中验收。</p><nav>${
          panels.map((p) => `<a href="/svelte/${p}/index.html">${p}</a>`).join("")
        }</nav><main>${
          panels.map((p) =>
            `<section><h2>${p}</h2><iframe title="${p}" src="/svelte/${p}/index.html${url.search}"></iframe></section>`
          ).join("")
        }</main></body></html>`,
      );
      return;
    }
    if (url.pathname === "/responsive") {
      const fixturePanel = url.searchParams.get("panel");
      if (!panels.includes(fixturePanel)) { res.writeHead(400).end(); return; }
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>Panel responsive QA</title>
      <style>body{margin:16px;background:#233247;color:white;font:14px system-ui}nav{display:flex;gap:8px;margin-bottom:12px}button{padding:8px 12px}iframe{display:block;border:0;width:700px;max-width:100%;height:700px}</style></head><body>
      <nav><button id="short">短屏 360 × 440</button><button id="wide">宽屏 700 × 700</button></nav>
      <iframe title="真实面板响应式验收" src="/svelte/${fixturePanel}/index.html?${url.searchParams.has("dark") ? "dark&" : ""}short&long"></iframe>
      <script>const frame=document.querySelector('iframe');const resize=(size)=>{frame.style.width=size[0]+'px';frame.style.height=size[1]+'px';frame.src=frame.src};document.getElementById('short').onclick=()=>resize([360,440]);document.getElementById('wide').onclick=()=>resize([700,700]);</script></body></html>`);
      return;
    }
    const panel = url.pathname.match(/^\/svelte\/([^/]+)\/index.html$/)?.[1];
    if (panels.includes(panel)) {
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(
        `<!doctype html><html lang="zh-CN" style="color-scheme:${
          url.searchParams.has("dark") ? "dark" : "light"
        }"><head><meta charset="utf-8"><title>${panel} · MAC UI isolated QA</title><link rel="stylesheet" href="./index.css"><style>${theme}
      :root{--spacing-xs:8px;--color-gray-25:#fcfcfc;--color-gray-50:#f4f4f4;--color-gray-75:#eee;--color-gray-100:#f3f4f6;--color-gray-200:#e5e7eb;--color-gray-300:#ccd1d8;--color-gray-400:#9ca3af;--color-gray-500:#6b7280;--color-gray-700:#374151;--color-gray-900:#111827;--system-accent-color:#086bdc;--system-accent-dark-color:#086bdc;--system-accent-light-color:#79b8ff;--system-accent-darker-color:#075fca;--system-accent-lighter-color:#9dcbff;--color-red-700:#b3283d}
      #root .mac-frosted-surface,#root .mac-power{color-scheme:${url.searchParams.has("dark") ? "dark" : "light"} !important}
      #fixture-wallpaper{position:fixed;inset:0;z-index:0;background:repeating-linear-gradient(0deg,transparent 0 32px,#ffffff40 32px 64px),linear-gradient(145deg,${url.searchParams.has("dark") ? "#18212d,#29374b" : "#d8e4f1,#b9cedb"})}html{min-height:100%}body{min-height:100%;margin:0}#root{position:relative;z-index:1}#fixture-status{position:relative;z-index:2;display:block;font:11px monospace;color:#fff;background:#172439;padding:8px;white-space:pre-wrap;max-width:380px;overflow-wrap:anywhere}
      </style></head><body><div id="fixture-wallpaper" aria-hidden="true"></div><div id="root"></div><output id="fixture-status">隔离测试 · 尚无原生操作</output><script>${mock}</script><script type="module" src="./index.js"></script></body></html>`,
      );
      return;
    }
    const icon = /^\/icons\/([A-Za-z0-9_-]+\.svg)$/.exec(url.pathname);
    const path = icon ? resolve(iconRoot, icon[1]) : resolve(dist, `.${decodeURIComponent(url.pathname)}`);
    if (!path.startsWith((icon ? iconRoot : dist) + sep)) {
      res.writeHead(403).end();
      return;
    }
    res.setHeader("Content-Type", mime[extname(path)] || "application/octet-stream");
    res.end(await readFile(path));
  } catch {
    res.writeHead(404).end("Not found");
  }
});
server.listen(port, "127.0.0.1", () => console.log(`MAC UI isolated QA: http://127.0.0.1:${port}/`));
