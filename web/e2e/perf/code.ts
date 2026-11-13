// Measurement 6 (perf.ts): a session's code panel over a large change
// and a large tree (milestone 11, track B-page).
//
// The repository, made in the fleet's temporary directory with git and
// added as a project of gpu-box through the CLI: one file of 40,000 lines
// with every tenth line changed (4,000 hunks, about 32,000 diff lines:
// seven of the node's 5,000-line pages; a hunk is never split across
// pages, so a file rewritten whole would be one hunk cut at 5,000 lines),
// 1,500 small files changed beside it, and a folder of 5,000 files (the
// node's bound for one listing). A structured
// session runs in the project's folder. What is measured is the page:
// the Changes tab drawing its first diff line and scrolling the whole
// list (each file's diff and each page asked for as its row is reached),
// and the Files tab drawing the tree and the big folder and scrolling it.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import type { Browser } from "@playwright/test";
import { expect } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { openUi } from "../open";
import { DESKTOP } from "./conditions";
import { pageFootprint, rest, scrollThrough } from "./inspect";
import { installProbe, readMarks, waitForMark } from "./probe";
import { loadNow, runs, type Section } from "./report";
import { folder, start, stop, vessel } from "./sessions";
import {
  type LongTasks,
  longTasks,
  median,
  type Metric,
  rows,
  taskMetrics,
} from "./stats";

const NODE = "gpu-box";
const BIG_LINES = 40_000;
/** Every this many lines of the big file, one changed. */
const CHANGED_EVERY = 10;
const SMALL_FILES = 1_500;
const TREE_FILES = 5_000;
const SCROLL_STEPS = 100;
const REST_MS = 5_000;

interface Opening {
  readonly firstMs: number;
  readonly scroll: LongTasks;
  readonly stepMedianMs: number;
  readonly stepLongestMs: number;
  readonly rest: LongTasks;
  readonly heapBytes: number;
  readonly elements: number;
  readonly drawn: number;
}

function metrics(what: string): readonly Metric<Opening>[] {
  return [
    { name: `${what}: first row drawn`, unit: "ms", of: (run) => run.firstMs },
    ...taskMetrics<Opening>("scroll", (run) => run.scroll),
    { name: "scroll: step, median", unit: "ms", of: (run) => run.stepMedianMs },
    {
      name: "scroll: step, longest",
      unit: "ms",
      of: (run) => run.stepLongestMs,
    },
    ...taskMetrics<Opening>("rest", (run) => run.rest),
    { name: "JS heap used", unit: "bytes", of: (run) => run.heapBytes },
    {
      name: "elements in the document",
      unit: "count",
      of: (run) => run.elements,
    },
    {
      name: "rows drawn after the scroll",
      unit: "count",
      of: (run) => run.drawn,
    },
  ];
}

function git(cwd: string, args: string[]): void {
  execFileSync("git", args, {
    cwd,
    env: { ...process.env, HOME: cwd, GIT_CONFIG_NOSYSTEM: "1" },
    stdio: "ignore",
  });
}

/** The repository, committed, then changed. */
function makeRepository(root: string): void {
  git(root, ["init", "-q", "-b", "main"]);
  git(root, ["config", "user.name", "Perf"]);
  git(root, ["config", "user.email", "perf@vessel.invalid"]);
  const big = (changed: boolean) =>
    Array.from(
      { length: BIG_LINES },
      (_, line) =>
        `export const value${String(line)} = ${String(changed && line % CHANGED_EVERY === 0 ? line * 5 : line * 3)}; // line ${String(line)}`,
    ).join("\n");
  writeFileSync(join(root, "big.ts"), `${big(false)}\n`);
  mkdirSync(join(root, "small"), { recursive: true });
  mkdirSync(join(root, "many"), { recursive: true });
  for (let index = 0; index < SMALL_FILES; index += 1) {
    writeFileSync(
      join(root, "small", `f${String(index)}.ts`),
      `export const a = ${String(index)};\n`,
    );
  }
  for (let index = 0; index < TREE_FILES; index += 1) {
    writeFileSync(
      join(root, "many", `entry-${String(index).padStart(5, "0")}.md`),
      "# entry\n",
    );
  }
  git(root, ["add", "-A"]);
  git(root, ["commit", "-q", "-m", "big"]);
  writeFileSync(join(root, "big.ts"), `${big(true)}\n`);
  for (let index = 0; index < SMALL_FILES; index += 1) {
    writeFileSync(
      join(root, "small", `f${String(index)}.ts`),
      `export const a = ${String(index + 1)};\nexport const b = 2;\n`,
    );
  }
}

