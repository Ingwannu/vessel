// Measurement 2 (perf.ts): a structured session whose timeline holds
// thousands of entries, opened from the board.
//
// Each session's one turn is `entries <n>` to `fake-acp`: n updates of
// three kinds in turn (a message chunk with a short Markdown paragraph, a
// thought chunk, a completed tool call with an id of its own). The page's
// reducer (src/data/timeline.ts) merges only consecutive chunks of one
// kind and finds a tool call by its id, so each update stays an entry:
// n entries, a third of each kind, besides the session's own (agent
// ready, the prompt, the turn's end). The turn has ended before the page
// opens, so what is measured is the page replaying and drawing the log.

import type { Browser, CDPSession, Page } from "@playwright/test";
import { expect } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { openUi } from "../open";
import { DESKTOP } from "./conditions";
import { pageFootprint, rest, scrollThrough } from "./inspect";
import { installProbe, readMarks, waitForMark } from "./probe";
import { loadNow, runs, type Section, type Table } from "./report";
import { folder, promptAndWait, start, stop } from "./sessions";
import {
  type LongTasks,
  longTasks,
  median,
  type Metric,
  rows,
  taskMetrics,
} from "./stats";

/** The timeline sizes measured, in entries. */
const SIZES: readonly number[] = [5_000, 20_000];

/** Scroll steps from the bottom of the timeline to its top. */
const SCROLL_STEPS = 100;

const NODE = "gpu-box";

/** The page left alone once the last entry is drawn (it lists the
 * fleet's sessions every 2 s meanwhile). */
const REST_MS = 10_000;

interface Opening {
  readonly firstMs: number;
  readonly lastMs: number;
  readonly fold: LongTasks;
  readonly rest: LongTasks;
  readonly scroll: LongTasks;
  readonly stepMedianMs: number;
  readonly stepLongestMs: number;
  readonly heapBytes: number;
  readonly elements: number;
  readonly entries: number;
}

const METRICS: readonly Metric<Opening>[] = [
  { name: "first entry drawn", unit: "ms", of: (run) => run.firstMs },
  { name: "last entry drawn", unit: "ms", of: (run) => run.lastMs },
  ...taskMetrics<Opening>("to the last entry", (run) => run.fold),
  ...taskMetrics<Opening>("rest", (run) => run.rest),
  ...taskMetrics<Opening>("scroll", (run) => run.scroll),
  { name: "scroll: step, median", unit: "ms", of: (run) => run.stepMedianMs },
  { name: "scroll: step, longest", unit: "ms", of: (run) => run.stepLongestMs },
  { name: "JS heap used", unit: "bytes", of: (run) => run.heapBytes },
  {
    name: "elements in the document",
    unit: "count",
    of: (run) => run.elements,
  },
  { name: "timeline items drawn", unit: "count", of: (run) => run.entries },
];

/** Open the board in a fresh context, then the session `id` from its
 * card, and measure. */
async function openOnce(browser: Browser, id: string): Promise<Opening> {
  const context = await browser.newContext(DESKTOP);
  try {
    const page = await context.newPage();
    await installProbe(page, [
      { kind: "present", name: "first", selector: ".timeline .t-item" },
      {
        kind: "last",
        name: "last",
        parent: ".timeline-column",
        text: "Turn ended",
      },
    ]);
    const cdp = await context.newCDPSession(page);
    await openUi(page);
    const card = page.locator(`li.card[data-session="${id}"] > a.card-open`);
    await expect(card).toBeVisible();
    await card.click();
    const first = await waitForMark(page, "first", 120_000);
    const last = await waitForMark(page, "last", 900_000);
    return await settle(page, cdp, first, last);
  } finally {
    await context.close();
  }
}

/** The page once the last entry is drawn: at rest, its size, then the
 * scroll. */
async function settle(
  page: Page,
  cdp: CDPSession,
  first: number,
  last: number,
): Promise<Opening> {
  const resting = await rest(page, REST_MS);
  const size = await pageFootprint(page, cdp);
  const entries = await page.locator(".timeline-column > .t-item").count();
  const scroll = await scrollThrough(page, ".timeline", SCROLL_STEPS, "up");
  const marks = await readMarks(page);
  const click = marks.at.get("click") ?? Number.NaN;
  return {
    firstMs: first - click,
    lastMs: last - click,
    fold: longTasks(marks.tasks, click, last),
    rest: longTasks(marks.tasks, resting.start, resting.end),
    scroll: longTasks(marks.tasks, scroll.start, scroll.end),
    stepMedianMs: median(scroll.stepsMs),
    stepLongestMs: Math.max(...scroll.stepsMs),
    heapBytes: size.heapBytes,
    elements: size.elements,
    entries,
  };
}

export async function measureTimeline(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const sessions = new Map<number, string>();
  try {
    for (const size of SIZES) {
      const id = await start(
        env,
        NODE,
        folder(env, `timeline-${String(size)}`),
        "structured",
      );
      sessions.set(size, id);
      await promptAndWait(env, NODE, id, `entries ${String(size)}`, 600_000);
    }
    const load = { before: loadNow(), after: Number.NaN };
    const count = runs(3);
    const tables: Table[] = [];
    const notes: string[] = [];
    for (const [size, id] of sessions) {
      const openings: Opening[] = [];
      for (let run = 0; run < count; run += 1) {
        openings.push(await openOnce(browser, id));
      }
      const drawn = openings.map((opening) => opening.entries);
      notes.push(
        `${String(size)} entries: ${drawn.join(", ")} timeline items drawn (the ${String(size)} and the session's own)`,
      );
      if (drawn.some((entries) => entries < size)) {
        throw new Error(
          `fewer than ${String(size)} entries drawn: ${drawn.join(", ")}`,
        );
      }
      tables.push({
        name: `${String(size)} entries, desktop (1440×900), unthrottled`,
        rows: rows(METRICS, openings),
      });
    }
    load.after = loadNow();
    return {
      title: "Timeline: a structured session of thousands of entries",
      method: [
        `${String(count)} runs per size, each in a fresh browser context: the board opened, then the session's card clicked (the page navigates within itself).`,
        "Entries: fake-acp's `entries <n>`, a message (short Markdown paragraph), a thought and a completed tool call in turn, so the reducer merges none; the turn ended before the page opened.",
        "First / last entry drawn: from the click (its event's time) to the first animation frame with a timeline item laid out / with the turn's end as the timeline's last item.",
        `Long tasks (50 ms or more, the Long Tasks API): from the click to the last entry; at rest, the ${String(REST_MS / 1000)} s after it, untouched (the page lists sessions every 2 s); while scrolling.`,
        `Scroll: the timeline from its bottom to its top in ${String(SCROLL_STEPS)} equal steps; a step sets scrollTop and waits two animation frames.`,
        "JS heap used: Performance.getMetrics JSHeapUsedSize after HeapProfiler.collectGarbage, after the rest; elements: document.getElementsByTagName('*').",
      ],
      tables,
      notes,
      load,
    };
  } finally {
    await stop(env, NODE, [...sessions.values()]);
  }
}
