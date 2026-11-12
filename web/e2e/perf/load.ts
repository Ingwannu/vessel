// Measurement 1 (perf.ts): the page loading the board, cold, under each
// profile of conditions.ts.
//
// The fleet holds three live terminal sessions, one per machine; one
// works in a folder with a Hangul name, which its card's title shows, so
// drawing the board needs Pretendard (nothing else the board draws by
// default is Hangul: the page's own words are English).
//
// Usable: from the navigation's start (the page's time origin) to the
// first animation frame at which the board's first lane and the three
// cards are laid out with a box and no font face is still loading
// (`document.fonts.status`), so the faces the cards draw are in.

import type { Browser } from "@playwright/test";

import type { FleetEnv } from "../fleet";
import { launchUi } from "../open";
import {
  applyConditions,
  type Conditions,
  PROFILES,
  type Transfer,
  TransferCount,
} from "./conditions";
import { installProbe, readMarks, waitForMark } from "./probe";
import { loadNow, runs, type Section, type Table } from "./report";
import { eventually, folder, running, start, stop } from "./sessions";
import { type Metric, rows } from "./stats";

/** One cold load. */
interface Load {
  readonly fcpMs: number;
  readonly usableMs: number;
  readonly bytes: Transfer;
  /** Whether Pretendard had loaded by then. */
  readonly pretendard: boolean;
}

const METRICS: readonly Metric<Load>[] = [
  { name: "first contentful paint", unit: "ms", of: (load) => load.fcpMs },
  { name: "board usable", unit: "ms", of: (load) => load.usableMs },
  { name: "bytes: document", unit: "bytes", of: (load) => load.bytes.document },
  { name: "bytes: script", unit: "bytes", of: (load) => load.bytes.script },
  {
    name: "bytes: stylesheet",
    unit: "bytes",
    of: (load) => load.bytes.stylesheet,
  },
  { name: "bytes: font", unit: "bytes", of: (load) => load.bytes.font },
  { name: "bytes: other", unit: "bytes", of: (load) => load.bytes.other },
  { name: "bytes: HTTP total", unit: "bytes", of: (load) => load.bytes.total },
  {
    name: "bytes: WebSocket received",
    unit: "bytes",
    of: (load) => load.bytes.websocket,
  },
];

/** The sessions the board shows: one per machine, gpu-box's in a folder
 * whose Hangul name becomes its card's title. */
const SESSIONS: readonly { node: string; folder: string }[] = [
  { node: "laptop", folder: "release-notes" },
  { node: "gpu-box", folder: "배포-점검" },
  { node: "build-server", folder: "nightly-build" },
];

/** A fresh context under `conditions`, a fresh launch, the board. */
async function loadOnce(
  browser: Browser,
  conditions: Conditions,
): Promise<Load> {
  const context = await browser.newContext(conditions.context);
  try {
    const page = await context.newPage();
    await installProbe(page, [
      { kind: "board", name: "usable", cards: SESSIONS.length },
    ]);
    const cdp = await context.newCDPSession(page);
    await cdp.send("Network.enable", {});
    const transfer = new TransferCount(cdp);
    await applyConditions(cdp, conditions);
    const { url } = await launchUi(page);
    try {
      // `commit`: the page's own marks say when it was ready, and the
      // load event can come before the cards (Pretendard is asked for
      // only once a Hangul title is laid out).
      await page.goto(url, { waitUntil: "commit" });
    } catch {
      // Playwright's error would quote the one-use launch URL.
      throw new Error("Could not open the UI");
    }
    const usableMs = await waitForMark(page, "usable", 180_000);
    const bytes = transfer.now();
    const marks = await readMarks(page);
    const pretendard = await page.evaluate(() =>
      [...document.fonts].some(
        (face) => face.family === "Pretendard" && face.status === "loaded",
      ),
    );
    return { fcpMs: marks.fcp ?? Number.NaN, usableMs, bytes, pretendard };
  } finally {
    await context.close();
  }
}

export async function measureLoad(
  browser: Browser,
  env: FleetEnv,
): Promise<Section> {
  const ids = new Map<string, string>();
  for (const session of SESSIONS) {
    ids.set(
      session.node,
      await start(env, session.node, folder(env, session.folder), "terminal"),
    );
  }
  const load = { before: loadNow(), after: Number.NaN };
  try {
    for (const [node, id] of ids) {
      await eventually(
        `${id} to run`,
        async () => (await running(env, node)).includes(id),
        30_000,
      );
    }
    const count = runs(5);
    const tables: Table[] = [];
    const notes: string[] = [];
    for (const conditions of PROFILES) {
      const loads: Load[] = [];
      for (let run = 0; run < count; run += 1) {
        loads.push(await loadOnce(browser, conditions));
      }
      const drawn = loads.filter((one) => one.pretendard).length;
      notes.push(
        `${conditions.name}: Pretendard loaded when usable in ${String(drawn)} of ${String(count)} runs`,
      );
      tables.push({
        name: `${conditions.name}: ${conditions.description}`,
        rows: rows(METRICS, loads),
      });
    }
    load.after = loadNow();
    return {
      title: "Load: the board, cold, per profile",
      method: [
        `${String(count)} runs per profile, each in a fresh browser context (empty cache) with a fresh one-use launch.`,
        `The fleet holds ${String(SESSIONS.length)} live terminal sessions, one per machine; one card's title is Hangul (its folder's name), so the board needs Pretendard.`,
        "First contentful paint: performance's first-contentful-paint entry.",
        `Board usable: from the navigation's start to the first animation frame at which the first lane and all ${String(SESSIONS.length)} cards are laid out and no font face is loading.`,
        "Bytes: Network.loadingFinished encodedDataLength (on the wire, headers included) of every HTTP response finished by then, by the resource type of its response; WebSocket: payload bytes of the frames received by then.",
      ],
      tables,
      notes,
      load,
    };
  } finally {
    for (const [node, id] of ids) {
      await stop(env, node, [id]);
    }
  }
}
