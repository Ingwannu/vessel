// How long a find takes over a long session: 5,000 entries, half prompts
// of a line and half agent messages of about 1,500 characters of
// Markdown. The bounds are loose (a slow CI runner passes); the numbers
// measured are in docs/plans/backend-m11.md, "Track W: progress".
// `VESSEL_FIND_TIMINGS=1 npx vitest run src/data/find.speed.test.ts`
// prints them.

import { expect, test } from "vitest";

import { findAll, findInSlices } from "./find";
import type { Entry } from "./timeline-model";

const PARAGRAPH =
  "The **build** failed in `crates/vessel-web` because the test named " +
  "`a_qr_code_is_an_svg` expected a header. I changed [the check](https://x.test) " +
  "and ran the suite again; everything passes now. ";

function conversation(size: number): Entry[] {
  return Array.from({ length: size }, (_, index): Entry =>
    index % 2 === 0
      ? {
          kind: "prompt",
          key: `p${String(index)}`,
          seq: index,
          at: "",
          text: `Please look at item ${String(index)} and fix what is broken.`,
          replayed: false,
          attachments: [],
          turn: index,
        }
      : {
          kind: "message",
          key: `m${String(index)}`,
          seq: index,
          at: "",
          text: PARAGRAPH.repeat(8),
        },
  );
}

function timed(run: () => void): number {
  const start = performance.now();
  run();
  return performance.now() - start;
}

test("a find over 5,000 entries stays well under a frame once folded", async () => {
  const entries = conversation(5000);
  const cold = timed(() => findAll(entries, "suite again"));
  const warm: number[] = [];
  for (let run = 0; run < 20; run += 1) {
    warm.push(timed(() => findAll(entries, "suite again")));
  }
  warm.sort((a, b) => a - b);
  const median = warm[Math.floor(warm.length / 2)] ?? 0;
  // The slices the page runs: the longest stretch without a yield.
  let longest = 0;
  let mark = performance.now();
  const found = await findInSlices(
    conversation(5000),
    "suite again",
    () => false,
    () => {
      longest = Math.max(longest, performance.now() - mark);
      return Promise.resolve().then(() => {
        mark = performance.now();
      });
    },
  );
  longest = Math.max(longest, performance.now() - mark);
  if (process.env["VESSEL_FIND_TIMINGS"] === "1") {
    process.stderr.write(
      `find, 5000 entries: first ${cold.toFixed(1)} ms, later median ${median.toFixed(1)} ms, longest slice (cold) ${longest.toFixed(1)} ms, ${String(found?.length ?? 0)} matches\n`,
    );
  }
  expect(found).toHaveLength(2500 * 8);
  expect(cold).toBeLessThan(2000);
  expect(median).toBeLessThan(200);
});
