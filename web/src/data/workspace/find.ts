// Files of a project or a worktree found by name (`file.find`, protocol
// 10): the one wrapper of that call.
//
// [Decision Log]
// - Purpose: two places search files by name as a person types: the
//   composer's `@` and the Files panel's open-by-name. They were built in
//   two tracks at once (milestone 11, wave 2), and milestone 10 showed
//   what that does to a shared need: two modules for one call, joined
//   afterwards. This module was written before either track started, so
//   both import it and neither writes its own.
// - Constraint: the search runs on every keystroke against a repository
//   that may hold hundreds of thousands of paths. The node stops an older
//   search when a newer one arrives with the same key from the same
//   connection, and answers the older one `superseded` with no matches.
// - Choice: a field takes one key for its life (`newSearchKey`) and sends
//   it with every search; an answer marked `superseded` is the caller's
//   to drop, since a newer answer is on its way. Nothing is debounced
//   here: the node's replacement is the bound, and a field that wants to
//   wait before asking decides so where it is drawn.
// - Every other call about projects, worktrees, git and files lives in
//   this directory too, one module per subject, in the shape of
//   `data/calls.ts` (a typed result or a typed failure, nothing thrown).

import { SEARCH_KEY_MAX, searchKeyOf } from "../../bridge/ids";
import type { NodeKey } from "../../wire/bridge";
import type { FoundFiles, SearchKey, WorkspaceRoot } from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** The key after `last` (the first is 1; after the largest the node takes,
 * the count starts again). */
export function nextSearchKey(last: number): number {
  return last >= SEARCH_KEY_MAX ? 1 : last + 1;
}

let last = 0;

/** A key for one field's searches, different from every other field's on
 * this page: taken once when the field appears and sent with each of its
 * searches. */
export function newSearchKey(): SearchKey {
  last = nextSearchKey(last);
  return searchKeyOf(last);
}

/** The files under `root` whose path matches `query`, best first, asked
 * of `node`. `key` is the asking field's (`newSearchKey`). An answer with
 * `superseded` set was replaced by the field's next search and has no
 * matches: drop it. */
export function findFiles(
  fleet: Fleet,
  node: NodeKey,
  root: WorkspaceRoot,
  query: string,
  key: SearchKey,
): Promise<Outcome<FoundFiles>> {
  return attempt(() =>
    fleet.bridge.call(node, "file.find", { root, query, key }),
  );
}
