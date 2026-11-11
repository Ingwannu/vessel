// @ts-check
// Every text file of the bundle compressed once, at build: `<file>.br`
// (brotli, best quality) and `<file>.gz` (gzip, level 9) beside it, and
// `dist/.encodings` listing `<url> <sha256 of the file>` for each file
// compressed. vessel-web's build embeds the forms with their file and
// serves the one a request accepts (`crates/vessel-web/src/encoding.rs`);
// it refuses a form whose file's hash no longer matches the list, so a
// stale one is never served.
//
// [Decision Log] (milestone 11, track P; docs/PERFORMANCE.md)
// - Purpose: the page's script went over the wire uncompressed (1,147 KB)
//   to clients that all accept brotli or gzip.
// - Constraint: compressed once, not per request; no new dependency.
// - Choice: Node's own zlib, which the web build already runs on; it has
//   both encoders. Runs after integrity.mjs, since the index it rewrites
//   is compressed too. Fonts (woff2 is brotli inside) and images are left
//   as they are; a form is kept only when it is smaller than its file.
// - Alternatives: `vite-plugin-compression` (a dependency for the same
//   twenty lines); compressing in Rust's build script (two crates, run
//   unoptimised there on every bundle change).
import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const dist = fileURLToPath(new URL("../dist/", import.meta.url));
/** Extensions of the files worth compressing: text. */
const TEXT = [".html", ".js", ".mjs", ".css", ".json", ".svg", ".txt", ".map"];

/** Every file under `dir`.
 * @param {string} dir
 * @returns {string[]} */
function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

/** @type {string[]} */
const manifest = [];
let before = 0;
let brotli = 0;
for (const path of walk(dist)) {
  const name = relative(dist, path);
  if (name.startsWith(".") || !TEXT.some((ext) => name.endsWith(ext))) {
    continue;
  }
  const bytes = readFileSync(path);
  /** @type {[".br" | ".gz", Buffer][]} */
  const forms = [
    [
      ".br",
      brotliCompressSync(bytes, {
        params: {
          [constants.BROTLI_PARAM_QUALITY]: constants.BROTLI_MAX_QUALITY,
          [constants.BROTLI_PARAM_SIZE_HINT]: bytes.length,
        },
      }),
    ],
    [".gz", gzipSync(bytes, { level: 9 })],
  ];
  let kept = false;
  for (const [suffix, compressed] of forms) {
    if (compressed.length < bytes.length) {
      writeFileSync(`${path}${suffix}`, compressed);
      kept = true;
      if (suffix === ".br") {
        brotli += compressed.length;
      }
    }
  }
  if (kept) {
    before += bytes.length;
    const url = `/${name.split(sep).join("/")}`;
    const digest = createHash("sha256").update(bytes).digest("hex");
    manifest.push(`${url} ${digest}`);
  }
}
writeFileSync(join(dist, ".encodings"), `${manifest.sort().join("\n")}\n`);
process.stdout.write(
  `compressed ${String(manifest.length)} files: ${String(Math.round(before / 1024))} KiB, ${String(Math.round(brotli / 1024))} KiB as brotli\n`,
);
