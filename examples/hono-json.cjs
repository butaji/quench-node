// hono-json.cjs — serve a JSON GET /json endpoint with hono, then fetch it.
const http = require("node:http");
const { Hono } = require("hono");

const app = new Hono();
app.get("/json", (c) => c.json({ hello: "quench-hono", count: 42 }));

// Mount the hono app on a real node:http server so the endpoint is served
// over actual HTTP rather than hono's in-memory dispatch. hono's fetch() and
// Hono#request() return a Response synchronously; only .text() is async.
let port;
const server = http.createServer((request, response) => {
  const result = app.fetch(new Request(`http://localhost:${port}${request.url}`));
  result.text().then((body) => {
    response.writeHead(result.status, { "content-type": "application/json" });
    response.end(body);
  });
});

server.listen(0, () => {
  port = server.address().port;
  http.get(`http://localhost:${port}/json`, (response) => {
    let body = "";
    response.setEncoding("utf8");
    response.on("data", (chunk) => (body += chunk));
    response.on("end", () => {
      console.log(body);
      server.close();
    });
  });
});