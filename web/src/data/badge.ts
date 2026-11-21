// The page's badge and favicon (claims follow-up, fifth row; ADR 0089,
// "The page's badge and favicon"): how many workspaces want the person, on
// the app icon where the browser offers a badge, and whether anything runs
// or wants them, on the favicon.
//
// [Decision Log]
// - Purpose: Paseo shows the count on the desktop's dock and swaps the
//   favicon among none, running and attention (`use-favicon-status.ts`,
//   `desktop-badge-state.ts:5-14`). The page showed neither.
// - Paseo's rule, ported: each agent falls in one bucket, by priority
//   needs input, failed, running, attention, done
//   (`protocol/src/agent-state-bucket.ts:14-36`). A workspace takes the
//   highest-priority bucket of its agents (`workspace-directory.ts`). The
//   badge counts workspaces in needs input, failed or attention. The
//   favicon looks at the agents themselves: running when any runs, else
//   attention when any wants attention (`use-favicon-status.ts`).
// - Vessel's buckets: needs input is the node's `blocked` phrase (a
//   permission or a question waits); failed is a failed session; running is
//   a live one; attention is an unread one (`marks.unread`, Vessel's
//   "requires attention"); done is the rest. Archived sessions count for
//   nothing, as they are off the sidebar.
// - A workspace is the sidebar's: the session's project or worktree root
//   (`workspace/root.ts`), or the session alone when it has none.
// - Alternative: count sessions. It differs from Paseo wherever a workspace
//   holds several agents.

import type { NodeKey } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import { placement } from "./board";
import { isArchived, isUnread } from "./marks";
import { entriesOf } from "./records";
import { rootKey, sessionRoot } from "./workspace/root";

/** Paseo's workspace state buckets. */
export type Bucket =
  "needs_input" | "failed" | "running" | "attention" | "done";

/** Paseo's priority: the lower, the more it wins. */
const PRIORITY: Record<Bucket, number> = {
  needs_input: 0,
  failed: 1,
  running: 2,
  attention: 3,
  done: 4,
};

/** The bucket a session falls in. */
export function bucketOf(session: SessionInfo): Bucket {
  const placed = placement(session);
  if (placed.phrase.from === "node" && placed.phrase.status === "blocked") {
    return "needs_input";
  }
  const state = session.status.state;
  if (state === "failed") {
    return "failed";
  }
  if (state === "starting" || state === "running") {
    return "running";
  }
  return isUnread(session) ? "attention" : "done";
}

/** Each workspace's bucket, across every machine's sessions. */
export function workspaceBuckets(
  sessions: Readonly<Record<NodeKey, readonly SessionInfo[]>>,
): Bucket[] {
  const buckets = new Map<string, Bucket>();
  for (const [node, list] of entriesOf(sessions)) {
    for (const session of list) {
      if (isArchived(session)) {
        continue;
      }
      const root = sessionRoot(session);
      const key = `${node}\u0000${root === null ? `session:${session.id}` : rootKey(root)}`;
      const bucket = bucketOf(session);
      const earlier = buckets.get(key);
      if (earlier === undefined || PRIORITY[bucket] < PRIORITY[earlier]) {
        buckets.set(key, bucket);
      }
    }
  }
  return [...buckets.values()];
}

/** Paseo's badge count: workspaces in needs input, failed or attention;
 * `0` when none (the badge is then cleared). */
export function badgeCount(buckets: readonly Bucket[]): number {
  return buckets.filter(
    (bucket) =>
      bucket === "needs_input" || bucket === "failed" || bucket === "attention",
  ).length;
}

/** The favicon's state (Paseo's `deriveFaviconStatus`). */
export type FaviconStatus = "none" | "running" | "attention";

/** Paseo's `deriveFaviconStatus`, over the sessions themselves rather than
 * workspaces: running when any live session runs (a waiting one is still
 * running there, as Paseo's agent status is), else attention when any is
 * unread, else none. A failed session turns no favicon, as in Paseo. */
export function faviconStatus(
  sessions: Readonly<Record<NodeKey, readonly SessionInfo[]>>,
): FaviconStatus {
  const shown = entriesOf(sessions).flatMap(([, list]) =>
    list.filter((session) => !isArchived(session)),
  );
  if (
    shown.some(
      (session) =>
        session.status.state === "starting" ||
        session.status.state === "running",
    )
  ) {
    return "running";
  }
  return shown.some(isUnread) ? "attention" : "none";
}

/** The favicon's path for `status`, as the build writes them
 * (`vite-mark.ts`). */
export function faviconHref(status: FaviconStatus): string {
  return status === "none" ? "/favicon.svg" : `/favicon-${status}.svg`;
}
