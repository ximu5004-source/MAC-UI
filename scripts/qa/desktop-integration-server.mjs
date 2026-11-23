// Serve actual built widgets with a browser-only native bridge. Run npm run build:ui first.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, resolve, sep } from "node:path";

const port = Number(process.argv[2] || 3592);
const root = resolve("dist");
const panels = ["desktop-shell", "notifications"];
const mockUrl = new URL("./desktop-integration-mock.js", import.meta.url);
const mime = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".yml": "text/plain; charset=utf-8" };
createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://127.0.0.1:${port}`);
    res.setHeader("Cache-Control", "no-store");
    const icon = /^\/fixture-icons\/(file|folder|pdf|txt|png|exe)\.svg$/.exec(url.pathname)?.[1];
    if (icon) {
      const colors = { file: "#e8eff7", folder: "#65baff", pdf: "#fff2ee", txt: "#fff", png: "#edfff4", exe: "#7ca9ed" };
      res.setHeader("Content-Type", "image/svg+xml");
      res.end(`<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">${icon === "folder"
        ? '<path d="M4 18a5 5 0 0 1 5-5h18l7 7h21a5 5 0 0 1 5 5v28H4z" fill="#65baff"/><path d="M4 25h56v28H4z" fill="#91ceff"/>'
        : `<path d="M14 4h25l12 12v44H14z" fill="${colors[icon]}"/><path d="M39 4v12h12" fill="#a5b4c8"/><path d="M22 30h21M22 37h21M22 44h14" stroke="#7690ae" stroke-width="3"/><text x="32" y="56" text-anchor="middle" font-family="sans-serif" font-size="8" fill="#244b79">${icon.toUpperCase()}</text>`}</svg>`);
      return;
    }
    if (url.pathname === "/") {
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<h1>桌面集成隔离验收</h1><p>虚构文件与通知；所有系统调用已拦截。</p>${panels.map((panel) => `<p><a href="/svelte/${panel}/index.html">${panel}</a></p>`).join("")}`);
      return;
    }
    const panel = /^\/svelte\/([^/]+)\/index.html$/.exec(url.pathname)?.[1];
    if (panels.includes(panel)) {
      const mock = await readFile(mockUrl, "utf8");
      res.setHeader("Content-Type", "text/html; charset=utf-8");
      res.end(`<!doctype html><html lang="zh-CN" data-color-scheme="${url.searchParams.has("dark") ? "dark" : "light"}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${panel} 隔离验收</title><link rel="stylesheet" href="./index.css"><style>
      :root{font:14px 'Segoe UI',sans-serif;--color-persist-white:#fff;--color-gray-300:#b9c8d9;--color-accent:#086bdc;--mac-accent:#086bdc;--mac-panel-text:#18253a;--mac-panel-surface:#ffffffdd}
      html{background:${url.searchParams.has("light-wallpaper") ? "linear-gradient(120deg,#eef3fa,#dce5ef)" : "linear-gradient(120deg,#4c7494,#303f6a)"}!important}body{margin:0}
      #fixture-status{position:fixed;bottom:0;left:0;z-index:9999;max-width:95vw;background:#152035;color:#fff;font:11px monospace;padding:4px;pointer-events:none;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
      #fixture-menu{position:fixed;right:20px;top:20px;z-index:10000;display:grid;max-height:85vh;overflow:auto;padding:8px;background:#fff;box-shadow:0 8px 40px #0005;border-radius:12px}#fixture-menu button{text-align:left;border:0;background:transparent;padding:6px 16px}#fixture-menu button:hover{background:#dcecff}
      </style></head><body><div id="root"></div><output id="fixture-status" aria-live="off">仅使用虚构数据</output><script>${mock}</script><script type="module" src="./index.js"></script></body></html>`);
      return;
    }
    const path = resolve(root, `.${decodeURIComponent(url.pathname)}`);
    if (!path.startsWith(root + sep)) { res.writeHead(403).end(); return; }
    res.setHeader("Content-Type", mime[extname(path)] || "application/octet-stream");
    res.end(await readFile(path));
  } catch { res.writeHead(404).end("Not found"); }
}).listen(port, "127.0.0.1", () => console.log(`Integration fixture: http://127.0.0.1:${port}/`));
