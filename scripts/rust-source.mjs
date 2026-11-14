// @ts-check
// What the Rust checks under scripts/ read of a source file: its text
// with comments and literals blanked, which lines are test code, and
// which files are test code as a whole. Shared by check-rust-size.mjs and
// check-boundaries.mjs so the two agree on what "test code" means.
//
// [Decision Log]
// - Purpose: the file budget (docs/QUALITY.md, "Modules") counts the lines
//   a reader of the product has to hold, so test code is left out, as
//   web/scripts/check-size.mjs leaves the page's tests out.
// - Alternatives: counting whole files (a file with a large test module
//   would be "too long" for its tests, and moving tests out would be the
//   cheapest fix, which is not the point); asking rustc or rust-analyzer
//   for the cfg(test) items (needs a build; this check must run in a
//   second without compiling).
// - Choice: a small lexer that blanks comments, strings, raw strings and
//   char literals, then pairs braces. It is exact for the forms this code
//   base uses (`#[cfg(test)]` and `#[cfg(all(test, ...))]` on a `mod`, an
//   `fn` or an `impl`, `#[cfg(test)] mod x;` naming a file). It does not
//   evaluate other cfg predicates: `cfg(any(test, ...))` code is compiled
//   outside tests and counts as product code.
// - Must stay true: the measure is deterministic and only depends on the
//   file's text, so a ceiling recorded today means the same tomorrow.
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { basename, dirname, join, relative, sep } from "node:path";

/** A file's path relative to the repository root, with `/` separators. */
/** @param {string} root @param {string} path @returns {string} */
export function repoPath(root, path) {
  return relative(root, path).split(sep).join("/");
}

/** Every `.rs` file under `dir`, skipping `target` directories. */
/** @param {string} dir @returns {string[]} */
export function rustFiles(dir) {
  if (!existsSync(dir)) {
    return [];
  }
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      return name === "target" ? [] : rustFiles(path);
    }
    return name.endsWith(".rs") ? [path] : [];
  });
}

/** @param {string} c @returns {boolean} */
function isIdent(c) {
  return /[A-Za-z0-9_]/.test(c);
}

/**
 * The source with every comment, string, raw string and char literal
 * replaced by spaces (newlines kept), so braces, brackets and attributes
 * found in it are code. Lifetimes (`'a`) are left as they are.
 * @param {string} text
 * @returns {string}
 */
export function mask(text) {
  const out = [...text];
  const n = out.length;
  /** @param {number} from @param {number} to */
  const blank = (from, to) => {
    for (let k = from; k < to && k < n; k += 1) {
      if (out[k] !== "\n") {
        out[k] = " ";
      }
    }
  };
  let i = 0;
  while (i < n) {
    const c = out[i];
    const next = out[i + 1] ?? "";
    const before = i > 0 ? (out[i - 1] ?? "") : "";
    if (c === "/" && next === "/") {
      let j = i;
      while (j < n && out[j] !== "\n") {
        j += 1;
      }
      blank(i, j);
      i = j;
    } else if (c === "/" && next === "*") {
      let depth = 1;
      let j = i + 2;
      while (j < n && depth > 0) {
        if (out[j] === "/" && out[j + 1] === "*") {
          depth += 1;
          j += 2;
        } else if (out[j] === "*" && out[j + 1] === "/") {
          depth -= 1;
          j += 2;
        } else {
          j += 1;
        }
      }
      blank(i, j);
      i = j;
    } else if (
      (c === "r" || ((c === "b" || c === "c") && next === "r")) &&
      !isIdent(before)
    ) {
      // A raw string: r"..", r#".."#, br"..", cr"..".
      let j = c === "r" ? i + 1 : i + 2;
      let hashes = 0;
      while (out[j] === "#") {
        hashes += 1;
        j += 1;
      }
      if (out[j] === '"') {
        const close = `"${"#".repeat(hashes)}`;
        let k = j + 1;
        while (k < n && out.slice(k, k + close.length).join("") !== close) {
          k += 1;
        }
        blank(i, k + close.length);
        i = k + close.length;
      } else {
        i += 1;
      }
    } else if (c === '"') {
      let j = i + 1;
      while (j < n && out[j] !== '"') {
        j += out[j] === "\\" ? 2 : 1;
      }
      blank(i, j + 1);
      i = j + 1;
    } else if (c === "'") {
      // A char literal ('x', '\n', '\u{1F600}', '\''), not a lifetime.
      if (next === "\\") {
        let j = i + 2;
        while (j < n && out[j] !== "'") {
          j += 1;
        }
        blank(i, j + 1);
        i = j + 1;
      } else if (out[i + 2] === "'") {
        blank(i, i + 3);
        i += 3;
      } else {
        i += 1;
      }
    } else {
      i += 1;
    }
  }
  return out.join("");
}

/** The index just past the bracket matching the one at `open`. */
/** @param {string} masked @param {number} open @returns {number} */
function matching(masked, open) {
  const pairs = /** @type {const} */ ({ "{": "}", "[": "]", "(": ")" });
  /** @type {string[]} */
  const stack = [];
  for (let k = open; k < masked.length; k += 1) {
    const c = masked.charAt(k);
    if (c === "{" || c === "[" || c === "(") {
      stack.push(pairs[c]);
    } else if (c === "}" || c === "]" || c === ")") {
      stack.pop();
      if (stack.length === 0) {
        return k + 1;
      }
    }
  }
  return masked.length;
}

