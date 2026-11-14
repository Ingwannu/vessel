// @ts-check
// A list of known offenders that only shrinks, shared by the checks that
// grandfather what was there when they began (the file budget, the
// native app's token literals).
//
// [Decision Log]
// - Purpose: a new check lands with zero failures (docs/QUALITY.md, P4),
//   yet the debt it measures must not grow and must be seen shrinking.
// - Choice: a text file of `<key> <count>` lines, sorted by key, `#`
//   comments. Each measured key over the budget must be listed at exactly
//   its count: higher fails (it grew), lower fails until the new count is
//   recorded (so it cannot grow back), within the budget or gone fails
//   until its line is removed. Nothing may be added: review refuses a new
//   line; the check cannot tell a new line from an old one.
// - Why exact counts and not ceilings alone: a ceiling that is never
//   lowered lets a file shrink by a hundred lines and then regrow them.
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { root } from "./workspace.mjs";

/**
 * The list's entries; a missing file is an empty list.
 * @param {string} list the list's repository path
 * @returns {{ counts: Map<string, number>, problems: string[] }}
 */
function read(list) {
  /** @type {Map<string, number>} */
  const counts = new Map();
  /** @type {string[]} */
  const problems = [];
  const path = join(root, list);
  if (!existsSync(path)) {
    return { counts, problems };
  }
  let previous = "";
  for (const [index, raw] of readFileSync(path, "utf8").split("\n").entries()) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) {
      continue;
    }
    const where = `${list}:${String(index + 1)}`;
    const parsed = /^(.+\S)\s+(\d+)$/.exec(line);
    const key = parsed?.[1];
    const count = Number(parsed?.[2]);
    if (key === undefined || !Number.isSafeInteger(count)) {
      problems.push(`${where}: expected "<key> <count>", found "${line}"`);
      continue;
    }
    if (counts.has(key)) {
      problems.push(`${where}: ${key} is listed twice`);
    } else if (key < previous) {
      problems.push(`${where}: ${key} is out of order; keep the list sorted`);
    }
    previous = key;
    counts.set(key, count);
  }
  return { counts, problems };
}

/**
 * @typedef {object} Rule
 * @property {string} list the list's repository path
 * @property {number} budget the most a key may measure without a line
 * @property {string} unit what is counted ("lines", "literals")
 * @property {string} fix what to do with a key over the budget
 */

/**
 * The offences of `measured` against the list, sorted, one line each, and
 * the number of listed keys.
 * @param {Rule} rule
 * @param {ReadonlyMap<string, number>} measured every key the check saw
 * @returns {{ problems: string[], listed: number }}
 */
export function compare(rule, measured) {
  const { list, budget, unit, fix } = rule;
  const { counts: listed, problems } = read(list);
  const over = `over the budget of ${String(budget)}`;
  for (const [key, count] of measured) {
    const recorded = listed.get(key);
    const has = `${key}: ${String(count)} ${unit}`;
    if (recorded === undefined) {
      if (count > budget) {
        problems.push(`${has}, ${over}; ${fix}`);
      }
    } else if (count <= budget) {
      problems.push(
        `${has}, within the budget of ${String(budget)}; take it off ${list}`,
      );
    } else if (count > recorded) {
      problems.push(
        `${has}; listed at ${String(recorded)}, it may not grow (budget ${String(budget)})`,
      );
    } else if (count < recorded) {
      problems.push(
        `${has}, under its recorded ${String(recorded)}; record ${String(count)} in ${list}`,
      );
    }
  }
  for (const key of listed.keys()) {
    if (!measured.has(key)) {
      problems.push(
        `${key}: listed, but the check no longer sees it (gone, renamed or test code); take it off ${list}`,
      );
    }
  }
  problems.sort();
  return { problems, listed: listed.size };
}

/**
 * The list a check would start from today: every key over the budget.
 * @param {ReadonlyMap<string, number>} measured
 * @param {number} budget
 * @returns {string}
 */
export function initialList(measured, budget) {
  return [...measured.entries()]
    .filter(([, count]) => count > budget)
    .sort(([a], [b]) => (a < b ? -1 : 1))
    .map(([key, count]) => `${key} ${String(count)}\n`)
    .join("");
}
