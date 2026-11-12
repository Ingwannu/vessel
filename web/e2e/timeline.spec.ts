// The windowed timeline (track P) and find and the outline (track W)
// together, end to end: a conversation of 20,000 entries, of which the
// page draws only the rows near the view, and a match or a prompt
// thousands of rows away from them. Going there must draw the row, mark
// the match (or give the prompt focus), and keep it where it was put
// while the rows around it are measured and the spacers' estimates
// change (use-window.ts, window-pin.ts).
//
// Its own file, last in the run's order: the session stays on the fleet's
// board, ended, after it.

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { openUi } from "./open";
import { folder, promptAndWait, start, stop } from "./perf/sessions";

const NODE = "gpu-box";
const ENTRIES = 20_000;

/** Every frame from now on: where the find's current match is, relative
 * to the timeline's view (`null` while there is none). */
async function watchMatch(page: Page): Promise<void> {
  await page.evaluate(() => {
    const samples: (number | null)[] = [];
    Reflect.set(window, "__matchSamples", samples);
    const frame = () => {
      const scroller = document.querySelector(".timeline");
      const active = CSS.highlights.get("vessel-find-active");
      let top: number | null = null;
      if (scroller !== null && active !== undefined) {
        for (const range of active) {
          if (range instanceof Range && range.startContainer.isConnected) {
            top =
              range.getBoundingClientRect().top -
              scroller.getBoundingClientRect().top;
          }
        }
      }
      samples.push(top);
      if (samples.length < 100_000) {
        requestAnimationFrame(frame);
      }
    };
    requestAnimationFrame(frame);
  });
}

/** The samples `watchMatch` took since `from`. */
async function samples(page: Page, from: number): Promise<(number | null)[]> {
  return page.evaluate((start) => {
    const taken: unknown = Reflect.get(window, "__matchSamples");
    return Array.isArray(taken)
      ? taken
          .slice(start)
          .map((value: unknown) => (typeof value === "number" ? value : null))
      : [];
  }, from);
}

/** Find `query` (one match), and check it lands: the active mark is that
 * text, in the middle of the view, and from the first frame it was drawn
 * on to a second later it never moved by more than a pixel. */
async function findFar(page: Page, query: string): Promise<void> {
  const before = (await samples(page, 0)).length;
  const find = page.getByRole("textbox", { name: "Find in session" });
  await find.fill(query);
  await expect(
    page.getByRole("search", { name: "Find in session" }),
  ).toContainText("1/1");
  await expect
    .poll(() =>
      page.evaluate(() => {
        const active = CSS.highlights.get("vessel-find-active");
        const ranges = active === undefined ? [] : [...active];
        return ranges.map((range) =>
          range instanceof Range ? range.toString() : "",
        );
      }),
    )
    .toEqual([query]);
  // A second of frames after landing.
  await page.waitForTimeout(1_000);
  const seen = (await samples(page, before)).filter(
    (value): value is number => value !== null,
  );
  expect(seen.length).toBeGreaterThan(10);
  const view = await page.locator(".timeline").evaluate((element) => ({
    height: element.clientHeight,
  }));
  const first = seen[0] ?? Number.NaN;
  for (const top of seen) {
    expect(Math.abs(top - first)).toBeLessThanOrEqual(1);
  }
  // In the middle of the view, a line's height either way.
  expect(Math.abs(first - view.height / 2)).toBeLessThan(40);
  // The window still draws only what is near the view.
  expect(await page.locator(".timeline .t-row").count()).toBeLessThan(400);
}

test("find and the outline go to rows thousands of entries away and hold them there, folded or not", async ({
  page,
}) => {
  test.setTimeout(900_000);
  const env = loadEnv();
  const id = await start(env, NODE, folder(env, "far-find"), "structured");
  const origin = await openUi(page);
  let collapsed = false;
  try {
    await promptAndWait(env, NODE, id, `entries ${String(ENTRIES)}`, 600_000);
    await page.goto(`${origin}/sessions`);
    const card = page.locator(`li.card[data-session="${id}"] > a.card-open`);
    await expect(card).toBeVisible();
    await card.click();
    const timeline = page.getByRole("region", { name: "Timeline" });
    // A second prompt, so the outline has two marks: one far above.
    const text = page.getByRole("textbox", { name: "Prompt" });
    await expect(timeline.getByText("Turn ended")).toBeVisible({
      timeout: 120_000,
    });
    await text.fill("plain");
    await text.press("Enter");
    await expect(
      timeline.getByText("echo: plain", { exact: true }),
    ).toBeVisible({ timeout: 60_000 });
    await watchMatch(page);
    await text.press("Control+f");
    // Far above (the view is at the end), then far below.
    await findFar(page, "Entry 300.");
    await findFar(page, "Entry 15000.");
    // A scroll that is not the pin's (a script, a screen reader moving
    // to an element) is the reader's: the held row lets go of the view.
    const left = await page.locator(".timeline").evaluate(async (element) => {
      element.scrollTop = 4_000;
      for (let frame = 0; frame < 10; frame += 1) {
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      return element.scrollTop;
    });
    // Near where the script put it (the browser's anchoring may move it
    // as the rows there are measured), far from the match it held.
    expect(left).toBeLessThan(50_000);
    // Find keeps focus in its field.
    await expect(
      page.getByRole("textbox", { name: "Find in session" }),
    ).toBeFocused();
    await page.keyboard.press("Escape");
    // The outline's first mark: the prompt 20,000 entries above, at the
    // view's top, with focus.
    await page.getByRole("button", { name: /^Jump to prompt 1 of 2/ }).click();
    const prompt = timeline.locator("[data-kind='prompt']").first();
    await expect(prompt).toBeFocused();
    await expect(prompt).toContainText(`entries ${String(ENTRIES)}`);
    const placed = await prompt.evaluate((element) => {
      const scroller = element.closest(".timeline");
      return scroller === null
        ? Number.NaN
        : element.getBoundingClientRect().top -
            scroller.getBoundingClientRect().top;
    });
    expect(Math.abs(placed - 16)).toBeLessThanOrEqual(1);
    // With tool calls collapsed, a run of a thought and a tool call is one
    // row: the window's rows are fewer than the entries, and find's keys
    // still name rows.
    await page.goto(`${origin}/settings/appearance`);
    await page.getByRole("combobox", { name: "Conversation display" }).click();
    await page.getByRole("option", { name: "Collapse tool calls" }).click();
    collapsed = true;
    await page.goBack();
    await expect(
      timeline.getByText("echo: plain", { exact: true }),
    ).toBeVisible({
      timeout: 120_000,
    });
    await expect(
      timeline.getByRole("button", { name: "Read 1 file" }).first(),
    ).toBeVisible();
    await watchMatch(page);
    await page.getByRole("textbox", { name: "Prompt" }).press("Control+f");
    await findFar(page, "Entry 900.");
    await findFar(page, "Entry 18000.");
  } finally {
    await stop(env, NODE, [id]);
    if (collapsed) {
      // The fleet's one `vessel ui` keeps preferences for the whole run.
      await page.goto(`${origin}/settings/appearance`);
      await page
        .getByRole("combobox", { name: "Conversation display" })
        .click();
      await page.getByRole("option", { name: "Full", exact: true }).click();
    }
  }
});
