// @ts-check
// The crate map as law (docs/QUALITY.md, "Modules"): every workspace
// crate's `vessel-*` dependencies are exactly the ones its row in
// docs/ARCHITECTURE.md's crate table names, and the external crates that
// table's companion ("External crates with one owner") gives an owner to
// are used by their owners only.
//
// [Decision Log]
// - Purpose: AGENTS.md said "do not add an edge without updating
//   docs/ARCHITECTURE.md", and review missed it twice: `vessel-node ->
//   vessel-agent` (2026-11-12) and `vessel-testkit -> vessel-proto`
//   (2026-11-14) were in the manifests and not in the table. A table a
//   check reads cannot drift from the manifests.
// - Source of truth: the document, not this script. The script holds no
//   crate names of its own; changing the map is changing the table (and
//   ADR 0001, which records why each edge exists).
// - Edges compared: normal and build dependencies, optional or
//   target-specific ones included (they compile into the product). Dev
//   dependencies are test wiring; ARCHITECTURE describes them in prose.
//   External owners are checked on the same kinds: a test may set up a
//   state through another crate's format (vessel-node's e2e `callback`
//   moves a pairing time back in the database with rusqlite).
// - Why `cargo metadata --no-deps`: it reads the manifests only, builds
//   nothing and needs no network, so the check runs in a second.
// - Behaviour the gate relies on: nothing written, output sorted, one line
//   per offence, exit 1 on any offence.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const DOC = "docs/ARCHITECTURE.md";

/**
 * @typedef {object} Dependency
 * @property {string} name
 * @property {string | null} kind `null` for a normal dependency
 */
/**
 * @typedef {object} Package
 * @property {string} name
 * @property {Dependency[]} dependencies
 */

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
 * The table rows under a heading, up to the next heading.
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

/** @type {string[]} */
const problems = [];
const doc = readFileSync(join(root, DOC), "utf8").split("\n");

// The crate table: `| \`vessel-x\` | job | proto, rpc |` ("-" for none).
/** @type {Map<string, Set<string>>} */
const documented = new Map();
for (const row of tableUnder(doc, "## Crates")) {
  const name = ticked(row[0] ?? "")[0];
  const last = row[row.length - 1] ?? "";
  if (name === undefined || row.length < 3) {
    continue;
  }
  const edges =
    last === "-"
      ? []
      : last.split(",").map((short) => `vessel-${short.trim()}`);
  documented.set(name, new Set(edges));
}

// The owners table: `| \`crate\` | only \`vessel-a\`, \`vessel-b\` | why |`
// or `| \`crate\` | never \`vessel-a\` | why |`.
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

/** @type {{ packages: Package[] }} */
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--no-deps", "--format-version", "1", "--offline"],
    { cwd: root, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  ),
);
const members = new Set(metadata.packages.map((p) => p.name));

for (const name of [...documented.keys()]) {
  if (!members.has(name)) {
    problems.push(`${name}: in ${DOC}'s crate table but not a workspace crate`);
  }
}
for (const pkg of metadata.packages) {
  const table = documented.get(pkg.name);
  if (table === undefined) {
    problems.push(
      `${pkg.name}: a workspace crate missing from ${DOC}'s crate table`,
    );
    continue;
  }
  const actual = new Set(
    pkg.dependencies
      .filter((d) => members.has(d.name) && d.kind !== "dev")
      .map((d) => d.name),
  );
  for (const edge of [...actual].sort()) {
    if (!table.has(edge)) {
      problems.push(
        `${pkg.name} -> ${edge}: in its Cargo.toml, not in ${DOC}'s crate table (add the edge with its reason, here and in ADR 0001, or remove the dependency)`,
      );
    }
  }
  for (const edge of [...table].sort()) {
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

problems.sort();
if (problems.length > 0) {
  process.stderr.write(`boundaries:\n${problems.join("\n")}\n`);
  process.exitCode = 1;
} else {
  process.stdout.write(
    `boundaries: ${String(members.size)} crates match ${DOC}, ${String(owners.length)} owned external crates\n`,
  );
}
