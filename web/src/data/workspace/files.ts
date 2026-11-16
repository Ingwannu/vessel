// A project's or a worktree's files (`file.*`, protocols 10 and 11): typed calls
// in the shape of `data/calls.ts`. Finding by name is `find.ts`'s, the
// one wrapper of `file.find`.
//
// Every path is a `RelPath` taken from a listing or a search, never
// typed into a field as a whole; the node checks each against the root
// after resolving links, and answers one `not_found` for a path that is
// not there and one that leads out.

import type { NodeKey } from "../../wire/bridge";
import type {
  DirListing,
  FileRead,
  FileVersion,
  GrepCase,
  GrepMode,
  GrepResult,
  ObservedFile,
  RelPath,
  WorkspaceRoot,
  WriteExpect,
} from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** One folder's entries, folders first; ignored ones only when asked. */
export function listDir(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  dir: RelPath,
  ignored: boolean,
): Promise<Outcome<DirListing>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.list", { root, dir, ignored }),
  );
}

/** A file's saved state (`file.observe`, protocol 26, ADR 0105): at
 * once without `after`; else when it is no longer `after`, or at the
 * node's heartbeat with the same state. */
export function observeFile(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  after: ObservedFile | null,
): Promise<Outcome<ObservedFile>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.observe", { root, path, after }),
  );
}

/** A page of a file from byte `offset` (256 KiB, the node's default). */
export function readFile(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  offset: number,
): Promise<Outcome<FileRead>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.read", { root, path, offset }),
  );
}

/** Write `text` over `path` when `expect` holds (the content's hash read,
 * so a change made since is a `conflict` and nothing is written;
 * `save.ts` decides which). */
export function writeFile(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  text: string,
  expect: WriteExpect,
): Promise<Outcome<FileVersion>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.write", { root, path, text, expect }),
  );
}

/** Make a folder. */
export function createDir(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.create_dir", { root, path }),
  );
}

/** Rename or move `from` to `to`, never over an existing path. */
export function renamePath(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  from: RelPath,
  to: RelPath,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.rename", { root, from, to }),
  );
}

/** Delete a file, a link, or a folder (only an empty one unless
 * `recursive`). */
export function deletePath(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  path: RelPath,
  recursive: boolean,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.delete", { root, path, recursive }),
  );
}

/** What to look for in the files' contents. */
export interface Grep {
  query: string;
  mode: GrepMode;
  case: GrepCase;
}

/** Lines of files under `root` that hold `grep.query` (tracked and
 * untracked files that are not ignored; binary files left out). */
export function searchFiles(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  grep: Grep,
): Promise<Outcome<GrepResult>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.grep", {
      root,
      query: grep.query,
      mode: grep.mode,
      case: grep.case,
    }),
  );
}
