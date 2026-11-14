// @ts-check
// The Rust file budget (docs/QUALITY.md, "Modules"): a file of product
// code over 500 lines fails, unless it is on the list of files still to be
// divided (scripts/rust-size-allow.txt), and then it may not grow past the
// size the list records for it. Test code is not counted (what counts as
// test code: scripts/rust-source.mjs).
//
// [Decision Log]
// - Purpose: AGENTS.md asked for a reason behind any file over about 400
//   lines, and review did not hold it: on 2026-11-14 33 files of product
//   code were over 500 lines and 7 over 800. The page has had the same
//   budget as a lint failure since milestone 10
//   (web/scripts/check-size.mjs); this is its Rust counterpart.
// - Why 500 and not 400: 500 is the figure the 2026-11-14 measurement used,
//   so the list holds exactly the files it names; the target stays one
//   responsibility per file, and most files are far under it.
// - The list only shrinks, in two ways. A listed file whose count fell
//   below its recorded size fails until the list records the new size,
//   so a file cannot shrink and then grow back; a listed file back within
//   the budget, or gone, fails until its line is removed. Nothing is
//   added: a new long file is divided, not listed (review refuses a line
//   added to the list).
// - Scope: the workspace's crates (`crates/`). The native app
//   (`spikes/native`) and the release tools (`spikes/release`) are other
//   workspaces with their own gate; `--dir spikes/native --list` prints the
//   list such a gate would start from.
// - Behaviour the gate relies on: no network, nothing written, output
//   sorted by path, one line per offence, exit 1 on any offence.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { productLines } from "./rust-source.mjs";

const root = fileURLToPath(new URL("..", import.meta.url));

/** Lines of product code a file may hold. */
const BUDGET = 500;

/** The list of files still to be divided, beside this script. */
const ALLOW = join(root, "scripts", "rust-size-allow.txt");

/**
 * The list: `<path> <lines>` per line, `#` comments, blank lines ignored.
 * @returns {{ ceilings: Map<string, number>, problems: string[] }}
 */
function readList() {
  /** @type {Map<string, number>} */
  const ceilings = new Map();
  /** @type {string[]} */
  const problems = [];
  const lines = readFileSync(ALLOW, "utf8").split("\n");
  let previous = "";
  for (const [index, raw] of lines.entries()) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) {
      continue;
    }
    const where = `scripts/rust-size-allow.txt:${String(index + 1)}`;
    const parsed = /^(\S+)\s+(\d+)$/.exec(line);
    const path = parsed?.[1];
    const lines = Number(parsed?.[2]);
    if (path === undefined || !Number.isSafeInteger(lines)) {
      problems.push(`${where}: expected "<path> <lines>", found "${line}"`);
      continue;
    }
    if (ceilings.has(path)) {
      problems.push(`${where}: ${path} is listed twice`);
    } else if (path < previous) {
      problems.push(`${where}: ${path} is out of order; keep the list sorted`);
    }
    previous = path;
    ceilings.set(path, lines);
  }
  return { ceilings, problems };
}

/** @param {readonly string[]} argv @returns {{ dir: string, list: boolean }} */
function options(argv) {
  const at = argv.indexOf("--dir");
  return {
    dir: at === -1 ? "crates" : (argv[at + 1] ?? "crates"),
    list: argv.includes("--list"),
  };
}

const { dir, list } = options(process.argv.slice(2));
const counts = productLines(root, [dir]);
const sorted = [...counts.entries()].sort(([a], [b]) => (a < b ? -1 : 1));

if (list) {
  // What a new list would hold: for a workspace adopting the budget.
  const over = sorted.filter(([, lines]) => lines > BUDGET);
  process.stdout.write(
    over.map(([path, lines]) => `${path} ${String(lines)}\n`).join(""),
  );
} else {
  const { ceilings, problems } = readList();
  for (const [path, lines] of sorted) {
    const ceiling = ceilings.get(path);
    if (ceiling === undefined) {
      if (lines > BUDGET) {
        problems.push(
          `${path}: ${String(lines)} lines, over the budget of ${String(BUDGET)}; divide it`,
        );
      }
    } else if (lines <= BUDGET) {
      problems.push(
        `${path}: ${String(lines)} lines, within the budget of ${String(BUDGET)}; take it off scripts/rust-size-allow.txt`,
      );
    } else if (lines > ceiling) {
      problems.push(
        `${path}: ${String(lines)} lines; it is still to be divided and may not grow past ${String(ceiling)} (budget ${String(BUDGET)})`,
      );
    } else if (lines < ceiling) {
      problems.push(
        `${path}: ${String(lines)} lines, under its recorded ${String(ceiling)}; record ${String(lines)} in scripts/rust-size-allow.txt (budget ${String(BUDGET)})`,
      );
    }
  }
  for (const path of [...ceilings.keys()].sort()) {
    if (!counts.has(path)) {
      problems.push(
        `${path}: listed but not product code under ${dir}/ (gone, renamed or test code); take it off scripts/rust-size-allow.txt`,
      );
    }
  }
  problems.sort();
  if (problems.length > 0) {
    process.stderr.write(`rust-size:\n${problems.join("\n")}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write(
      `rust-size: ${String(counts.size)} files checked against ${String(BUDGET)} lines, ${String(ceilings.size)} still to divide\n`,
    );
  }
}
