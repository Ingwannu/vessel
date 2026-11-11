// Measurement 4 (perf.ts): a board of 500 cards.
//
// 500 live terminal sessions of `fake-tui` on one machine (build-server),
// each in a folder of its own, started through the CLI before the page
// opens. A `fake-tui` session is two processes (the harness and the
// grandchild it keeps, `fake-tui --sleeper`), so the node's tree grows
// by about a thousand; what it holds is recorded beside the numbers.

import { freemem } from "node:os";

import type { Browser, Page } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { launchUi } from "../open";
import { DESKTOP } from "./conditions";
import { pageFootprint, rest, scrollThrough } from "./inspect";
import { installProbe, readMarks, waitForMark } from "./probe";
import { loadNow, runs, type Section } from "./report";
import {
  eventually,
  folder,
  footprint,
  type Footprint,
  inParallel,
  nodePid,
  running,
  start,
  stop,
} from "./sessions";
import {
  type LongTasks,
  longTasks,
  median,
  type Metric,
  rows,
  taskMetrics,
} from "./stats";

const NODE = "build-server";
const CARDS = 500;
const SCROLL_STEPS = 50;
const REST_MS = 10_000;

interface BoardLoad {
  readonly usableMs: number;
  readonly allMs: number;
  readonly load: LongTasks;
  readonly rest: LongTasks;
  readonly scroll: LongTasks;
  readonly stepMedianMs: number;
  readonly stepLongestMs: number;
  readonly heapBytes: number;
  readonly elements: number;
  readonly cards: number;
}

const METRICS: readonly Metric<BoardLoad>[] = [
  { name: "board usable (first card)", unit: "ms", of: (run) => run.usableMs },
  {
    name: `all ${String(CARDS)} cards drawn`,
    unit: "ms",
    of: (run) => run.allMs,
  },
  ...taskMetrics<BoardLoad>("load", (run) => run.load),
  ...taskMetrics<BoardLoad>("rest", (run) => run.rest),
  ...taskMetrics<BoardLoad>("scroll", (run) => run.scroll),
  { name: "scroll: step, median", unit: "ms", of: (run) => run.stepMedianMs },
  { name: "scroll: step, longest", unit: "ms", of: (run) => run.stepLongestMs },
  { name: "JS heap used", unit: "bytes", of: (run) => run.heapBytes },
  {
    name: "elements in the document",
    unit: "count",
    of: (run) => run.elements,
  },
  { name: "cards drawn", unit: "count", of: (run) => run.cards },
];

/** The `data-lane` of the lane holding the most cards. */
async function fullestLane(page: Page): Promise<string> {
  return page.evaluate(() => {
    let fullest = "";
    let most = -1;
    for (const lane of document.querySelectorAll("section.lane")) {
      const cards = lane.querySelectorAll("li.card").length;
      if (cards > most) {
        most = cards;
        fullest = lane.getAttribute("data-lane") ?? "";
      }
    }
    return fullest;
  });
}

/** A fresh context, a fresh launch, the board with every card. */
async function loadOnce(browser: Browser): Promise<BoardLoad> {
  const context = await browser.newContext(DESKTOP);
  try {
    const page = await context.newPage();
    await installProbe(page, [
      { kind: "board", name: "usable", cards: 1 },
      { kind: "board", name: "all", cards: CARDS },
    ]);
    const cdp = await context.newCDPSession(page);
    const { url } = await launchUi(page);
    try {
      await page.goto(url, { waitUntil: "commit" });
    } catch {
      // Playwright's error would quote the one-use launch URL.
      throw new Error("Could not open the UI");
    }
    const usableMs = await waitForMark(page, "usable", 300_000);
    const allMs = await waitForMark(page, "all", 300_000);
    const resting = await rest(page, REST_MS);
    const lane = await fullestLane(page);
    const scroll = await scrollThrough(
      page,
      `section.lane[data-lane="${lane}"] .lane-body`,
      SCROLL_STEPS,
      "down",
    );
    const size = await pageFootprint(page, cdp);
    const cards = await page.locator("li.card").count();
    const marks = await readMarks(page);
    return {
      usableMs,
      allMs,
      load: longTasks(marks.tasks, 0, allMs),
      rest: longTasks(marks.tasks, resting.start, resting.end),
      scroll: longTasks(marks.tasks, scroll.start, scroll.end),
      stepMedianMs: median(scroll.stepsMs),
      stepLongestMs: Math.max(...scroll.stepsMs),
      heapBytes: size.heapBytes,
      elements: size.elements,
      cards,
    };
  } finally {
    await context.close();
  }
}

