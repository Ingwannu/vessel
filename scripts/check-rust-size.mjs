// @ts-check
// The Rust file budget (docs/QUALITY.md, M3): a file of product code over
// 500 lines fails, unless it is on the workspace's list of files still to
// be divided (`<workspace>/scripts/rust-size-allow.txt`), and then it may
// not grow past the size the list records for it (scripts/ratchet.mjs).
// Test code is not counted (what counts as test code:
// scripts/rust-source.mjs).
//
// Usage: `node scripts/check-rust-size.mjs` for the root workspace;
// `--workspace <dir>` (relative to the current directory) for another;
// `--list` prints the list a workspace adopting the budget starts from.
//
// [Decision Log]
// - Purpose: AGENTS.md asked for a reason behind any file over about 400
//   lines, and review did not hold it: on 2026-11-14 33 files of product
//   code in the root workspace were over 500 lines and 7 over 800. The
//   page has had the same budget as a lint failure since milestone 10
//   (web/scripts/check-size.mjs); this is its Rust counterpart.
// - Why 500 and not 400: 500 is the figure the 2026-11-14 measurement
//   used, so the list holds exactly the files it names; the target stays
//   one responsibility per file, and most files are far under it.
// - Scope: the member crates of one workspace, from its manifest, so the
//   check needs no cargo; each workspace keeps its own list.
// - Behaviour the gate relies on: no network, nothing written, output
//   sorted, one line per offence, exit 1 on any offence, 2 when the check
//   cannot run.
import { compare, initialList } from "./ratchet.mjs";
import { productLines } from "./rust-source.mjs";
import { members, root, run, workspaceOf } from "./workspace.mjs";

/** Lines of product code a file may hold. */
const BUDGET = 500;

run("rust-size", () => {
  const argv = process.argv.slice(2);
  const ws = workspaceOf(argv);
  const counts = productLines(root, members(ws));
  if (argv.includes("--list")) {
    process.stdout.write(initialList(counts, BUDGET));
    return;
  }
  const list = `${ws === "." ? "" : `${ws}/`}scripts/rust-size-allow.txt`;
  const { problems, listed } = compare(
    { list, budget: BUDGET, unit: "lines", fix: "divide it" },
    counts,
  );
  if (problems.length > 0) {
    process.stderr.write(`rust-size (${ws}):\n${problems.join("\n")}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write(
      `rust-size (${ws}): ${String(counts.size)} files checked against ${String(BUDGET)} lines, ${String(listed)} still to divide\n`,
    );
  }
});
