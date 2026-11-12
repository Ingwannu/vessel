// Measurement 5 (perf.ts): find in a long session, and going to a match
// thousands of entries from the view (milestone 11: track W's find over
// track P's window, docs/PERFORMANCE.md).
//
// The sessions are the timeline measurement's (`entries <n>` to
// `fake-acp`: a message, a thought and a tool call in turn). The page
// opens at the end of the log; then, in one find bar:
//
// 1. `Entry 300.`: one match, about 300 entries from the top. Its first
//    search folds every entry's text (find.ts keeps each entry's folded
//    text from then on), and going there is a jump of the whole log.
// 2. `short paragraph`: every message, the search over text already
//    folded; its first match is the first message.
// 3. `Entry <n × 0.9>`: one match far below.
//
// Each is timed from the field's `input` event (the probe's mark) to the
// first frame showing it: the count, then the match marked inside the
// view. The field waits 100 ms after typing stops before searching
// (find-bar.tsx's settle), which every figure includes.

import type { Browser, Page } from "@playwright/test";
import { expect } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { openUi } from "../open";
import { DESKTOP } from "./conditions";
import { installProbe, readMarks, waitForMark } from "./probe";
import { loadNow, runs, type Section, type Table } from "./report";
import { folder, promptAndWait, start, stop } from "./sessions";
import {
  type LongTasks,
  longTasks,
  type Metric,
  rows,
  taskMetrics,
} from "./stats";

const SIZES: readonly number[] = [5_000, 20_000];

const NODE = "gpu-box";

/** Frames the match's place is read for once it is shown. */
const HOLD_FRAMES = 60;

interface Finding {
  readonly firstCountMs: number;
  readonly farAboveMs: number;
  readonly farAboveDriftPx: number;
  readonly repeatCountMs: number;
  readonly farBelowMs: number;
  readonly farBelowDriftPx: number;
  readonly tasks: LongTasks;
  readonly matches: number;
}

const METRICS: readonly Metric<Finding>[] = [
  {
    name: "first search: typed to the count",
    unit: "ms",
    of: (run) => run.firstCountMs,
  },
  {
    name: "a match far above: typed to it shown",
    unit: "ms",
    of: (run) => run.farAboveMs,
  },
  {
    name: "… how far it moved in the 60 frames after",
    unit: "px",
    of: (run) => run.farAboveDriftPx,
  },
  {
    name: "a search again, every message: typed to the count",
    unit: "ms",
    of: (run) => run.repeatCountMs,
  },
  {
    name: "a match far below: typed to it shown",
    unit: "ms",
    of: (run) => run.farBelowMs,
  },
  {
    name: "… how far it moved in the 60 frames after",
    unit: "px",
    of: (run) => run.farBelowDriftPx,
  },
  ...taskMetrics<Finding>("the three finds", (run) => run.tasks),
  { name: "matches counted", unit: "count", of: (run) => run.matches },
];

/** How far, in pixels, find's current match moves over `frames` frames
 * from now. */
async function drift(page: Page, frames: number): Promise<number> {
  return page.evaluate(async (count) => {
    const view = document.querySelector(".timeline");
    const place = (): number | null => {
      const active = CSS.highlights.get("vessel-find-active");
      if (view === null || active === undefined) {
        return null;
      }
      for (const range of active) {
        if (range instanceof Range && range.startContainer.isConnected) {
          return (
            range.getBoundingClientRect().top - view.getBoundingClientRect().top
          );
        }
      }
      return null;
    };
    const seen: number[] = [];
    for (let frame = 0; frame < count; frame += 1) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const top = place();
      if (top !== null) {
        seen.push(top);
      }
    }
    return seen.length === 0
      ? Number.POSITIVE_INFINITY
      : Math.max(...seen) - Math.min(...seen);
  }, frames);
}

/** The time `input:<query>` was marked, once it has been. */
async function typed(page: Page, query: string): Promise<number> {
  return waitForMark(page, `input:${query}`, 10_000);
}

