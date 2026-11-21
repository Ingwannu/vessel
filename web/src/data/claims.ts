// A notice shown only once its machine admits this window as the one
// recipient (ADR 0089 `notification.claim`; ADR 0114 "Claims"). This is the
// claims follow-up's second row; the app's
// `vessel_app_view::notices::claims` holds the same rules.
//
// [Decision Log]
// - Purpose: two open windows both notified, which is ADR 0114's stated
//   difference. Paseo elects one client (`websocket-server.ts:2838-3010`).
//   With ADR 0089's protocol, a window asks its machine before it shows.
// - Constraint: the protocol is unassigned. `claimsOn` is false for every
//   machine until it is, and until then a notice takes ADR 0114's path
//   unchanged. The number comes from the generated `wire/proto`, so
//   assigning it on the node line turns the page's claims on too.
// - Source: a claim names the exact inbox entry, by key and cursor. The
//   only way to learn them is `inbox.list`: newest first, sixteen a page,
//   no session filter. The page reads the machine's first page once per
//   change. For each notice it takes the newest unread entry of that
//   session whose item matches the notice's kind (`sourceFor`).
// - Not on the first page: the notice falls back to ADR 0114's path and
//   shows (the coordinator's rule, 2026-11-20). ADR 0089 states the limit.
//   A first page that cannot be read does the same for every notice
//   (2026-11-21, at assignment).
// - Otherwise only `admitted` shows. Every other outcome, a failed call and
//   a lost reply show nothing, and a claim is never retried (ADR 0089).

import type { NodeKey, NodeStatus } from "../wire/bridge";
import { ATTENTION_NOTIFICATIONS_PROTOCOL as ASSIGNED_ATTENTION_PROTOCOL } from "../wire/proto";
import type {
  InboxEntry,
  InboxItem,
  EventSeq,
  InboxPage,
  NotificationClaim,
  NotificationSource,
  SessionId,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import type { Notice, NoticeKind } from "./notices";
import { attempt, type Outcome } from "./outcome";

/** vessel-proto's `ATTENTION_NOTIFICATIONS_PROTOCOL`, from the generated
 * `wire/proto` (`ts.rs` PROTOCOLS): `null` until the node line assigns it,
 * and then that number with no edit here. Annotated wider than either
 * value, so neither a `null` nor a number makes a guard below look
 * constant to the type checker. */
export const ATTENTION_NOTIFICATIONS_PROTOCOL: number | null =
  ASSIGNED_ATTENTION_PROTOCOL;

/** Whether a machine at `protocol` admits ADR 0089's protocol: never
 * while it is unassigned. */
export function admitsClaims(protocol: number | null): boolean {
  return (
    ATTENTION_NOTIFICATIONS_PROTOCOL !== null &&
    protocol !== null &&
    protocol >= ATTENTION_NOTIFICATIONS_PROTOCOL
  );
}

/** Whether `status` is a connected machine that admits claims. */
export function claimsOn(status: NodeStatus | undefined): boolean {
  return status?.state === "connected" && admitsClaims(status.protocol);
}

/** Whether an inbox item is what `kind` told of: a request for "needs
 * you", a turn's end for "finished". */
function toldOf(kind: NoticeKind, item: InboxItem): boolean {
  switch (kind) {
    case "needs-you":
      return (
        item.kind === "permission" ||
        item.kind === "question" ||
        item.kind === "in_session"
      );
    case "finished":
      return item.kind === "finished" || item.kind === "limit";
  }
}

/** The exact source to claim for `notice`: the newest unread entry of its
 * session that matches its kind, in `entries` (a first page, newest
 * first). `null` when the first page does not hold it. */
export function sourceFor(
  entries: readonly InboxEntry[],
  notice: Notice,
): NotificationSource | null {
  const entry = entries.find(
    (each) =>
      each.key.session === notice.session.id &&
      each.read === "unread" &&
      toldOf(notice.kind, each.item),
  );
  return entry === undefined
    ? null
    : { target: "session", key: entry.key, cursor: entry.cursor };
}

/** What asking came to for one notice. */
export type Asked =
  | { kind: "not_on_first_page" }
  | { kind: "answered"; claim: NotificationClaim }
  | { kind: "failed" };

/** Whether the page shows the notice after asking. */
export function shows(asked: Asked): boolean {
  return (
    asked.kind === "not_on_first_page" ||
    (asked.kind === "answered" && asked.claim === "admitted")
  );
}

/** What a machine's calls give the claims: a first inbox page, a claim. */
export interface ClaimCalls {
  firstPage: () => Promise<InboxPage | null>;
  claim: (source: NotificationSource) => Promise<NotificationClaim | null>;
}

/** The calls on `node`; a failure is `null`. */
export function claimCalls(fleet: Fleet, node: NodeKey): ClaimCalls {
  return {
    async firstPage() {
      const page = await attempt(() =>
        fleet.bridge.call(node, "inbox.list", { before: null }),
      );
      return page.ok ? page.value : null;
    },
    async claim(source) {
      const claimed = await attempt(() =>
        fleet.bridge.call(node, "notification.claim", { source }),
      );
      return claimed.ok ? claimed.value : null;
    },
  };
}

/** Mark `session` read through `through` in the inbox (`inbox.read`,
 * claims row 4), in place of `session.mark read` on a machine that admits
 * ADR 0089's protocol. */
export function readInbox(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  through: EventSeq,
): Promise<Outcome<unknown>> {
  return attempt(() =>
    fleet.bridge.call(node, "inbox.read", { session, through }),
  );
}

/** Ask a machine about its notices: one first page, then a claim per
 * notice found on it, never retried. Gives back the notices to tell. */
export async function askBeforeTelling(
  calls: ClaimCalls,
  notices: readonly Notice[],
): Promise<Notice[]> {
  const page = await calls.firstPage();
  if (page === null) {
    // A first page that could not be read names no source, as a notice
    // past it does: every notice takes ADR 0114's path and is told, rather
    // than a transient failure silencing them (ADR 0089, at assignment).
    return [...notices];
  }
  const asked = await Promise.all(
    notices.map(async (notice): Promise<[Notice, Asked]> => {
      const source = sourceFor(page.entries, notice);
      if (source === null) {
        return [notice, { kind: "not_on_first_page" }];
      }
      const claim = await calls.claim(source);
      return [
        notice,
        claim === null ? { kind: "failed" } : { kind: "answered", claim },
      ];
    }),
  );
  const told = asked
    .filter(([, outcome]) => shows(outcome))
    .map(([notice]) => notice);
  return told;
}
