// Measurement 3 (perf.ts): a terminal session under heavy output.
//
// `fake-tui`'s `flood <lines> <width>` writes that many lines of that
// many bytes (and a CR LF each) as fast as the terminal takes them, then
// prints `flooded <lines>`. The command is typed into the page's
// terminal (xterm.js) as a person types it; the page draws with xterm's
// DOM renderer, whose rows are the text of `.xterm-rows`.
//
// Rendered rate: the flood's bytes over the time from the Enter key's
// event to the first animation frame at which `flooded <lines>` is in
// the terminal's rows. That time holds the node's PTY, the node, the
// bridge and the page together: it is the rate a person sees.

import type { Browser } from "@playwright/test";
import { expect } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { openUi } from "../open";
import { DESKTOP } from "./conditions";
import { installProbe, readMarks, restingFrame, waitForMark } from "./probe";
import { loadNow, runs, type Section, type Table } from "./report";
import { eventually, folder, running, start, stop } from "./sessions";
import {
  type Frames,
  frames,
  type LongTasks,
  longTasks,
  type Metric,
  rows,
  taskMetrics,
} from "./stats";

/** The floods measured. */
const FLOODS: readonly { lines: number; width: number }[] = [
  { lines: 20_000, width: 100 },
  { lines: 100_000, width: 100 },
  // Long enough (about ten seconds on release binaries) that the rate
  // is the sustained one, not a burst's.
  { lines: 500_000, width: 100 },
];

const NODE = "gpu-box";

interface Flood {
  readonly elapsedMs: number;
  readonly bytesPerSecond: number;
  readonly frames: Frames;
  readonly tasks: LongTasks;
}

const METRICS: readonly Metric<Flood>[] = [
  {
    name: "Enter to the last line drawn",
    unit: "ms",
    of: (run) => run.elapsedMs,
  },
  { name: "rendered rate", unit: "bytes/s", of: (run) => run.bytesPerSecond },
  {
    name: "frame interval at rest",
    unit: "ms",
    of: (run) => run.frames.referenceMs,
  },
  { name: "frames drawn", unit: "frames", of: (run) => run.frames.drawn },
  { name: "frames expected", unit: "frames", of: (run) => run.frames.expected },
  { name: "late frames", unit: "frames", of: (run) => run.frames.late },
  { name: "frames missed", unit: "frames", of: (run) => run.frames.missed },
  { name: "longest frame gap", unit: "ms", of: (run) => run.frames.longestMs },
  ...taskMetrics<Flood>("flood", (run) => run.tasks),
];

/** A fresh terminal session, opened in a fresh context, flooded. */
async function floodOnce(
  browser: Browser,
  env: FleetEnv,
  lines: number,
  width: number,
): Promise<Flood> {
  const id = await start(env, NODE, folder(env, "flood"), "terminal");
  const context = await browser.newContext(DESKTOP);
  try {
    await eventually(
      `${id} to run`,
      async () => (await running(env, NODE)).includes(id),
      30_000,
    );
    const page = await context.newPage();
    await installProbe(page, [
      {
        kind: "text",
        name: "flooded",
        selector: ".xterm-rows",
        text: `flooded ${String(lines)}`,
      },
    ]);
    const origin = await openUi(page);
    await page.goto(`${origin}/sessions/${NODE}/${id}`);
    const terminal = page.getByLabel("Session terminal");
    await expect(terminal).toContainText("ready");
    const referenceMs = await restingFrame(page);
    await terminal.click();
    const command = `flood ${String(lines)} ${String(width)}`;
    await page.keyboard.insertText(command);
    await expect(terminal).toContainText(command);
    await page.keyboard.press("Enter");
    const flooded = await waitForMark(page, "flooded", 600_000);
    const marks = await readMarks(page);
    const enter = marks.at.get("enter") ?? Number.NaN;
    const elapsedMs = flooded - enter;
    // Each line is `width` bytes and a CR LF.
    const bytes = lines * (width + 2);
    return {
      elapsedMs,
      bytesPerSecond: bytes / (elapsedMs / 1000),
      frames: frames(marks.frames, enter, flooded, referenceMs),
      tasks: longTasks(marks.tasks, enter, flooded),
    };
  } finally {
    await context.close();
    await stop(env, NODE, [id]);
  }
}

export async function measureTerminal(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const load = { before: loadNow(), after: Number.NaN };
  const count = runs(3);
  const tables: Table[] = [];
  for (const { lines, width } of FLOODS) {
    const floods: Flood[] = [];
    for (let run = 0; run < count; run += 1) {
      floods.push(await floodOnce(browser, env, lines, width));
    }
    tables.push({
      name: `flood ${String(lines)} ${String(width)}: ${(lines * (width + 2)).toLocaleString("en-US")} bytes, desktop (1440×900), unthrottled`,
      rows: rows(METRICS, floods),
    });
  }
  load.after = loadNow();
  return {
    title: "Terminal: a session under heavy output",
    method: [
      `${String(count)} runs per size, each a fresh terminal session in a fresh browser context; the command typed into the terminal, then Enter.`,
      "Rendered rate: lines × (width + 2) bytes over the time from the Enter key's event to the first animation frame with `flooded <lines>` in the terminal's rows (.xterm-rows, xterm's DOM renderer).",
      "Frame interval at rest: the median gap over 60 animation frames before the command. Late frame: a gap between consecutive frames over 1.5 × that interval; frames missed: for each late gap, round(gap / interval) − 1. Expected: the stretch over the interval.",
      "Long tasks (50 ms or more) from Enter to the last line drawn.",
    ],
    tables,
    notes: [],
    load,
  };
}