/** Whether an attribute's inside (`cfg(...)`) holds only under test. */
/** @param {string} inside @returns {boolean} */
function onlyUnderTest(inside) {
  const compact = inside.replace(/\s+/g, "");
  if (compact === "cfg(test)") {
    return true;
  }
  const all = /^cfg\(all\((.*)\)\)$/.exec(compact);
  if (all === null) {
    return false;
  }
  // `test` as one of all(...)'s own terms, not inside any(..) or not(..).
  let depth = 0;
  let term = "";
  for (const c of `${all[1] ?? ""},`) {
    if (c === "(") {
      depth += 1;
    } else if (c === ")") {
      depth -= 1;
    }
    if (c === "," && depth === 0) {
      if (term === "test") {
        return true;
      }
      term = "";
    } else {
      term += c;
    }
  }
  return false;
}

/**
 * @typedef {object} Shape
 * @property {number} lines every line of the file
 * @property {number} testLines lines inside cfg(test) items
 * @property {ReadonlySet<number>} testLineSet those lines, counted from 0
 * @property {string[]} testModules names of `#[cfg(test)] mod x;` files
 */

/**
 * The file's lines, the ones inside items compiled only under test (from
 * the line of the attribute to the item's last line), and the out-of-line
 * modules declared only under test.
 * @param {string} text
 * @returns {Shape}
 */
export function shape(text) {
  const masked = mask(text);
  const lineOf = (/** @type {number} */ index) =>
    masked.slice(0, index).split("\n").length - 1;
  const lines = text.endsWith("\n")
    ? text.split("\n").length - 1
    : text.split("\n").length;
  if (/#!\[\s*cfg\s*\(\s*test\s*\)\s*\]/.test(masked)) {
    const every = new Set(Array.from({ length: lines }, (_, line) => line));
    return { lines, testLines: lines, testLineSet: every, testModules: [] };
  }
  /** @type {Set<number>} */
  const test = new Set();
  /** @type {string[]} */
  const testModules = [];
  const attribute = /#\[/g;
  let found = attribute.exec(masked);
  while (found !== null) {
    const start = found.index;
    const end = matching(masked, start + 1);
    if (onlyUnderTest(masked.slice(start + 2, end - 1))) {
      // Skip further attributes, then the item runs to its `;` or to the
      // brace closing its first `{`, whichever the item reaches first.
      let k = end;
      for (;;) {
        while (/\s/.test(masked.charAt(k))) {
          k += 1;
        }
        if (masked.startsWith("#[", k)) {
          k = matching(masked, k + 1);
        } else {
          break;
        }
      }
      const itemStart = k;
      let itemEnd = masked.length;
      for (; k < masked.length; k += 1) {
        const c = masked.charAt(k);
        if (c === ";") {
          itemEnd = k + 1;
          break;
        }
        if (c === "{") {
          itemEnd = matching(masked, k);
          break;
        }
        if (c === "(" || c === "[") {
          k = matching(masked, k) - 1;
        }
      }
      const declared =
        /^(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z0-9_]+)\s*;$/.exec(
          masked.slice(itemStart, itemEnd).replace(/\s+/g, " ").trim(),
        );
      if (declared?.[1] !== undefined) {
        testModules.push(declared[1]);
      }
      for (let line = lineOf(start); line <= lineOf(itemEnd - 1); line += 1) {
        test.add(line);
      }
      attribute.lastIndex = itemEnd;
    } else {
      attribute.lastIndex = end;
    }
    found = attribute.exec(masked);
  }
  return { lines, testLines: test.size, testLineSet: test, testModules };
}

/**
 * Whether the path alone says test code (see docs/QUALITY.md).
 * @param {string} path a repository path with `/` separators
 * @returns {boolean}
 */
export function testByPath(path) {
  const parts = path.split("/");
  const name = parts[parts.length - 1] ?? "";
  return (
    parts.some(
      (part) => part === "tests" || part === "examples" || part === "benches",
    ) ||
    parts.includes("vessel-testkit") ||
    name === "tests.rs" ||
    name === "testing.rs" ||
    name.endsWith("_tests.rs")
  );
}

/**
 * The directory a file's out-of-line child modules live in: its own for a
 * crate root or `mod.rs`, a directory named after it otherwise.
 * @param {string} path
 * @returns {string}
 */
function childDir(path) {
  const name = basename(path);
  const root =
    name === "mod.rs" ||
    name === "lib.rs" ||
    name === "main.rs" ||
    name === "build.rs" ||
    basename(dirname(path)) === "bin";
  return root ? dirname(path) : join(dirname(path), name.slice(0, -3));
}

/**
 * Every non-test `.rs` file under `dirs` with its non-test line count. A
 * file is test code by its path (`testByPath`), or because a parent
 * declares it `#[cfg(test)] mod x;`, or because it lies under such a
 * module's directory.
 * @param {string} root the repository root
 * @param {readonly string[]} dirs directories under the root to read
 * @returns {Map<string, number>} repository path to non-test lines
 */
export function productLines(root, dirs) {
  const all = dirs.flatMap((dir) => rustFiles(join(root, dir)));
  /** @type {Map<string, Shape>} */
  const shapes = new Map();
  /** @type {Set<string>} */
  const testDirs = new Set();
  /** @type {Set<string>} */
  const testFiles = new Set();
  for (const path of all) {
    const s = shape(readFileSync(path, "utf8"));
    shapes.set(path, s);
    for (const module of s.testModules) {
      const dir = join(childDir(path), module);
      testFiles.add(`${dir}.rs`);
      testDirs.add(dir + sep);
    }
  }
  /** @type {Map<string, number>} */
  const result = new Map();
  for (const path of all) {
    const name = repoPath(root, path);
    const s = shapes.get(path);
    if (
      s === undefined ||
      testByPath(name) ||
      testFiles.has(path) ||
      [...testDirs].some((dir) => path.startsWith(dir))
    ) {
      continue;
    }
    result.set(name, s.lines - s.testLines);
  }
  return result;
}
