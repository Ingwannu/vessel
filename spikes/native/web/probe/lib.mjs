// What every probe of the browser build shares: a static server for the
// build's output on a port of its own, headless Chromium with or without
// WebGPU, and the page's console kept for the notes
// (docs/plans/backend-m12-browser.md).
//
// Playwright comes from the page's own packages (`npm ci` in web/): the
// probes add no dependency. Chromium is Playwright's; WebGPU in a headless
// browser on a machine without a GPU is SwiftShader's (a software
// adapter), so a probe can say whether it draws, not how fast a GPU does.

import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../../../..");
const require = createRequire(path.join(repo, "web/package.json"));
const { chromium, devices } = require("@playwright/test");

export { devices };

/** Where captures and logs go: `$EVIDENCE`, else a scratch directory. */
export const evidence = process.env.EVIDENCE ?? "/tmp/vessel-browser-evidence";
mkdirSync(evidence, { recursive: true });

/** The build's output: `$DIST`, else `spikes/native/web/dist`. */
export const dist = process.env.DIST ?? path.resolve(here, "../dist");

/** Serve `dist` on `port` (loopback only) until `stop()`. */
export async function serve(port) {
  const server = spawn(
    "python3",
    ["-m", "http.server", String(port), "--bind", "127.0.0.1", "--directory", dist],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  for (let tries = 0; tries < 100; tries += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/index.html`);
      if (response.ok) break;
    } catch {
      // not listening yet
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return { url: `http://127.0.0.1:${port}/`, stop: () => server.kill() };
}

/**
 * Headless Chromium. `webgpu: false` turns WebGPU off (the platform then
 * tries WebGL2); `true` asks for SwiftShader's software adapter.
 */
export async function launch({ webgpu = true, executablePath, extra = [] } = {}) {
  const args = webgpu
    ? [
        "--enable-unsafe-webgpu",
        "--enable-features=Vulkan,WebGPU",
        "--use-vulkan=swiftshader",
        "--use-webgpu-adapter=swiftshader",
        "--use-angle=swiftshader",
        "--enable-unsafe-swiftshader",
      ]
    : ["--disable-features=WebGPU", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"];
  return chromium.launch({ headless: true, args: [...args, ...extra], executablePath });
}

/** Keep the page's console and errors, in order, for the notes. */
export function recordConsole(page) {
  const lines = [];
  page.on("console", (message) => lines.push(`[${message.type()}] ${message.text()}`));
  page.on("pageerror", (error) => lines.push(`[pageerror] ${error.message}`));
  return lines;
}

/** Wait until the page logs that graphics started (or failed), or `ms`. */
export async function waitForGraphics(lines, ms = 60000) {
  const start = Date.now();
  while (Date.now() - start < ms) {
    if (lines.some((line) => line.includes("Browser graphics initialized"))) return "ok";
    if (lines.some((line) => line.includes("Failed to initialize browser graphics"))) return "failed";
    if (lines.some((line) => line.startsWith("[pageerror]"))) return "error";
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  return "timeout";
}

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
