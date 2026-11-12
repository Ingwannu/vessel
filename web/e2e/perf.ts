// `npm run perf`: the page's speed, measured (milestone 11, track P,
// docs/plans/backend-m11.md; the numbers are kept in docs/PERFORMANCE.md).
// Run on demand only, never by `npm run e2e`: it takes minutes and its
// result is a set of numbers, not a pass or a fail.
//
// Five measurements against the fleet of global-setup.ts, one test each
// (`npm run perf -- -g <word>` runs one): the board loading over a slow
// and a phone-like link, a timeline of thousands of entries, find in it
// and going to a match far away, a terminal under heavy output, a board
// of 500 cards. Each starts the sessions it needs through the CLI and
// stops them when it ends. The rule of the track: no optimisation
// without a number before and after from this harness on the same
// machine.
//
// Environment: `VESSEL_PERF_OUT` (the JSON file; artifacts/perf.json by
// default), `VESSEL_PERF_RUNS` (runs per measurement, overriding each
// one's default), `VESSEL_CARGO_PROFILE=release` (the fleet on release
// binaries, fleet.ts).

import { test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { measureBoard } from "./perf/board";
import { measureFind } from "./perf/find";
import { measureLoad } from "./perf/load";
import { record } from "./perf/report";
import { measureTerminal } from "./perf/terminal";
import { measureTimeline } from "./perf/timeline";

const HOUR = 60 * 60_000;

test("load over a slow link and a phone-like link", async ({ browser }) => {
  test.setTimeout(HOUR);
  const env = loadEnv();
  record(env, browser.version(), "load", await measureLoad(browser, env));
});

test("a timeline of thousands of entries", async ({ browser }) => {
  test.setTimeout(HOUR);
  const env = loadEnv();
  record(
    env,
    browser.version(),
    "timeline",
    await measureTimeline(browser, env),
  );
});

test("find in a long session, and a match far away", async ({ browser }) => {
  test.setTimeout(HOUR);
  const env = loadEnv();
  record(env, browser.version(), "find", await measureFind(browser, env));
});

test("a terminal under heavy output", async ({ browser }) => {
  test.setTimeout(HOUR);
  const env = loadEnv();
  record(
    env,
    browser.version(),
    "terminal",
    await measureTerminal(browser, env),
  );
});

test("a board of 500 cards", async ({ browser }) => {
  test.setTimeout(HOUR);
  const env = loadEnv();
  record(env, browser.version(), "board", await measureBoard(browser, env));
});
