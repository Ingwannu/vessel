// @ts-check
// The crate map as law (docs/QUALITY.md, M2), for one workspace: every
// crate's `vessel-*` dependencies are exactly the ones its row in the
// workspace's crate table names; the external crates the table's
// companion "External crates with one owner" names are used by their
// owners only; and the paths "Names only some files may use" lists appear
// only where it allows them.
//
// Usage: `node scripts/check-boundaries.mjs` for the root workspace;
// `--workspace <dir>` (relative to the current directory) for another.
//
// [Decision Log]
// - Purpose: AGENTS.md said "do not add an edge without updating
//   docs/ARCHITECTURE.md", and review missed it twice: `vessel-node ->
//   vessel-agent` (2026-11-12) and `vessel-testkit -> vessel-proto`
//   (2026-11-14) were in the manifests and not in the table. A table a
//   check reads cannot drift from the manifests.
// - Source of truth: the document, not this script. The script knows only
//   where each workspace's table is (`MAPS`); changing the map is changing
//   the table (and, in the root workspace, ADR 0001).
// - The table: a row's first cell names the crate in backticks (its
//   package name among them), its last cell the `vessel-*` crates it may
//   depend on: package names in backticks, or short names (`proto, rpc`
//   for `vessel-proto`, `vessel-rpc`), or `-` for none.
// - Edges compared: normal and build dependencies named `vessel-*`,
//   optional or target-specific ones included (they compile into the
//   product). Dev dependencies are test wiring; the documents describe them
//   in prose. External owners are checked on the same kinds: a test may set
//   up a state through another crate's format (vessel-node's e2e
//   `callback` moves a pairing time back in the database with rusqlite).
// - Names: a path in backticks (`gpui_platform`, `std::fs`) found in the
//   crate's product code, outside comments and literals, in a file the row
//   does not allow (a file, or a directory written with a trailing `/`,
//   relative to the crate). A name brought in through a grouped import
//   (`use std::{fs, io}`) is not seen; review covers that form.
// - Why `cargo metadata --no-deps`: it reads the manifests only, builds
//   nothing and needs no network, so the check runs in a second.
// - Behaviour the gate relies on: nothing written, output sorted, one line
//   per offence, exit 1 on any offence, 2 when the check cannot run (no
//   cargo on PATH, a document missing).
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { productLines, mask, shape } from "./rust-source.mjs";
import { Stop, cargoMetadata, root, run, workspaceOf } from "./workspace.mjs";

/** Where each workspace's crate table is: the document and its heading. */
const MAPS = /** @type {const} */ ({
  ".": { doc: "docs/ARCHITECTURE.md", heading: "## Crates" },
  "spikes/native": {
    doc: "spikes/native/README.md",
    heading: "## Crates, and what may depend on what",
  },
  "spikes/release": {
    doc: "spikes/release/README.md",
    heading: "## Crates, and what may depend on what",
  },
});

/** The cells of a Markdown table row, trimmed. */
/** @param {string} line @returns {string[]} */
function cells(line) {
  return line
    .split("|")
    .slice(1, -1)
    .map((cell) => cell.trim());
}

