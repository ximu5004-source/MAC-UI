// Repository-local synthetic mesh preview. No Tauri/native API, real capture,
// arbitrary filesystem route, OS preference, user data, or automatic motion.
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";

const port = Number(process.argv[2] || 3587);
if (!Number.isSafeInteger(port) || port < 1024 || port > 65535) throw new Error("Invalid preview port");
const data = await readFile(new URL("../../target/qa/genie-geometry.json", import.meta.url));
const html = await readFile(new URL("./genie-preview.html", import.meta.url));
createServer((request, response) => {
  const path = new URL(request.url, `http://127.0.0.1:${port}`).pathname;
  response.setHeader("Cache-Control", "no-store");
  if (path === "/") {
    response.setHeader("Content-Type", "text/html; charset=utf-8");
    response.end(html);
  } else if (path === "/fixture.json") {
    response.setHeader("Content-Type", "application/json; charset=utf-8");
    response.end(data);
  } else {
    response.writeHead(404).end("Not found");
  }
}).listen(port, "127.0.0.1", () => console.log(`Synthetic Genie geometry preview: http://127.0.0.1:${port}/`));
