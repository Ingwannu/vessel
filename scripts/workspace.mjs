// @ts-check
// What the checks under scripts/ know of a Cargo workspace: which one a
// run is about, its member crates (from its manifest), `cargo metadata`
// for it, and how a check stops when it cannot go on.
//
// [Decision Log]
// - Purpose: the repository has three Cargo workspaces (the root,
//   `spikes/native`, `spikes/release`), each with its own gate. The checks
//   are the same for each; only the data differs (the list of long files,
//   the crate map), and that data lives in the workspace it describes.
// - Choice: `--workspace <dir>` names a workspace relative to the current
//   directory, so a gate run from `spikes/native` says `--workspace .`; no
//   flag means the root workspace, so the root gate needs no argument.
// - Members are read from the manifest's `members` (names and `dir/*`),
//   not from `cargo metadata`: the size check then runs without cargo.
// - A check that cannot go on (no cargo on PATH, a manifest it cannot
//   read) prints one line saying what is missing and what to do, and exits
//   2, apart from 1 for offences; never a stack trace.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

/** The repository's root directory. */
export const root = fileURLToPath(new URL("..", import.meta.url));

/** A condition a check cannot go past; its message is the one line shown. */
export class Stop extends Error {}

/**
 * The workspace a run is about, as a repository path with `/` separators
 * (`.` for the root): `--workspace <dir>`, relative to the current
 * directory, or the root workspace.
 * @param {readonly string[]} argv
 * @returns {string}
 */
export function workspaceOf(argv) {
  const at = argv.indexOf("--workspace");
  if (at === -1) {
    return ".";
  }
  const given = argv[at + 1];
  if (given === undefined) {
    throw new Stop("--workspace needs a directory");
  }
  const path = relative(root, resolve(given)).split(sep).join("/");
  const ws = path === "" ? "." : path;
  if (path.startsWith("..") || !isWorkspace(ws)) {
    throw new Stop(
      `${given}: not a Cargo workspace of this repository (no [workspace] in its Cargo.toml)`,
    );
  }
  return ws;
}

/** @param {string} ws @returns {boolean} */
function isWorkspace(ws) {
  const manifest = join(root, ws, "Cargo.toml");
  return (
    existsSync(manifest) &&
    /^\[workspace\]/m.test(readFileSync(manifest, "utf8"))
  );
}

/**
 * The workspace's member crates' directories, as repository paths, from
 * its manifest's `members` (plain directories and `dir/*`).
 * @param {string} ws
 * @returns {string[]}
 */
export function members(ws) {
  const manifest = join(root, ws, "Cargo.toml");
  if (!existsSync(manifest)) {
    throw new Stop(`${ws}/Cargo.toml: not found`);
  }
  const text = readFileSync(manifest, "utf8");
  const list = /^\[workspace\][^[]*?^members\s*=\s*\[([^\]]*)\]/ms.exec(text);
  if (list?.[1] === undefined) {
    throw new Stop(`${ws}/Cargo.toml: no [workspace] members list`);
  }
  const prefix = ws === "." ? "" : `${ws}/`;
  return [...list[1].matchAll(/"([^"]+)"/g)].flatMap((m) => {
    const member = m[1] ?? "";
    if (member.endsWith("/*")) {
      const parent = member.slice(0, -2);
      return readdirSync(join(root, ws, parent))
        .filter((name) =>
          statSync(join(root, ws, parent, name, "Cargo.toml"), {
            throwIfNoEntry: false,
          })?.isFile(),
        )
        .map((name) => `${prefix}${parent}/${name}`);
    }
    if (member.includes("*")) {
      throw new Stop(
        `${ws}/Cargo.toml: the member pattern "${member}" is not one this check reads (a directory or dir/*)`,
      );
    }
    return [`${prefix}${member}`];
  });
}

/**
 * @typedef {object} Dependency
 * @property {string} name the package's name (not a rename)
 * @property {string | null} kind `null` for a normal dependency
 */
/**
 * @typedef {object} Package
 * @property {string} name
 * @property {string} manifest_path
 * @property {Dependency[]} dependencies
 */

/**
 * `cargo metadata --no-deps` for the workspace: its manifests only, no
 * build, no network.
 * @param {string} ws
 * @returns {Package[]}
 */
export function cargoMetadata(ws) {
  /** @type {string} */
  let out;
  try {
    out = execFileSync(
      "cargo",
      [
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        "--offline",
        "--manifest-path",
        join(root, ws, "Cargo.toml"),
      ],
      {
        cwd: root,
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") {
      throw new Stop(
        "cargo was not found on PATH; install the Rust toolchain (rust-toolchain.toml names it) or put cargo on PATH, then run this again",
      );
    }
    const stderr =
      error instanceof Error && "stderr" in error ? String(error.stderr) : "";
    const first =
      stderr.split("\n").find((line) => line.trim() !== "") ?? String(error);
    throw new Stop(`cargo metadata failed for ${ws}: ${first.trim()}`);
  }
  /** @type {unknown} */
  const parsed = JSON.parse(out);
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("packages" in parsed) ||
    !Array.isArray(parsed.packages)
  ) {
    throw new Stop(`cargo metadata for ${ws} gave no package list`);
  }
  /** @type {Package[]} */
  const packages = parsed.packages;
  return packages;
}

/**
 * Runs a check's body; a `Stop` (or anything else thrown) becomes one line
 * on stderr and exit code 2.
 * @param {string} name the check's name, the line's prefix
 * @param {() => void} body
 */
export function run(name, body) {
  try {
    body();
  } catch (error) {
    const text = error instanceof Error ? error.message : String(error);
    process.stderr.write(`${name}: ${text}\n`);
    process.exitCode = 2;
  }
}
