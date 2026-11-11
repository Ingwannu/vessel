// @ts-check
// The page's file budget (docs/plans/backend-m10.md, "Structure"): a
// TypeScript file over 300 lines, or a stylesheet over 400, fails, unless
// it is on the list of files still to be divided, and then it may not grow
// past the size it had when it was listed.
//
// [Decision Log]
// - Purpose: milestones 7 to 9 left route files of 450 to 700 lines that
//   fetch, decide and draw in one function. Review did not stop them
//   growing; a lint failure does.
// - The list only shrinks: a track that divides a file removes it (and the
//   check says when a listed file is back under the budget, so the line
//   is not left behind); nothing is added to it.
// - Scope: everything under `src/` but the generated wire types. Tests and
//   the e2e harness are not the page and are not counted.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const web = fileURLToPath(new URL("..", import.meta.url));
const src = join(web, "src");

/** The budgets, by extension. */
const BUDGET = { ".ts": 300, ".tsx": 300, ".css": 400 };

/** Files still to be divided, with the lines each had when it was listed
 * (2026-11-11, milestone 10's track W0). Each may shrink, never grow.
 * `data/timeline.ts` is listed as track E left it on `backend/m10` (382
 * lines on this track's base, 406 with track E's title events), so the
 * merge of the two tracks passes. */
/** @type {Readonly<Record<string, number>>} */
const STILL_TO_DIVIDE = {
  "src/bridge/client.ts": 467,
  "src/data/timeline.ts": 406,
  "src/data/board.ts": 353,
  "src/routes/devices.tsx": 349,
  "src/routes/fleet.tsx": 316,
};

/** @param {string} path @returns {string[]} */
function files(path) {
  if (statSync(path).isFile()) {
    return [path];
  }
  return readdirSync(path).flatMap((name) => files(join(path, name)));
}

/** @param {string} path @returns {number | undefined} */
function budgetOf(path) {
  if (path.endsWith(".test.ts")) {
    return undefined;
  }
  for (const [extension, lines] of Object.entries(BUDGET)) {
    if (path.endsWith(extension)) {
      return lines;
    }
  }
  return undefined;
}

/** @type {string[]} */
const problems = [];
/** @type {string[]} */
const notes = [];
for (const path of files(src)) {
  const name = relative(web, path).split(sep).join("/");
  const budget = budgetOf(path);
  if (budget === undefined || name.startsWith("src/wire/")) {
    continue;
  }
  const lines = readFileSync(path, "utf8").split("\n").length - 1;
  const ceiling = STILL_TO_DIVIDE[name];
  if (ceiling === undefined) {
    if (lines > budget) {
      problems.push(
        `${name}: ${String(lines)} lines, over the budget of ${String(budget)}; divide it`,
      );
    }
  } else if (lines > ceiling) {
    problems.push(
      `${name}: ${String(lines)} lines; it is still to be divided and may not grow past ${String(ceiling)}`,
    );
  } else if (lines <= budget) {
    notes.push(
      `${name}: ${String(lines)} lines, within the budget: take it off the list`,
    );
  }
}
for (const name of Object.keys(STILL_TO_DIVIDE)) {
  if (
    !files(src).some(
      (path) => relative(web, path).split(sep).join("/") === name,
    )
  ) {
    notes.push(`${name}: gone; take it off the list`);
  }
}

if (problems.length > 0 || notes.length > 0) {
  process.stderr.write(`File budget:\n${[...problems, ...notes].join("\n")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write(
    `size: within budget, ${String(Object.keys(STILL_TO_DIVIDE).length)} files still to divide\n`,
  );
}
