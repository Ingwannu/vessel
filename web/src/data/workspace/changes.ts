// The Changes tab's model: which comparison it shows (agent-orchestrator's
// changes picker), each file's header (its status letter and counts), and
// every changed file's diff as one list of rows for one windowed list.
//
// [Decision Log]
// - Purpose: agent-orchestrator's review pane draws every changed file
//   one under another, each a 36px header over its diff, collapsible;
//   "a diff ... is paged ...: the page asks for more when the reader gets
//   there ... never renders a whole large diff at once".
// - Constraint: the node answers the summary (at most 5,000 files) and a
//   file's diff (5,000 lines a page) separately; diffs are asked for only
//   when their place in the list is drawn.
// - Choice: one flat list of rows, a file's header then, when it is open,
//   its diff's rows or a row saying what the diff is (loading, binary,
//   too large) or that there is more of it. The drawn window asks for
//   the diffs and the pages its rows stand for.
// - Cost: the list is rebuilt when a page arrives (an array of the rows
//   already made, so the cost is the rows' count, not their text).

import type {
  ChangeStatus,
  DiffScope,
  FileChange,
  RelPath,
} from "../../wire/proto";
import { type DiffBody, type DiffRow } from "./diff-rows";

/** The comparisons the picker offers, in its order. */
export type ScopeKind = "base" | "uncommitted" | "staged" | "unstaged";

/** What the picker calls a comparison. */
export function scopeLabel(scope: DiffScope): string {
  switch (scope.kind) {
    case "base":
      return "Changes";
    case "uncommitted":
      return "Uncommitted";
    case "staged":
      return "Staged";
    case "unstaged":
      return "Unstaged";
    case "commit":
      return `Commit ${scope.id.slice(0, 7)}`;
  }
}

/** What a comparison is, in one line under its name. */
export function scopeDetail(kind: ScopeKind): string {
  switch (kind) {
    case "base":
      return "Everything since the base branch";
    case "uncommitted":
      return "Not committed yet";
    case "staged":
      return "Staged for the next commit";
    case "unstaged":
      return "Not staged yet";
  }
}

/** The empty comparison's words. */
export function scopeEmpty(scope: DiffScope): string {
  switch (scope.kind) {
    case "base":
      return "No changes against base.";
    case "uncommitted":
    case "unstaged":
      return "No changes against HEAD.";
    case "staged":
      return "Nothing is staged.";
    case "commit":
      return "This commit changes nothing.";
  }
}

/** A file's status as its header's letter, and the tone it is drawn in. */
export function statusLetter(status: ChangeStatus): {
  letter: string;
  tone: "added" | "removed" | "modified" | "renamed" | "other";
  word: string;
} {
  switch (status) {
    case "added":
      return { letter: "A", tone: "added", word: "Added" };
    case "deleted":
      return { letter: "D", tone: "removed", word: "Deleted" };
    case "modified":
      return { letter: "M", tone: "modified", word: "Modified" };
    case "renamed":
      return { letter: "R", tone: "renamed", word: "Renamed" };
    case "copied":
      return { letter: "C", tone: "renamed", word: "Copied" };
    case "type_changed":
      return { letter: "T", tone: "modified", word: "Type changed" };
    case "conflicted":
      return { letter: "U", tone: "removed", word: "Conflicted" };
    case "unknown":
      return { letter: "?", tone: "other", word: "Changed" };
  }
}

/** A path as its folder (with its slash) and its name, drawn apart. */
export function splitPath(path: RelPath): { dir: string; name: string } {
  const at = path.lastIndexOf("/");
  return at < 0
    ? { dir: "", name: path }
    : { dir: path.slice(0, at + 1), name: path.slice(at + 1) };
}

/** What one file's diff is known to be. */
export type FileState =
  | { kind: "unread" }
  | { kind: "loading"; body: DiffBody | null }
  | { kind: "read"; body: DiffBody }
  | { kind: "failed"; message: string };

/** One row of the Changes list. */
export type ChangeRow =
  | { kind: "file"; key: string; change: FileChange; open: boolean }
  | DiffRow
  | { kind: "note"; key: string; path: RelPath; note: FileNote };

/** What a row in place of (or after) a file's diff says. */
export type FileNote =
  | { kind: "loading" }
  | { kind: "more" }
  | { kind: "binary" }
  | { kind: "too_large"; bytes: number }
  | { kind: "submodule"; from: string | null; to: string | null }
  | { kind: "unchanged" }
  | { kind: "unknown" }
  | { kind: "failed"; message: string };

/** The key prefix of `path`'s rows. */
export function rowPrefix(path: RelPath): string {
  return `${path}\u0000`;
}

function bodyRows(path: RelPath, body: DiffBody): ChangeRow[] {
  const note = (value: FileNote): ChangeRow => ({
    kind: "note",
    key: `${rowPrefix(path)}${value.kind}`,
    path,
    note: value,
  });
  switch (body.kind) {
    case "rows":
      return body.more === null
        ? [...body.rows]
        : [...body.rows, note({ kind: "more" })];
    case "binary":
      return [note({ kind: "binary" })];
    case "submodule":
      return [note({ kind: "submodule", from: body.from, to: body.to })];
    case "too_large":
      return [note({ kind: "too_large", bytes: body.bytes })];
    case "unchanged":
      return [note({ kind: "unchanged" })];
    case "unknown":
      return [note({ kind: "unknown" })];
  }
}

/** Every row of the list: each file's header and, when open, its diff. */
export function changeRows(
  files: readonly FileChange[],
  closed: ReadonlySet<RelPath>,
  states: ReadonlyMap<RelPath, FileState>,
): ChangeRow[] {
  const rows: ChangeRow[] = [];
  for (const change of files) {
    const open = !closed.has(change.path);
    rows.push({ kind: "file", key: rowPrefix(change.path), change, open });
    if (!open) {
      continue;
    }
    const state = states.get(change.path) ?? { kind: "unread" };
    switch (state.kind) {
      case "unread":
        rows.push({
          kind: "note",
          key: `${rowPrefix(change.path)}loading`,
          path: change.path,
          note: { kind: "loading" },
        });
        break;
      case "loading":
        rows.push(
          ...(state.body === null
            ? [
                {
                  kind: "note" as const,
                  key: `${rowPrefix(change.path)}loading`,
                  path: change.path,
                  note: { kind: "loading" as const },
                },
              ]
            : bodyRows(change.path, state.body)),
        );
        break;
      case "read":
        rows.push(...bodyRows(change.path, state.body));
        break;
      case "failed":
        rows.push({
          kind: "note",
          key: `${rowPrefix(change.path)}failed`,
          path: change.path,
          note: { kind: "failed", message: state.message },
        });
        break;
    }
  }
  return rows;
}

/** Added and removed lines over every file the summary counted. */
export function totals(files: readonly FileChange[]): {
  added: number;
  removed: number;
} {
  let added = 0;
  let removed = 0;
  for (const file of files) {
    added += file.additions ?? 0;
    removed += file.deletions ?? 0;
  }
  return { added, removed };
}

/** Whether a file's diff read before still stands for `change`: the
 * same status and counts (a refresh keeps it, else it is read again). */
export function sameChange(before: FileChange, change: FileChange): boolean {
  return (
    before.status === change.status &&
    before.additions === change.additions &&
    before.deletions === change.deletions &&
    before.old_path === change.old_path
  );
}
