// Question 2, the rest of text input in the new-session composer:
// selection by keyboard and by mouse, copy, cut and paste, undo, the
// caret, a long line, and what the browser's own find, right-click, zoom
// and autofill do with a canvas.
//
// Read back through the hidden textarea GPUI's web platform mirrors the
// field into (value and selection), the system clipboard (Playwright
// grants this page clipboard access), and captures.
//
// Usage: node edit.mjs [port]

import { writeFileSync } from "node:fs";
import path from "node:path";
import { evidence, launch, recordConsole, serve, sleep, waitForGraphics } from "./lib.mjs";

const port = Number(process.argv[2] ?? 18431);
const server = await serve(port);
const browser = await launch({ webgpu: true });
const report = [];
const note = (line) => {
  report.push(line);
  console.log(line);
};
try {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: server.url });
  const page = await context.newPage();
  const lines = recordConsole(page);
  await page.goto(server.url);
  note(`graphics: ${await waitForGraphics(lines)}`);
  await sleep(2500);
  const cdp = await context.newCDPSession(page);
  const shot = async (name) => {
    await sleep(400);
    await page.screenshot({ path: path.join(evidence, `${name}.png`) });
    note(`  capture ${name}.png`);
  };
  const mirror = async () => {
    // A key that moves nothing makes the platform re-sync its mirror.
    await sleep(150);
    return page.evaluate(() => {
      const area = document.querySelector("textarea");
      return area
        ? JSON.stringify({ value: area.value, selection: [area.selectionStart, area.selectionEnd] })
        : "no textarea";
    });
  };
  const clipboard = () => page.evaluate(() => navigator.clipboard.readText().catch((error) => `read failed: ${error}`));
  const setClipboard = (text) => page.evaluate((words) => navigator.clipboard.writeText(words), text);
  const field = { x: 760, y: 425 };

  note("1. typed text, then the caret: Home, three steps right");
  await page.mouse.click(field.x, field.y);
  await page.keyboard.type("hello 세상 world", { delay: 20 });
  await page.keyboard.press("Home");
  for (let step = 0; step < 3; step += 1) await page.keyboard.press("ArrowRight");
  note(`  mirror: ${await mirror()}`);
  await shot("q2-caret");

  note("2. selection by keyboard: Shift+End");
  await page.keyboard.press("Shift+End");
  note(`  mirror: ${await mirror()}`);
  await shot("q2-select-keyboard");

  note("3. copy (Ctrl+C) of the selection: the clipboard holds");
  await setClipboard("(before)");
  await page.keyboard.press("Control+c");
  await sleep(300);
  note(`  clipboard: ${JSON.stringify(await clipboard())}`);

  note("4. cut (Ctrl+X): the field and the clipboard");
  await page.keyboard.press("Control+x");
  await sleep(300);
  note(`  clipboard: ${JSON.stringify(await clipboard())}`);
  note(`  mirror: ${await mirror()}`);

  note("5. undo (Ctrl+Z), then redo (Ctrl+Shift+Z)");
  await page.keyboard.press("Control+z");
  note(`  after undo: ${await mirror()}`);
  await page.keyboard.press("Control+Shift+z");
  note(`  after redo: ${await mirror()}`);

  note("6. paste by keyboard (Ctrl+V) of 붙여넣기 from the system clipboard");
  await setClipboard("붙여넣기");
  await page.keyboard.press("End");
  await page.keyboard.press("Control+v");
  await sleep(400);
  note(`  mirror: ${await mirror()}`);

  note("7. paste as the browser's menu delivers it: a paste event on the focused element");
  await page.evaluate(() => {
    const data = new DataTransfer();
    data.setData("text/plain", "[메뉴 붙여넣기]");
    const event = new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true });
    document.activeElement?.dispatchEvent(event);
  });
  await sleep(300);
  note(`  mirror: ${await mirror()}`);
  await shot("q2-pasted");

  note("8. selection by mouse: a drag across the start of the field");
  await page.mouse.move(field.x - 345, field.y);
  await page.mouse.down();
  await page.mouse.move(field.x - 250, field.y, { steps: 8 });
  await page.mouse.up();
  note(`  mirror: ${await mirror()}`);
  await shot("q2-select-mouse");
  note("   double click selects a word:");
  await page.mouse.dblclick(field.x - 330, field.y);
  note(`  mirror: ${await mirror()}`);

  note("9. Ctrl+A then a long line (300 characters, Hangul and Latin)");
  await page.keyboard.press("Control+a");
  const long = "긴 문장을 한 줄에 계속 이어서 씁니다 and keeps going in Latin too, ".repeat(6);
  await cdp.send("Input.insertText", { text: long });
  await sleep(300);
  note(`  mirror length: ${JSON.parse(await mirror()).value.length}`);
  await shot("q2-long-line");

  note("10. the browser's find: window.find() over the page's text");
  const found = await page.evaluate(() => ({
    composer: window.find("긴 문장"),
    sidebar: window.find("vessel-client"),
    bodyText: document.body.innerText.slice(0, 80),
  }));
  note(`  ${JSON.stringify(found)}`);

  note("11. right-click on the field: what appears");
  const before = await page.evaluate(() => document.body.children.length);
  await page.mouse.click(field.x, field.y, { button: "right" });
  await sleep(500);
  const after = await page.evaluate(() => document.body.children.length);
  note(`  body children before/after: ${before}/${after}`);
  await shot("q2-right-click");
  await page.keyboard.press("Escape");

  note("12. zoom: the page's device pixel ratio changed to 2 (as Ctrl+= does)");
  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: 1280,
    height: 800,
    deviceScaleFactor: 2,
    mobile: false,
  });
  await sleep(1500);
  note(`  devicePixelRatio: ${await page.evaluate(() => window.devicePixelRatio)}`);
  const canvas = await page.evaluate(() => {
    const element = document.querySelector("canvas");
    return element ? `${element.width}x${element.height}` : "no canvas";
  });
  note(`  canvas backing size: ${canvas}`);
  await shot("q2-zoom-2x");
  await cdp.send("Emulation.clearDeviceMetricsOverride");

  note("13. autofill and password managers: the form fields on the page");
  const fields = await page.evaluate(() =>
    [...document.querySelectorAll("input, textarea, select, form")].map((element) => ({
      tag: element.tagName,
      type: element.getAttribute("type"),
      autocomplete: element.getAttribute("autocomplete"),
      style: element.getAttribute("style"),
    })),
  );
  note(`  ${JSON.stringify(fields)}`);

  writeFileSync(path.join(evidence, "q2-edit-console.log"), `${lines.join("\n")}\n`);
  writeFileSync(path.join(evidence, "q2-edit-report.txt"), `${report.join("\n")}\n`);
} finally {
  await browser.close();
  server.stop();
}
