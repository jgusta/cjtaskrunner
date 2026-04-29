import http from "node:http";
import { renderPage } from "./render.js";

const host = process.env.SSR_HOST || "127.0.0.1";
const port = Number(process.env.SSR_PORT || "3000");

const server = http.createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(renderPage());
});

server.listen(port, host, () => {
  console.log(`SSR example listening at http://${host}:${port}`);
});
