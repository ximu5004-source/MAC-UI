import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";

const fixtureRoot = resolve("target/frosted-panels-qa");
const iconRoot = resolve("dist/icons");
const mime = { ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".yml": "text/plain" };
createServer(async (req, res) => {
  try {
    const url = new URL(req.url, "http://127.0.0.1:3584");
    const icon = /^\/icons\/([A-Za-z0-9_-]+\.svg)$/.exec(url.pathname);
    const file = icon ? resolve(iconRoot, icon[1]) : resolve(fixtureRoot, `.${decodeURIComponent(url.pathname)}`);
    if (!file.startsWith((icon ? iconRoot : fixtureRoot) + sep)) { res.writeHead(403).end(); return; }
    res.setHeader("Cache-Control", "no-store");
    res.setHeader("Content-Type", mime[extname(file)] || "application/octet-stream");
    res.end(await readFile(file));
  } catch { res.writeHead(404).end("Not found"); }
}).listen(3584, "127.0.0.1", () => console.info("CSS/interaction QA only: http://127.0.0.1:3584/index.html"));