async function findOnce(
  browser: Browser,
  id: string,
  size: number,
): Promise<Finding> {
  const messages = Math.ceil(size / 3);
  const far = `Entry ${String(Math.floor((size * 0.9) / 3) * 3)}.`;
  const context = await browser.newContext(DESKTOP);
  try {
    const page = await context.newPage();
    await installProbe(page, [
      {
        kind: "last",
        name: "last",
        parent: ".timeline-column",
        text: "Turn ended",
      },
      { kind: "text", name: "count-1", selector: ".find-count", text: "1/1" },
      { kind: "match", name: "shown-1", text: "Entry 300." },
      {
        kind: "text",
        name: "count-2",
        selector: ".find-count",
        text: `1/${String(messages)}`,
      },
      { kind: "match", name: "shown-3", text: far },
    ]);
    await openUi(page);
    const card = page.locator(`li.card[data-session="${id}"] > a.card-open`);
    await expect(card).toBeVisible();
    await card.click();
    await waitForMark(page, "last", 900_000);
    await page.keyboard.press("Control+f");
    const field = page.getByRole("textbox", { name: "Find in session" });
    await field.fill("Entry 300.");
    const firstTyped = await typed(page, "Entry 300.");
    const firstCount = await waitForMark(page, "count-1", 60_000);
    const farAbove = await waitForMark(page, "shown-1", 60_000);
    const farAboveDrift = await drift(page, HOLD_FRAMES);
    await field.fill("short paragraph");
    const repeatTyped = await typed(page, "short paragraph");
    const repeatCount = await waitForMark(page, "count-2", 60_000);
    await field.fill(far);
    const farTyped = await typed(page, far);
    const farBelow = await waitForMark(page, "shown-3", 60_000);
    const farBelowDrift = await drift(page, HOLD_FRAMES);
    const marks = await readMarks(page);
    return {
      firstCountMs: firstCount - firstTyped,
      farAboveMs: farAbove - firstTyped,
      farAboveDriftPx: farAboveDrift,
      repeatCountMs: repeatCount - repeatTyped,
      farBelowMs: farBelow - farTyped,
      farBelowDriftPx: farBelowDrift,
      tasks: longTasks(marks.tasks, firstTyped, farBelow),
      matches: messages,
    };
  } finally {
    await context.close();
  }
}

export async function measureFind(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const sessions = new Map<number, string>();
  try {
    for (const size of SIZES) {
      const id = await start(
        env,
        NODE,
        folder(env, `find-${String(size)}`),
        "structured",
      );
      sessions.set(size, id);
      await promptAndWait(env, NODE, id, `entries ${String(size)}`, 600_000);
    }
    const load = { before: loadNow(), after: Number.NaN };
    const count = runs(3);
    const tables: Table[] = [];
    for (const [size, id] of sessions) {
      const findings: Finding[] = [];
      for (let run = 0; run < count; run += 1) {
        findings.push(await findOnce(browser, id, size));
      }
      tables.push({
        name: `${String(size)} entries, desktop (1440×900), unthrottled`,
        rows: rows(METRICS, findings),
      });
    }
    load.after = loadNow();
    return {
      title: "Find in a long session, and going to a match far away",
      method: [
        `${String(count)} runs per size, each in a fresh browser context: the board opened, the session's card clicked, the last entry drawn, then Mod+F and three queries typed into one find bar.`,
        "Entries: the timeline measurement's (`entries <n>`); the page opens at the log's end.",
        "Typed to the count / to it shown: from the find field's input event to the first animation frame with the count in the bar / with the match marked (the vessel-find-active highlight) inside the timeline's view. The field waits 100 ms after typing before it searches, included.",
        "`Entry 300.`: one match near the top (the first search: every entry's text folded); `short paragraph`: every message, over text folded already; `Entry <0.9 n>.`: one match far below.",
        `How far it moved: the match's top relative to the view, read for the ${String(HOLD_FRAMES)} frames after it was shown, the largest minus the smallest (pixels).`,
        "Long tasks: from the first query's input to the last match shown.",
      ],
      tables,
      notes: [],
      load,
    };
  } finally {
    await stop(env, NODE, [...sessions.values()]);
  }
}
