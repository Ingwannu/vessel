// The Files tab's tree as rows for a windowed list: the folders opened so
// far, each listed once by the node (`file.list`, folders first, at most
// 5,000 entries a folder), flattened in order with their depth.
//
// [Decision Log]
// - Purpose: agent-orchestrator's file tree, which lists a folder when it
//   is opened, not the whole repository up front.
// - Constraint: a repository may hold hundreds of thousands of files; a
//   folder's listing is bounded and says `more`; the page draws a window
//   of rows (`useWindow`).
// - Choice: the listings by folder path, the open folders as a set, and a
//   pure flattening into rows; a listing that has not arrived yet is one
//   "loading" row under its folder, which the drawn window asks for.
// - Cost: a folder closed and opened again is not listed again until the
//   tab refreshes; a file an agent adds meanwhile shows on the refresh.

import type { DirEntry, DirListing, RelPath } from "../../wire/proto";

/** What is known of one folder's entries. */
export type Listing =
  | { kind: "loading" }
  | { kind: "listed"; listing: DirListing }
  | { kind: "failed"; message: string };

/** One row of the tree. */
export type TreeRow =
  | {
      kind: "entry";
      key: string;
      path: RelPath;
      entry: DirEntry;
      depth: number;
      open: boolean;
    }
  | { kind: "loading"; key: string; dir: RelPath; depth: number }
  | { kind: "failed"; key: string; message: string; depth: number }
  | { kind: "more"; key: string; depth: number }
  | { kind: "empty"; key: string; depth: number }
  /** A name being typed: a new entry's, or one renamed in its place. */
  | {
      kind: "edit";
      key: string;
      depth: number;
      entry: "file" | "dir";
      initial: string;
    };

/** A name being typed in the tree (Paseo's EntryNameInputRow): a new file
 * or folder in `parent`, or `path` renamed. */
export type TreeEdit =
  | { kind: "create"; parent: RelPath; entry: "file" | "dir" }
  | { kind: "rename"; path: RelPath; entry: "file" | "dir"; name: string };

/** `name` inside `dir` (`""`: the root). */
export function childPath(dir: RelPath, name: string): RelPath {
  return dir === "" ? name : `${dir}/${name}`;
}

/** The folder `path` is in (`""`: the root). */
export function parentOf(path: RelPath): RelPath {
  const at = path.lastIndexOf("/");
  return at < 0 ? "" : path.slice(0, at);
}

/** The rows of the tree under `dir`, `depth` deep. */
export function treeRows(
  listings: ReadonlyMap<RelPath, Listing>,
  open: ReadonlySet<RelPath>,
  dir: RelPath = "",
  depth = 0,
): TreeRow[] {
  const listing = listings.get(dir);
  if (listing === undefined || listing.kind === "loading") {
    return [{ kind: "loading", key: `loading:${dir}`, dir, depth }];
  }
  if (listing.kind === "failed") {
    return [
      { kind: "failed", key: `failed:${dir}`, message: listing.message, depth },
    ];
  }
  const rows: TreeRow[] = [];
  if (listing.listing.entries.length === 0) {
    rows.push({ kind: "empty", key: `empty:${dir}`, depth });
  }
  for (const entry of listing.listing.entries) {
    const path = childPath(dir, entry.name);
    const isOpen = entry.kind === "dir" && open.has(path);
    rows.push({ kind: "entry", key: path, path, entry, depth, open: isOpen });
    if (isOpen) {
      rows.push(...treeRows(listings, open, path, depth + 1));
    }
  }
  if (listing.listing.more) {
    rows.push({ kind: "more", key: `more:${dir}`, depth });
  }
  return rows;
}

/** The folders on the way to `path`, outermost first (to open them). */
export function foldersOf(path: RelPath): RelPath[] {
  const parts = path.split("/");
  return parts
    .slice(0, -1)
    .map((_, index) => parts.slice(0, index + 1).join("/"));
}

/** `path` in the folder `folder` on the node, as the node writes paths
 * (Paseo's Copy path: the workspace root joined): a Windows folder keeps
 * its backslashes. */
export function absolutePath(folder: string, path: RelPath): string {
  const windows = folder.includes("\\") && !folder.includes("/");
  const separator = windows ? "\\" : "/";
  const base = folder.replace(/[\\/]+$/, "");
  if (path === "") return base === "" ? separator : base;
  const relative = windows ? path.replaceAll("/", "\\") : path;
  return `${base}${separator}${relative}`;
}

/** `rows` with `edit`'s name row: a new entry's first in its folder (the
 * root's at the top), a renamed one in the entry's place. */
export function withEdit(
  rows: readonly TreeRow[],
  edit: TreeEdit | null,
): TreeRow[] {
  if (edit === null) return [...rows];
  if (edit.kind === "rename") {
    return rows.map((row) =>
      row.kind === "entry" && row.path === edit.path
        ? {
            kind: "edit",
            key: `edit:${edit.path}`,
            depth: row.depth,
            entry: edit.entry,
            initial: edit.name,
          }
        : row,
    );
  }
  const at =
    edit.parent === ""
      ? -1
      : rows.findIndex(
          (row) => row.kind === "entry" && row.path === edit.parent,
        );
  const parent = at < 0 ? null : rows[at];
  const depth = parent?.kind === "entry" ? parent.depth + 1 : 0;
  const row: TreeRow = {
    kind: "edit",
    key: `edit:${edit.parent}/`,
    depth,
    entry: edit.entry,
    initial: "",
  };
  return [...rows.slice(0, at + 1), row, ...rows.slice(at + 1)];
}

/** Why `name` cannot name an entry (Paseo's file explorer service's
 * words), or `null`. */
export function nameProblem(name: string): string | null {
  const trimmed = name.trim();
  if (trimmed === "" || trimmed === "." || trimmed === "..") {
    return "Invalid name";
  }
  return /[\\/]/.test(trimmed) ? "Name cannot contain path separators" : null;
}

/** Whether `path` is `dir` or inside it. */
export function isWithin(path: RelPath, dir: RelPath): boolean {
  return path === dir || path.startsWith(`${dir}/`);
}

/** The open folders once `from` is renamed `to` (Paseo's
 * replaceExplorerPathPrefix over its expanded paths). */
export function renamedOpen(
  open: ReadonlySet<RelPath>,
  from: RelPath,
  to: RelPath,
): Set<RelPath> {
  return new Set(
    [...open].map((path) =>
      isWithin(path, from) ? `${to}${path.slice(from.length)}` : path,
    ),
  );
}

/** The open folders once `gone` is deleted. */
export function withoutWithin(
  open: ReadonlySet<RelPath>,
  gone: RelPath,
): Set<RelPath> {
  return new Set([...open].filter((path) => !isWithin(path, gone)));
}
