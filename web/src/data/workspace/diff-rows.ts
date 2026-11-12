// A file's diff as rows for a windowed list: a header per hunk, then its
// lines with their numbers on each side (agent-orchestrator's
// `WorkspaceDiffView`, GOAT's and Paseo's unified diff).
//
// [Decision Log]
// - Purpose: the Changes tab draws every changed file's diff one under
//   another, and a diff may be thousands of lines; "never render a whole
//   large diff at once".
// - Constraint: the node pages a file's diff by hunk (5,000 lines a
//   call, `more_from`), so a file's rows grow as the reader goes; the
//   list is windowed (`useWindow`), which wants a stable key per row.
// - Choice: one flat array of rows per file, keys from the hunk's index
//   and the line's place in it (stable as pages are added), the old and
//   new line numbers worked out here once, so a drawn row only reads.
// - Cost: a 5,000-line page is 5,000 small objects; the rows of a file
//   are made once per page, not per render.

import type { DiffLine, FileDiff, Hunk, LineKind } from "../../wire/proto";

/** One row of a file's diff. */
export type DiffRow =
  | {
      kind: "hunk";
      key: string;
      /** `@@ -12,7 +12,9 @@`, as git writes it. */
      header: string;
      /** The function or section git names after it, if any. */
      section: string | null;
    }
  | {
      kind: "line";
      key: string;
      line: LineKind;
      text: string;
      /** Whether the node cut the line (over 4 KiB). */
      cut: boolean;
      /** Its number on the old side, when it is there. */
      old: number | null;
      /** Its number on the new side, when it is there. */
      new: number | null;
    };

/** A hunk's header as git writes it. */
export function hunkHeader(hunk: Hunk): string {
  return `@@ -${String(hunk.old_start)},${String(hunk.old_lines)} +${String(hunk.new_start)},${String(hunk.new_lines)} @@`;
}

/** The rows of `hunk`, the `index`th of its file. */
function hunkRows(hunk: Hunk, index: number, prefix: string): DiffRow[] {
  const rows: DiffRow[] = [
    {
      kind: "hunk",
      key: `${prefix}h${String(index)}`,
      header: hunkHeader(hunk),
      section: hunk.section ?? null,
    },
  ];
  let old = hunk.old_start;
  let next = hunk.new_start;
  hunk.lines.forEach((line: DiffLine, at) => {
    const numbers = numbered(line.kind, old, next);
    old = numbers.nextOld;
    next = numbers.nextNew;
    rows.push({
      kind: "line",
      key: `${prefix}h${String(index)}:${String(at)}`,
      line: line.kind,
      text: line.text,
      cut: line.cut ?? false,
      old: numbers.old,
      new: numbers.new,
    });
  });
  return rows;
}

/** A line's numbers on each side, and the numbers after it. */
function numbered(
  kind: LineKind,
  old: number,
  next: number,
): {
  old: number | null;
  new: number | null;
  nextOld: number;
  nextNew: number;
} {
  switch (kind) {
    case "context":
      return { old, new: next, nextOld: old + 1, nextNew: next + 1 };
    case "added":
      return { old: null, new: next, nextOld: old, nextNew: next + 1 };
    case "removed":
      return { old, new: null, nextOld: old + 1, nextNew: next };
    case "no_newline":
      return { old: null, new: null, nextOld: old, nextNew: next };
  }
}

/** The rows of a page of `diff`'s hunks, the first being hunk number
 * `first` of the file (0 for the first page, the previous page's
 * `more_from` after). `prefix` keeps keys apart between files. */
export function diffRows(
  hunks: readonly Hunk[],
  first: number,
  prefix: string,
): DiffRow[] {
  return hunks.flatMap((hunk, index) => hunkRows(hunk, first + index, prefix));
}

/** What a file's diff holds, for its header and its body. */
export type DiffBody =
  | { kind: "rows"; rows: readonly DiffRow[]; more: number | null }
  | { kind: "binary" }
  | { kind: "submodule"; from: string | null; to: string | null }
  | { kind: "too_large"; bytes: number }
  | { kind: "unchanged" }
  | { kind: "unknown" };

/** `page` added to what was read of the file so far (`before`, `null`
 * for the first page). */
export function addPage(
  before: DiffBody | null,
  page: FileDiff,
  prefix: string,
): DiffBody {
  const content = page.content;
  switch (content.kind) {
    case "hunks": {
      const kept = before?.kind === "rows" ? before.rows : [];
      const first = before?.kind === "rows" ? (before.more ?? 0) : 0;
      return {
        kind: "rows",
        rows: [...kept, ...diffRows(content.hunks, first, prefix)],
        more: page.more_from ?? null,
      };
    }
    case "binary":
      return { kind: "binary" };
    case "submodule":
      return {
        kind: "submodule",
        from: content.from ?? null,
        to: content.to ?? null,
      };
    case "too_large":
      return { kind: "too_large", bytes: content.bytes };
    case "unchanged":
      return { kind: "unchanged" };
    case "unknown":
      return { kind: "unknown" };
  }
}

/** The lines `rows` add and remove (a page's own count, for a file whose
 * summary had none). */
export function countLines(rows: readonly DiffRow[]): {
  added: number;
  removed: number;
} {
  let added = 0;
  let removed = 0;
  for (const row of rows) {
    if (row.kind === "line" && row.line === "added") {
      added += 1;
    } else if (row.kind === "line" && row.line === "removed") {
      removed += 1;
    }
  }
  return { added, removed };
}