/** A footprint as a note's words. */
function described(tree: Footprint | null): string {
  const mib = (bytes: number): string => (bytes / 2 ** 20).toFixed(0);
  return tree === null
    ? "not read (no /proc)"
    : `${String(tree.processes)} processes; the node ${mib(tree.rootsBytes)} MiB resident, the tree ${mib(tree.residentBytes)} MiB (VmRSS summed)`;
}

/** Start the cards' sessions into `ids`, run `work`, then stop them all,
 * whatever happened; what the node held at each point goes in `notes`. */
async function withCards<T>(
  env: FleetEnv,
  ids: string[],
  notes: string[],
  work: () => Promise<T>,
): Promise<T> {
  const pid = nodePid(env, NODE);
  const tree = (): Footprint | null => (pid === null ? null : footprint([pid]));
  notes.push(`${NODE}'s node before: ${described(tree())}`);
  const freeBefore = freemem();
  const began = Date.now();
  try {
    const numbers = Array.from({ length: CARDS }, (_, index) => index + 1);
    await inParallel(numbers, 16, async (number) => {
      const name = `card-${String(number).padStart(3, "0")}`;
      ids.push(await start(env, NODE, folder(env, name), "terminal"));
    });
    await eventually(
      `${String(CARDS)} sessions to run`,
      async () => {
        const live = new Set(await running(env, NODE));
        return ids.every((id) => live.has(id));
      },
      300_000,
    );
    notes.push(
      `${String(CARDS)} sessions started through the CLI, 16 at a time, all running after ${((Date.now() - began) / 1000).toFixed(1)} s`,
      `${NODE}'s node with them: ${described(tree())}; the machine's free memory went from ${(freeBefore / 2 ** 30).toFixed(1)} to ${(freemem() / 2 ** 30).toFixed(1)} GiB`,
    );
    return await work();
  } finally {
    await stop(env, NODE, ids);
    await eventually(
      "the sessions to stop",
      async () => {
        const live = new Set(await running(env, NODE));
        return ids.every((id) => !live.has(id));
      },
      120_000,
    );
    notes.push(`${NODE}'s node after stopping them: ${described(tree())}`);
  }
}

export async function measureBoard(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const notes: string[] = [];
  const count = runs(3);
  const { loads, load } = await withCards(env, [], notes, async () => {
    const load = { before: loadNow(), after: Number.NaN };
    const loads: BoardLoad[] = [];
    for (let run = 0; run < count; run += 1) {
      loads.push(await loadOnce(browser));
    }
    load.after = loadNow();
    return { loads, load };
  });
  return {
    title: `Board: ${String(CARDS)} cards`,
    method: [
      `${String(CARDS)} live terminal sessions on ${NODE}, started before the page opens; the board for all machines; ${String(count)} runs, each a fresh browser context with a fresh launch, desktop (1440×900), unthrottled.`,
      "Board usable: from the navigation's start to the first animation frame with the first lane and a card laid out and no font face loading; all cards: the same with every card.",
      `Load: long tasks (50 ms or more) from the navigation's start to all cards drawn. Rest: the ${String(REST_MS / 1000)} s after, untouched (the page lists sessions every 2 s).`,
      `Scroll: the fullest lane's column from its top to its bottom in ${String(SCROLL_STEPS)} equal steps; a step sets scrollTop and waits two animation frames.`,
      "JS heap used after HeapProfiler.collectGarbage; elements: document.getElementsByTagName('*').",
    ],
    tables: [{ name: `${String(CARDS)} cards`, rows: rows(METRICS, loads) }],
    notes,
    load,
  };
}
