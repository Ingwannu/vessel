import { describe, expect, test } from "vitest";

import type {
  InboxEntry,
  InboxItem,
  InboxPage,
  NotificationClaim,
  NotificationSource,
  SessionInfo,
} from "../wire/proto";
import { fixtureId } from "../testing/ids";
import {
  ATTENTION_NOTIFICATIONS_PROTOCOL,
  askBeforeTelling,
  type ClaimCalls,
  admitsClaims,
  claimsOn,
  shows,
  sourceFor,
} from "./claims";
import type { Notice, NoticeKind } from "./notices";

const NODE = fixtureId("NodeKey", "lab");

function session(id: string): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: `/work/${id}` },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: "2026-11-21T00:00:00Z",
    next_seq: 0,
  };
}

function notice(id: string, kind: NoticeKind): Notice {
  return { kind, node: NODE, session: session(id) };
}

function entry(
  cursor: number,
  id: string,
  seq: number,
  read: "read" | "unread",
  item: InboxItem,
): InboxEntry {
  return {
    cursor,
    key: { session: fixtureId("SessionId", id), seq },
    title: null,
    project: null,
    project_name: null,
    at: "2026-11-21T00:00:00Z",
    read,
    item,
  };
}

const finished: InboxItem = { kind: "finished" };
const limit: InboxItem = { kind: "limit" };
const failed: InboxItem = { kind: "failed" };

function page(entries: InboxEntry[]): InboxPage {
  return {
    counts: { total: entries.length, waiting: 0, unread: entries.length },
    entries,
    next: null,
  };
}

describe("claims (ADR 0089), page side", () => {
  test("no machine claims while the attention protocol is unassigned", () => {
    if (ATTENTION_NOTIFICATIONS_PROTOCOL === null) {
      expect(claimsOn(undefined)).toBe(false);
    }
    expect(claimsOn({ state: "idle" })).toBe(false);
  });

  test("a read goes as inbox.read only on a machine that admits the protocol (row 4)", () => {
    // While unassigned, every machine keeps the read mark, at any protocol.
    expect(admitsClaims(Number.MAX_SAFE_INTEGER)).toBe(
      ATTENTION_NOTIFICATIONS_PROTOCOL !== null,
    );
    expect(admitsClaims(null)).toBe(false);
  });

  test("the source is the newest unread entry of the session that matches the kind", () => {
    const entries = [
      entry(9, "other", 40, "unread", finished),
      entry(8, "here", 30, "read", finished),
      entry(7, "here", 20, "unread", limit),
      entry(6, "here", 10, "unread", finished),
      entry(5, "here", 5, "unread", failed),
    ];
    expect(sourceFor(entries, notice("here", "finished"))).toEqual({
      target: "session",
      key: { session: fixtureId("SessionId", "here"), seq: 20 },
      cursor: 7,
    });
    expect(sourceFor(entries, notice("here", "needs-you"))).toBeNull();
    expect(sourceFor(entries.slice(4), notice("here", "finished"))).toBeNull();
    expect(
      sourceFor(entries.slice(0, 1), notice("here", "finished")),
    ).toBeNull();
  });

  test("only admitted, or an entry past the first page, shows", () => {
    expect(shows({ kind: "answered", claim: "admitted" })).toBe(true);
    expect(shows({ kind: "not_on_first_page" })).toBe(true);
    const quiet: NotificationClaim[] = [
      "not_recipient",
      "suppressed",
      "no_attention",
      "already_claimed",
      "no_present_client",
      "unknown",
    ];
    for (const claim of quiet) {
      expect(shows({ kind: "answered", claim })).toBe(false);
    }
    expect(shows({ kind: "failed" })).toBe(false);
  });

  test("one first page, a claim per notice found, never retried; past the page falls back", async () => {
    const claimed: NotificationSource[] = [];
    const answers = new Map<string, NotificationClaim | null>([
      ["admitted", "admitted"],
      ["elsewhere", "not_recipient"],
      ["lost", null],
    ]);
    let pages = 0;
    const calls: ClaimCalls = {
      firstPage: () => {
        pages += 1;
        return Promise.resolve(
          page([
            entry(3, "admitted", 3, "unread", finished),
            entry(2, "elsewhere", 2, "unread", finished),
            entry(1, "lost", 1, "unread", finished),
          ]),
        );
      },
      claim: (source) => {
        claimed.push(source);
        const id = source.target === "session" ? source.key.session : "";
        return Promise.resolve(answers.get(id) ?? null);
      },
    };
    const told = await askBeforeTelling(calls, [
      notice("admitted", "finished"),
      notice("elsewhere", "finished"),
      notice("lost", "finished"),
      notice("past-the-page", "finished"),
    ]);
    expect(told.map((each) => each.session.id)).toEqual([
      fixtureId("SessionId", "admitted"),
      fixtureId("SessionId", "past-the-page"),
    ]);
    expect(pages).toBe(1);
    expect(claimed).toHaveLength(3);
  });

  test("a first page that cannot be read falls back to telling every notice, claiming none", async () => {
    let claims = 0;
    const calls: ClaimCalls = {
      firstPage: () => Promise.resolve(null),
      claim: () => {
        claims += 1;
        return Promise.resolve("admitted");
      },
    };
    const notices = [notice("one", "finished"), notice("two", "needs-you")];
    expect(await askBeforeTelling(calls, notices)).toEqual(notices);
    expect(claims).toBe(0);
  });
});
