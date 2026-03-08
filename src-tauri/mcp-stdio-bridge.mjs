#!/usr/bin/env node
// Bridges MCP stdio transport (used by Claude Desktop) to Hathor Forge's HTTP MCP endpoint.
// Reads JSON-RPC messages from stdin (one per line), POSTs them to the HTTP server,
// and writes the response back to stdout.

import { createInterface } from "node:readline";
import { request } from "node:http";

const MCP_URL = process.argv[2] || "http://127.0.0.1:9876/mcp";
const parsed = new URL(MCP_URL);

let pending = 0;
let stdinClosed = false;

function maybeExit() {
  if (stdinClosed && pending === 0) process.exit(0);
}

function post(body) {
  return new Promise((resolve, reject) => {
    const req = request(
      {
        hostname: parsed.hostname,
        port: parsed.port,
        path: parsed.pathname,
        method: "POST",
        headers: { "Content-Type": "application/json" },
      },
      (res) => {
        // 204 No Content = notification, no response to forward
        if (res.statusCode === 204) return resolve(null);
        let data = "";
        res.on("data", (chunk) => (data += chunk));
        res.on("end", () => resolve(data));
      }
    );
    req.on("error", reject);
    req.write(body);
    req.end();
  });
}

const rl = createInterface({ input: process.stdin });

rl.on("line", async (line) => {
  if (!line.trim()) return;
  pending++;
  try {
    const response = await post(line);
    if (response) process.stdout.write(response + "\n");
  } catch (err) {
    try {
      const req = JSON.parse(line);
      const errorResponse = JSON.stringify({
        jsonrpc: "2.0",
        id: req.id ?? null,
        error: { code: -32000, message: `Bridge error: ${err.message}` },
      });
      process.stdout.write(errorResponse + "\n");
    } catch {
      // Can't parse the request
    }
  } finally {
    pending--;
    maybeExit();
  }
});

rl.on("close", () => {
  stdinClosed = true;
  maybeExit();
});
