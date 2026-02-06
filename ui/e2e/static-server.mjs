import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import http from "node:http";
import path from "node:path";

const root = process.cwd();
const host = "127.0.0.1";
const port = Number(process.env.UI_PORT || 5173);

const MIME = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".ico": "image/x-icon",
  ".jpg": "image/jpeg",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".txt": "text/plain; charset=utf-8",
  ".webp": "image/webp",
};

function normalizePath(urlPath) {
  const safe = decodeURIComponent(urlPath.split("?")[0]);
  const clean = safe === "/" ? "/index.html" : safe;
  const abs = path.resolve(root, `.${clean}`);
  if (!abs.startsWith(root)) return null;
  return abs;
}

const server = http.createServer(async (req, res) => {
  const filePath = normalizePath(req.url || "/");
  if (!filePath) {
    res.writeHead(403);
    res.end("Forbidden");
    return;
  }

  try {
    const info = await stat(filePath);
    const finalPath = info.isDirectory() ? path.join(filePath, "index.html") : filePath;
    const ext = path.extname(finalPath).toLowerCase();
    const mime = MIME[ext] || "application/octet-stream";

    res.writeHead(200, { "Content-Type": mime });
    createReadStream(finalPath).pipe(res);
  } catch {
    res.writeHead(404);
    res.end("Not Found");
  }
});

server.listen(port, host, () => {
  // eslint-disable-next-line no-console
  console.log(`[ui-static] http://${host}:${port}`);
});

const shutdown = () => {
  server.close(() => process.exit(0));
};
process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);
