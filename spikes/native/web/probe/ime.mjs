// Question 2, the part that decides most: Hangul through an input method,
// in the new-session composer (the screen the app opens on), then in a
// session's composer.
//
// The composition is driven through the DevTools protocol's
// `Input.imeSetComposition` (each step of the syllable being composed, as
// a Korean IME reports it) and `Input.insertText` (the commit): Chromium
// then fires the same composition and input events on the focused element
// as a real IME does. That is how it is driven here; no IME runs.
//
// Read back after each step: the screen (captures), and the hidden
// textarea GPUI's web platform keeps as the IME's conduit, which it
// mirrors from the field's own text after every commit.
//
// Usage: node ime.mjs [port]

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
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: "ko-KR" });
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
  const mirror = () =>
    page.evaluate(() => {
      const area = document.querySelector("textarea");
      if (!area) return "no textarea";
      return JSON.stringify({
        value: area.value,
        selection: [area.selectionStart, area.selectionEnd],
        focused: document.activeElement === area,
      });
    });
  const compose = async (text) => {
    await cdp.send("Input.imeSetComposition", {
      text,
      selectionStart: text.length,
      selectionEnd: text.length,
    });
    await sleep(120);
  };
  const commit = async (text) => {
    await cdp.send("Input.insertText", { text });
    await sleep(200);
  };

  note("1. the new-session composer (focused at start, as the desktop's)");
  note(`  mirror at start: ${await mirror()}`);
  await shot("q2-start-ko");

  note("2. Latin typed as key events (Playwright keyboard.type)");
  await page.mouse.click(760, 425);
  await page.keyboard.type("Run tests ", { delay: 30 });
  note(`  mirror: ${await mirror()}`);

  note("3. Hangul composed: ㅎ 하 한 (commit 한), ㄱ 그 글 (commit 글)");
  for (const step of ["ㅎ", "하", "한"]) await compose(step);
  await shot("q2-ime-composing-han");
  note(`  mirror while composing: ${await mirror()}`);
  await commit("한");
  for (const step of ["ㄱ", "그", "글"]) await compose(step);
  await shot("q2-ime-composing-geul");
  await commit("글");
  note(`  mirror after commit: ${await mirror()}`);

  note("4. a space (key event), then 하나 with the carry-over: 한 + ㅏ commits 하, composes 나");
  await page.keyboard.type(" ");
  for (const step of ["ㅎ", "하", "한"]) await compose(step);
  await commit("하");
  await compose("나");
  await commit("나");
  note(`  mirror: ${await mirror()}`);

  note("5. a composition cancelled (ㅋ, then emptied): nothing should be inserted");
  await compose("ㅋ");
  await compose("");
  await commit("");
  note(`  mirror: ${await mirror()}`);
  await shot("q2-ime-committed");

  note("6. Enter sends: the conversation starts with the prompt as typed");
  await page.keyboard.press("Enter");
  await sleep(1500);
  await shot("q2-sent-session");
  note(`  mirror after send: ${await mirror()}`);

  note("7. the session's composer: compose 안녕 and send");
  await page.mouse.click(730, 743);
  for (const step of ["ㅇ", "아", "안"]) await compose(step);
  await commit("안");
  for (const step of ["ㄴ", "녀", "녕"]) await compose(step);
  await shot("q2-session-composing");
  await commit("녕");
  note(`  mirror: ${await mirror()}`);
  await page.keyboard.press("Enter");
  await sleep(1200);
  await shot("q2-session-echo");

  writeFileSync(path.join(evidence, "q2-ime-console.log"), `${lines.join("\n")}\n`);
  writeFileSync(path.join(evidence, "q2-ime-report.txt"), `${report.join("\n")}\n`);
} finally {
  await browser.close();
  server.stop();
}
