// Questions 4, 5 and 6 of docs/plans/backend-m12-browser.md:
//   4. what the accessibility tree holds (CDP Accessibility.getFullAXTree);
//   5. bytes a browser downloads (raw, gzip, brotli), time to the first
//      frame on a cold load (the app marks `vessel-first-frame`), memory
//      after it (the module's linear memory, the JavaScript heap);
//   6. what Chromium does with WebGPU off (the platform's WebGL2
//      fallback), and with WebGL off as well.
//
// Usage: node start.mjs [port] [runs]

import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { dist, evidence, launch, recordConsole, serve, sleep, waitForGraphics } from "./lib.mjs";

const port = Number(process.argv[2] ?? 18431);
const runs = Number(process.argv[3] ?? 3);
const report = [];
const note = (line) => {
  report.push(line);
  console.log(line);
};

note("5a. what a browser downloads (raw / gzip -9 / brotli 11)");
let total = [0, 0, 0];
for (const name of readdirSync(dist).sort()) {
  if (name.endsWith(".txt")) continue;
  const bytes = readFileSync(path.join(dist, name));
  const gz = gzipSync(bytes, { level: 9 }).length;
  const br = brotliCompressSync(bytes, {
    params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: bytes.length },
  }).length;
  total = [total[0] + statSync(path.join(dist, name)).size, total[1] + gz, total[2] + br];
  note(`  ${name}: ${bytes.length} / ${gz} / ${br}`);
}
note(`  total: ${total.join(" / ")}`);

const server = await serve(port);
try {
  const browser = await launch({ webgpu: true });
  try {
    note(`5b. cold loads (a new browser context each: nothing cached), ${runs} runs, 1280x800`);
    for (let run = 1; run <= runs; run += 1) {
      const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
      const page = await context.newPage();
      const lines = recordConsole(page);
      const cdp = await context.newCDPSession(page);
      await cdp.send("Performance.enable");
      await page.goto(server.url);
      const graphics = await waitForGraphics(lines);
      const firstFrame = await page.waitForFunction(
        () => performance.getEntriesByName("vessel-first-frame")[0]?.startTime ?? false,
        null,
        { timeout: 60000 },
      ).then((handle) => handle.jsonValue());
      const timing = await page.evaluate(() => {
        const wasm = performance.getEntriesByType("resource").find((entry) => entry.name.endsWith(".wasm"));
        return {
          wasmFetched: wasm ? Math.round(wasm.responseEnd) : null,
          domContentLoaded: Math.round(performance.getEntriesByType("navigation")[0].domContentLoadedEventEnd),
        };
      });
      await sleep(3000);
      const memory = await page.evaluate(() => window.vesselWasm?.memory?.buffer.byteLength ?? null);
      const metrics = Object.fromEntries(
        (await cdp.send("Performance.getMetrics")).metrics.map((metric) => [metric.name, metric.value]),
      );
      note(
        `  run ${run}: graphics ${graphics}; wasm fetched at ${timing.wasmFetched} ms; first frame at ${Math.round(firstFrame)} ms; ` +
          `wasm memory ${memory} bytes; JS heap used ${metrics.JSHeapUsedSize} bytes; nodes ${metrics.Nodes}`,
      );
      if (run === 1) {
        note("4. the accessibility tree (CDP Accessibility.getFullAXTree)");
        const { nodes } = await cdp.send("Accessibility.getFullAXTree");
        note(`  nodes: ${nodes.length}`);
        for (const node of nodes) {
          if (node.ignored) continue;
          const role = node.role?.value;
          const name = node.name?.value ?? "";
          note(`  - ${role} ${JSON.stringify(name)}`);
        }
        writeFileSync(path.join(evidence, "q4-ax-tree.json"), JSON.stringify(nodes, null, 1));
      }
      await context.close();
    }
  } finally {
    await browser.close();
  }

  for (const [label, options] of [
    ["6a. WebGPU off (--disable-features=WebGPU)", { webgpu: false }],
    ["6b. WebGPU and WebGL off (--disable-webgl as well)", { webgpu: false, extra: ["--disable-webgl", "--disable-3d-apis"] }],
  ]) {
    note(label);
    const browser = await launch(options);
    try {
      const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
      const lines = recordConsole(page);
      await page.goto(server.url);
      const graphics = await waitForGraphics(lines, 45000);
      await sleep(3000);
      const probe = await page.evaluate(() => ({
        gpu: "gpu" in navigator,
        webgl2: Boolean(document.createElement("canvas").getContext("webgl2")),
        text: document.body.innerText.slice(0, 300),
      }));
      note(`  graphics: ${graphics}; navigator.gpu: ${probe.gpu}; webgl2: ${probe.webgl2}`);
      for (const line of lines.filter((entry) => /graphics|WebGPU|WebGL|panicked|pageerror/i.test(entry))) {
        note(`  console: ${line.slice(0, 300)}`);
      }
      if (probe.text) note(`  page text: ${JSON.stringify(probe.text)}`);
      const name = options.extra ? "q6-no-webgl" : "q6-webgl2";
      await page.screenshot({ path: path.join(evidence, `${name}.png`) });
      note(`  capture ${name}.png`);
    } finally {
      await browser.close();
    }
  }
} finally {
  server.stop();
}
writeFileSync(path.join(evidence, "q4-q6-report.txt"), `${report.join("\n")}\n`);
