// Question 1: does the app's own UI draw in a browser. Loads the build,
// waits for GPUI's graphics to start, captures the board, and writes the
// console to the evidence directory.
//
// Usage: node draw.mjs [port]   (DIST, EVIDENCE: see lib.mjs)

import { writeFileSync } from "node:fs";
import path from "node:path";
import { evidence, launch, recordConsole, serve, sleep, waitForGraphics } from "./lib.mjs";

const port = Number(process.argv[2] ?? 18431);
const server = await serve(port);
const browser = await launch({ webgpu: true, executablePath: process.env.CHROME });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const lines = recordConsole(page);
  await page.goto(server.url);
  const adapter = await page.evaluate(async () => {
    if (!("gpu" in navigator)) return "navigator.gpu missing";
    const found = await navigator.gpu.requestAdapter();
    if (!found) return "no adapter";
    const info = found.info ?? {};
    return `adapter: ${info.vendor ?? "?"} ${info.architecture ?? "?"} ${info.description ?? ""}`;
  });
  lines.push(`[probe] ${adapter}`);
  const graphics = await waitForGraphics(lines);
  lines.push(`[probe] graphics: ${graphics}`);
  await sleep(3000);
  await page.screenshot({ path: path.join(evidence, "q1-board-light.png") });
  writeFileSync(path.join(evidence, "q1-draw-console.log"), `${lines.join("\n")}\n`);
  console.log(lines.join("\n"));
} finally {
  await browser.close();
  server.stop();
}