/** The names in backticks in a cell. */
/** @param {string} cell @returns {string[]} */
function ticked(cell) {
  return [...cell.matchAll(/`([^`]+)`/g)].map((m) => m[1] ?? "");
}

/**
 * The table rows under a heading, up to the next heading, without the
 * header row.
 * @param {readonly string[]} lines
 * @param {string} heading
 * @returns {string[][]}
 */
function tableUnder(lines, heading) {
  const start = lines.findIndex((line) => line.trim() === heading);
  if (start === -1) {
    return [];
  }
  /** @type {string[][]} */
  const rows = [];
  for (const line of lines.slice(start + 1)) {
    if (line.startsWith("#")) {
      break;
    }
    if (line.startsWith("|") && !/^\|\s*-/.test(line)) {
      rows.push(cells(line));
    }
  }
  return rows.slice(1);
}

/** @param {string} text @returns {RegExp} */
function nameUse(text) {
  const escaped = text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`(?<![A-Za-z0-9_:])${escaped}(?![A-Za-z0-9_])`);
}

run("boundaries", () => {
  const ws = workspaceOf(process.argv.slice(2));
  const map = Object.entries(MAPS).find(([dir]) => dir === ws)?.[1];
  if (map === undefined) {
    throw new Stop(
      `no crate table is known for ${ws}; name its document in MAPS (scripts/check-boundaries.mjs)`,
    );
  }
  const { doc: DOC, heading } = map;
  if (!existsSync(join(root, DOC))) {
    throw new Stop(`${DOC}: not found`);
  }
  const doc = readFileSync(join(root, DOC), "utf8").split("\n");
  const packages = cargoMetadata(ws);
  const memberNames = new Set(packages.map((p) => p.name));
  /** @type {string[]} */
  const problems = [];

  /** @type {Map<string, Set<string>>} */
  const documented = new Map();
  for (const row of tableUnder(doc, heading)) {
    const name = ticked(row[0] ?? "").find((n) => memberNames.has(n));
    const first = ticked(row[0] ?? "")[0];
    const last = row[row.length - 1] ?? "";
    if (row.length < 3 || first === undefined) {
      continue;
    }
    if (name === undefined) {
      problems.push(
        `${first}: in ${DOC}'s crate table but not a crate of ${ws}`,
      );
      continue;
    }
    const named = ticked(last);
    const edges =
      named.length > 0
        ? named
        : last === "-"
          ? []
          : last.split(",").map((short) => `vessel-${short.trim()}`);
    documented.set(name, new Set(edges));
  }

  /** @type {{ crate: string, only: boolean, crates: Set<string> }[]} */
  const owners = [];
  for (const row of tableUnder(doc, "### External crates with one owner")) {
    const crate = ticked(row[0] ?? "")[0];
    const rule = row[1] ?? "";
    if (crate === undefined) {
      continue;
    }
    if (!rule.startsWith("only ") && !rule.startsWith("never ")) {
      problems.push(
        `${DOC}: the owner of \`${crate}\` must read "only ..." or "never ...", found "${rule}"`,
      );
      continue;
    }
    owners.push({
      crate,
      only: rule.startsWith("only "),
      crates: new Set(ticked(rule)),
    });
  }

  for (const pkg of [...packages].sort((a, b) => (a.name < b.name ? -1 : 1))) {
    const table = documented.get(pkg.name);
    if (table === undefined) {
      problems.push(
        `${pkg.name}: a crate of ${ws} missing from ${DOC}'s crate table`,
      );
      continue;
    }
    const actual = new Set(
      pkg.dependencies
        .filter((d) => d.name.startsWith("vessel-") && d.kind !== "dev")
        .map((d) => d.name),
    );
    for (const edge of actual) {
      if (!table.has(edge)) {
        problems.push(
          `${pkg.name} -> ${edge}: in its Cargo.toml, not in ${DOC}'s crate table (add the edge with its reason, or remove the dependency)`,
        );
      }
    }
    for (const edge of table) {
      if (!actual.has(edge)) {
        problems.push(
          `${pkg.name} -> ${edge}: in ${DOC}'s crate table, not in its Cargo.toml (the table is stale)`,
        );
      }
    }
    for (const owner of owners) {
      const uses = pkg.dependencies.some(
        (d) => d.name === owner.crate && d.kind !== "dev",
      );
      if (uses && owner.only !== owner.crates.has(pkg.name)) {
        problems.push(
          `${pkg.name} -> ${owner.crate}: ${DOC} says ${owner.only ? "only" : "never"} ${[...owner.crates].join(", ")}`,
        );
      }
    }
  }

  // `| \`name\`, \`name\` | \`crate\` | only \`src/x.rs\` (or "nowhere") | why |`
  let names = 0;
  for (const row of tableUnder(doc, "### Names only some files may use")) {
    const [used, crateCell, whereCell] = [
      row[0] ?? "",
      row[1] ?? "",
      row[2] ?? "",
    ];
    const crate = ticked(crateCell)[0];
    const pkg = packages.find((p) => p.name === crate);
    if (pkg === undefined) {
      problems.push(
        `${DOC}: names row for \`${String(crate)}\`, not a crate of ${ws}`,
      );
      continue;
    }
    const allowed = new Set(
      whereCell.startsWith("only ") ? ticked(whereCell) : [],
    );
    const dir = relative(root, dirname(pkg.manifest_path)).split(sep).join("/");
    for (const path of productLines(root, [dir]).keys()) {
      const local = relative(join(root, dir), join(root, path))
        .split(sep)
        .join("/");
      if (
        [...allowed].some((entry) =>
          entry.endsWith("/") ? local.startsWith(entry) : local === entry,
        )
      ) {
        continue;
      }
      const text = readFileSync(join(root, path), "utf8");
      const tests = shape(text).testLineSet;
      const lines = mask(text).split("\n");
      for (const name of ticked(used)) {
        const pattern = nameUse(name);
        const hit = lines.findIndex(
          (line, index) => !tests.has(index) && pattern.test(line),
        );
        if (hit !== -1) {
          problems.push(
            `${path}:${String(hit + 1)}: names \`${name}\`; ${DOC} allows it in ${pkg.name} ${allowed.size > 0 ? `only in ${[...allowed].join(", ")}` : "nowhere"}`,
          );
        }
      }
    }
    names += 1;
  }

  problems.sort();
  if (problems.length > 0) {
    process.stderr.write(`boundaries (${ws}):\n${problems.join("\n")}\n`);
    process.exitCode = 1;
  } else {
    process.stdout.write(
      `boundaries (${ws}): ${String(packages.length)} crates match ${DOC}, ${String(owners.length)} owned external crates, ${String(names)} name rules\n`,
    );
  }
});
