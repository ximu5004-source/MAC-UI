// Isolated UI acceptance fixture: real built frontend, no native commands or user data.
// Run after npm run build:ui: node scripts/qa/launchpad-server.mjs
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";

const dist = resolve(process.argv[2] || "dist");
const port = Number(process.argv[3] || 3581);
const files = new Map();
// This theme uses CSS-native nesting; its only Sass syntax is line comments.
const theme = (await readFile("src/static/themes/default/styles/apps-menu.scss", "utf8")).replace(/^\s*\/\/.*$/gm, "");
const mock = await readFile(new URL("./launchpad-mock.js", import.meta.url), "utf8");
const mime = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".yml": "text/plain" };
const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://127.0.0.1:${port}`);
    res.setHeader("Cache-Control", "no-store");
    const fixtureIcon = /^\/fixture-icons\/(\d+)\.svg$/.exec(url.pathname);
    if (fixtureIcon) {
      const index = Number(fixtureIcon[1]);
      const colors = ["#2679bb", "#b84570", "#36a371", "#9a61bf", "#ba8b31"];
      res.setHeader("Content-Type", "image/svg+xml");
      // Synthetic artwork only. Transparent cutouts prove object-fit/halo
      // regressions without downloading or using any real application assets.
      res.end(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="8" y="8" width="48" height="48" rx="12" fill="${colors[index % colors.length]}"/><circle cx="32" cy="32" r="12" fill="white"/><circle cx="32" cy="32" r="5" fill="${colors[index % colors.length]}"/></svg>`);
      return;
    }
    if (url.pathname === "/boundary") {
      // Same-origin frame: the checkerboard must show through every exterior
      // pixel. The child uses the real Launchpad CSS and default widget theme.
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>Launchpad boundary QA</title>
        <style>body{margin:0;padding:24px;background:repeating-conic-gradient(#fff 0 25%,#879cab 0 50%) 0/32px 32px;font:16px system-ui}
        nav{display:flex;gap:12px;margin-bottom:24px}button{padding:8px 16px}iframe{display:block;border:0;width:min(900px,95vw);height:560px;color-scheme:light}
        </style></head><body><nav><button id="light">浅色主题</button><button id="dark">深色主题</button><button id="compact">紧凑窗口</button><button id="wide">宽窗口</button></nav>
        <iframe title="启动台边缘" src="/boundary-panel?theme=light"></iframe>
        <script>const frame=document.querySelector('iframe');for(const id of ['light','dark'])document.getElementById(id).onclick=()=>{frame.style.colorScheme=id;frame.src='/boundary-panel?theme='+id};
        document.getElementById('compact').onclick=()=>frame.style.width='640px';document.getElementById('wide').onclick=()=>frame.style.width='900px';</script></body></html>`);
      return;
    }
    if (url.pathname === "/responsive") {
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>Launchpad responsive QA</title>
        <style>body{margin:16px;background:#233247;color:white;font:14px system-ui}nav{display:flex;gap:8px;margin-bottom:12px}button{padding:8px 12px}iframe{display:block;border:0;width:1100px;max-width:100%;height:680px}</style></head>
        <body><nav><button id="short">短屏 640 × 440</button><button id="wide">宽屏 1100 × 680</button></nav>
        <iframe title="真实启动台响应式验收" src="/svelte/apps-menu/index.html?many-folder&dark"></iframe>
        <script>const frame=document.querySelector('iframe');document.getElementById('short').onclick=()=>{frame.style.width='640px';frame.style.height='440px'};document.getElementById('wide').onclick=()=>{frame.style.width='1100px';frame.style.height='680px'};</script></body></html>`);
      return;
    }
    if (url.pathname === "/boundary-panel") {
      const isDark = url.searchParams.get("theme") === "dark";
      const surface = isDark ? "#20242b" : "#fafafa";
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><link rel="stylesheet" href="/svelte/apps-menu/index.css">
        <style>@layer theme-default-shared { :root{--slu-std-bg-color:${surface};--system-accent-color:#1677ff;--shadow-m:0 4px 12px #0008;color-scheme:${isDark ? "dark" : "light"}}body{background:var(--slu-std-bg-color)}}
        @layer theme-default {${theme}}</style></head><body><div id="root"><main class="apps-menu launchpad" data-fullscreen="false">
        <div class="launchpad-backdrop" aria-hidden="true"></div><header class="launchpad-header"><h1>启动台 · 边缘回归</h1></header>
        </main></div></body></html>`);
      return;
    }
    if (url.pathname === "/fixture-storage") {
      const key = url.searchParams.get("key");
      if (!/^(launchpad[\w-]*\.json|favorites\.json|index\.log)$/.test(key || "")) { res.writeHead(400).end(); return; }
      if (req.method === "POST") {
        let body = "";
        for await (const chunk of req) { body += chunk; if (body.length > 1_000_000) throw new Error("Fixture too large"); }
        files.set(key, body);
        res.end("ok");
      } else if (files.has(key)) res.end(files.get(key));
      else res.writeHead(404).end("Fixture not saved");
      return;
    }
    if (url.pathname === "/svelte/apps-menu/index.html") {
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>Launchpad isolated QA</title><link rel="stylesheet" href="./index.css"><style>${theme}
      body .apps-menu.launchpad{color-scheme:${url.searchParams.has("dark") ? "dark" : "light"}}
      #fixture-wallpaper{position:fixed;inset:0;z-index:0;background:repeating-linear-gradient(0deg,transparent 0 32px,#ffffff40 32px 64px),linear-gradient(145deg,${url.searchParams.has("dark") ? "#131c24,#273648" : "#8faccf,#688397 50%,#304e71"})}#root{position:relative;z-index:1}</style></head><body><div id="fixture-wallpaper" aria-hidden="true"></div><div id="root"></div><output id="fixture-status" style="position:fixed;left:4px;bottom:2px;z-index:10000;font:11px sans-serif;color:white;background:#233047;pointer-events:none">隔离测试 · 不读取或修改 Windows 文件</output><script>${mock}</script><script type="module" src="./index.js"></script></body></html>`);
      return;
    }
    if (/^\/icons\/[A-Za-z0-9_-]+\.svg$/.test(url.pathname)) {
      res.setHeader("Content-Type", "image/svg+xml");
      res.end(await readFile(resolve("dist", `.${url.pathname}`)));
      return;
    }
    const path = resolve(dist, `.${decodeURIComponent(url.pathname)}`);
    if (!path.startsWith(dist + sep)) { res.writeHead(403).end(); return; }
    res.setHeader("Content-Type", mime[extname(path)] || "application/octet-stream");
    res.end(await readFile(path));
  } catch { res.writeHead(404).end("Not found"); }
});
server.listen(port, "127.0.0.1", () => console.log(`Isolated Launchpad QA: http://127.0.0.1:${port}/svelte/apps-menu/index.html`));
