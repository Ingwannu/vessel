// Question 3: what a phone's browser would get. The same build at 390x844
// CSS pixels, device pixel ratio 3, touch, an Android Chrome user agent
// (Chromium's device emulation: `(pointer: coarse)` matches, taps arrive
// as touch pointer events).
//
// What is checked: the layout at that width, the canvas's backing size,
// whether a tap on a field makes the platform ask for the on-screen
// keyboard (GPUI's web platform keeps a hidden textarea with
// `inputmode="none"` on touch screens and switches it to `text`, focused
// inside the tap, when the tap lands on a field: that is the only way a
// page can summon a phone's keyboard; headless Chromium draws no
// keyboard, so the attribute and the focus are what can be read), Hangul
// composed after the tap, and touch scrolling of a long conversation.
//
// Usage: node phone.mjs [port]

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
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 },
    // Headless Chromium's emulated device pixel ratio does not reach
    // `devicePixelContentBoxSize` (it stays in CSS pixels while
    // `devicePixelRatio` says 3), and GPUI's web platform divides the one
    // by the other: at an emulated 3 it lays out a 130-pixel-wide window.
    // A real phone reports both in device pixels. So the layout and touch
    // are checked at 1 (`DPR` to try another).
    deviceScaleFactor: Number(process.env.DPR ?? 1),
    isMobile: true,
    hasTouch: true,
    locale: "ko-KR",
    userAgent:
      "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/153.0.0.0 Mobile Safari/537.36",
  });
  const page = await context.newPage();
  const lines = recordConsole(page);
  await page.goto(server.url);
  note(`graphics: ${await waitForGraphics(lines)}`);
  await sleep(3000);
  const cdp = await context.newCDPSession(page);
  const shot = async (name) => {
    await sleep(500);
    await page.screenshot({ path: path.join(evidence, `${name}.png`) });
    note(`  capture ${name}.png`);
  };
  const state = () =>
    page.evaluate(() => {
      const area = document.querySelector("textarea");
      const canvas = document.querySelector("canvas");
      return JSON.stringify({
        coarse: matchMedia("(pointer: coarse)").matches,
        dpr: devicePixelRatio,
        canvas: canvas ? `${canvas.width}x${canvas.height}` : null,
        inputmode: area?.getAttribute("inputmode") ?? null,
        focused: document.activeElement === area,
        value: area?.value ?? null,
      });
    });

  note(`1. the first screen at 390x844, device pixel ratio ${process.env.DPR ?? 1}`);
  note(`  ${await state()}`);
  await shot("q3-phone-start");

  note("2. a tap outside any field (the sidebar's empty foot), then a tap on the composer");
  await page.touchscreen.tap(100, 600);
  await sleep(500);
  note(`  after a tap outside: ${await state()}`);
  await page.touchscreen.tap(320, 438);
  await sleep(500);
  note(`  after a tap on the field: ${await state()}`);

  note("3. Hangul composed after the tap: 휴대폰");
  const compose = async (text) => {
    await cdp.send("Input.imeSetComposition", { text, selectionStart: text.length, selectionEnd: text.length });
    await sleep(120);
  };
  for (const step of ["ㅎ", "휴"]) await compose(step);
  await cdp.send("Input.insertText", { text: "휴" });
  for (const step of ["ㄷ", "대"]) await compose(step);
  await cdp.send("Input.insertText", { text: "대" });
  for (const step of ["ㅍ", "포", "폰"]) await compose(step);
  await shot("q3-phone-composing");
  await cdp.send("Input.insertText", { text: "폰" });
  await sleep(300);
  note(`  after commit: ${await state()}`);
  await shot("q3-phone-typed");

  note("4. a long conversation (ci-fixes, 400 entries), scrolled by a touch drag");
  await page.touchscreen.tap(56, 279);
  await sleep(1500);
  await shot("q3-phone-long-before");
  const drag = async (fromY, toY) => {
    const point = (y) => [{ x: 300, y, id: 1 }];
    await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: point(fromY) });
    const steps = 12;
    for (let step = 1; step <= steps; step += 1) {
      const y = fromY + ((toY - fromY) * step) / steps;
      await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: point(y) });
      await sleep(16);
    }
    await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  };
  await drag(300, 700);
  await sleep(1200);
  await shot("q3-phone-long-dragged");
  note(`  after the drag: ${await state()}`);

  writeFileSync(path.join(evidence, "q3-phone-console.log"), `${lines.join("\n")}\n`);
  writeFileSync(path.join(evidence, "q3-phone-report.txt"), `${report.join("\n")}\n`);
} finally {
  await browser.close();
  server.stop();
}