/** Open the session `id` in a fresh context, click the panel's `tab`,
 * and measure the list `scroller` it shows. */
async function openOnce(
  browser: Browser,
  id: string,
  tab: "Changes" | "Files",
): Promise<Opening> {
  const context = await browser.newContext(DESKTOP);
  try {
    const page = await context.newPage();
    const row = tab === "Changes" ? ".d-line" : ".tree-row";
    const scroller = tab === "Changes" ? ".changes-scroll" : ".file-scroll";
    await installProbe(page, [
      { kind: "present", name: "first", selector: row },
    ]);
    const cdp = await context.newCDPSession(page);
    const origin = await openUi(page);
    await page.goto(`${origin}/sessions/${NODE}/${id}`);
    const tabs = page.getByRole("tablist", { name: "Session panel" });
    await expect(tabs).toBeVisible();
    const clicked = await page.evaluate(() => performance.now());
    await tabs.getByRole("tab", { name: tab }).click();
    const first = await waitForMark(page, "first", 120_000);
    if (tab === "Files") {
      await page.getByRole("button", { name: "many" }).click();
      await expect(
        page.getByRole("button", { name: "entry-00010.md" }),
      ).toBeVisible();
    }
    const scroll = await scrollThrough(page, scroller, SCROLL_STEPS, "down");
    const resting = await rest(page, REST_MS);
    const size = await pageFootprint(page, cdp);
    const drawn = await page.locator(row).count();
    const marks = await readMarks(page);
    return {
      firstMs: first - clicked,
      scroll: longTasks(marks.tasks, scroll.start, scroll.end),
      stepMedianMs: median(scroll.stepsMs),
      stepLongestMs: Math.max(...scroll.stepsMs),
      rest: longTasks(marks.tasks, resting.start, resting.end),
      heapBytes: size.heapBytes,
      elements: size.elements,
      drawn,
    };
  } finally {
    await context.close();
  }
}

/** One table of `count` openings of `tab`. */
async function table(
  browser: Browser,
  id: string,
  tab: "Changes" | "Files",
  count: number,
) {
  const openings: Opening[] = [];
  for (let run = 0; run < count; run += 1) {
    openings.push(await openOnce(browser, id, tab));
  }
  return {
    name:
      tab === "Changes"
        ? `Changes: ${String(BIG_LINES / CHANGED_EVERY)} hunks in one file and ${String(SMALL_FILES)} files, desktop`
        : `Files: a folder of ${String(TREE_FILES)} entries opened, desktop`,
    rows: rows(metrics(tab), openings),
  };
}

export async function measureCode(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const root = folder(env, "code-big");
  makeRepository(root);
  await vessel(env, NODE, ["project", "add", root]);
  const id = await start(env, NODE, root, "structured");
  try {
    const load = { before: loadNow(), after: Number.NaN };
    const count = runs(3);
    const tables = [
      await table(browser, id, "Changes", count),
      await table(browser, id, "Files", count),
    ];
    load.after = loadNow();
    return {
      title: "Code panel: a large change and a large tree",
      method: [
        `${String(count)} runs per tab, each in a fresh browser context: the session's page opened, then the panel's tab clicked.`,
        `The repository: one file of ${String(BIG_LINES)} lines with every ${String(CHANGED_EVERY)}th changed (${String(BIG_LINES / CHANGED_EVERY)} hunks, about ${String((BIG_LINES / CHANGED_EVERY) * 8)} diff lines, several of the node's 5,000-line pages); ${String(SMALL_FILES)} small files changed; a folder of ${String(TREE_FILES)} files. The session runs in the project's folder (Uncommitted is the comparison).`,
        "First row drawn: from the click (performance.now just before it) to the first animation frame with a diff line / a tree row laid out.",
        `Scroll: the list from its top to its end in ${String(SCROLL_STEPS)} equal steps (each file's diff and each page asked for as its row is reached); long tasks while scrolling and in the ${String(REST_MS / 1000)} s after.`,
        "JS heap used after a full collection; elements and rows drawn counted after the scroll.",
      ],
      tables,
      notes: [],
      load,
    };
  } finally {
    await stop(env, NODE, [id]);
  }
}
