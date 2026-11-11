import { describe, expect, it } from "vitest";

import type {
  Creator,
  DisplayStatus,
  KanbanColumn,
  SessionBoard,
  SessionInfo,
  SessionStatus,
} from "../wire/proto";
import {
  board,
  displayStatusLabel,
  isWorking,
  laneLabel,
  LANES,
  needsYou,
  phraseLabel,
  placement,
  sinceLabel,
  tone,
} from "./board";
import type { Row } from "./tree";

const running: SessionStatus = { state: "running" };
const starting: SessionStatus = { state: "starting" };
const exited: SessionStatus = { state: "exited", code: 0 };
const failed: SessionStatus = { state: "failed", message: "no login" };

function placed(
  column: KanbanColumn,
  display_status: DisplayStatus,
  at = "2026-11-07T10:00:00Z",
): SessionBoard {
  return {
    column,
    display_status,
    activity: "idle",
    last_activity_at: at,
  };
}

function session(
  id: string,
  status: SessionStatus,
  on?: SessionBoard,
): SessionInfo {
  const created_by: Creator = { kind: "owner" };
  return {
    id,
    spec: { harness: "claude-code", mode: "structured" },
    created_by,
    status,
    created_at: "2026-11-07T09:00:00Z",
    next_seq: 0,
    ...(on === undefined ? {} : { board: on }),
  };
}

function row(info: SessionInfo): Row {
  return {
    node: "laptop",
    session: info,
    depth: 0,
    parent: null,
    parentNode: null,
  };
}

describe("the sessions board", () => {
  it("places a session where its node says", () => {
    const at = placement(
      session("a", running, placed("needs_review", "changes_requested")),
    );
    expect(at.column).toBe("needs_review");
    expect(phraseLabel(at.phrase)).toBe("Changes requested");
    expect(at.activity).toBe("idle");
  });

  it("keeps the place a status implies, for a node that sends none", () => {
    const live = placement(session("a", running));
    expect(live.column).toBe("building");
    // Nothing says what the agent does, so the card reads the status.
    expect(phraseLabel(live.phrase)).toBe("running");
    expect(live.activity).toBeNull();
    expect(placement(session("b", starting)).column).toBe("building");
    const ended = placement(session("c", exited));
    expect(ended.column).toBe("archive");
    expect(phraseLabel(ended.phrase)).toBe("exited 0");
    expect(placement(session("d", failed)).column).toBe("archive");
  });

  it("names the lanes and the phrases as the reference does", () => {
    expect(LANES.map(laneLabel)).toEqual([
      "Building",
      "Validating",
      "In review",
      "Ready",
    ]);
    expect(displayStatusLabel("awaiting_pr")).toBe("Awaiting PR");
    expect(displayStatusLabel("fixing_ci")).toBe("Fixing CI failures");
    expect(displayStatusLabel("closed")).toBe("Closed without merge");
    expect(displayStatusLabel("needs_human_review")).toBe("Needs human review");
  });

  it("marks who waits on the owner and what is turning", () => {
    const at = (status: DisplayStatus) =>
      placement(session("a", running, placed("building", status)));
    for (const status of [
      "blocked",
      "ci_failing",
      "changes_requested",
    ] as const) {
      expect(needsYou(at(status)), status).toBe(true);
    }
    expect(needsYou(at("working"))).toBe(false);
    expect(needsYou(at("commented"))).toBe(false);
    // An older node never says a session needs its owner.
    expect(needsYou(placement(session("a", running)))).toBe(false);
    for (const status of [
      "working",
      "fixing_ci",
      "addressing_comments",
      "reviewing",
      "review_pending",
    ] as const) {
      expect(isWorking(at(status)), status).toBe(true);
    }
    expect(isWorking(at("mergeable"))).toBe(false);
    expect(isWorking(at("blocked"))).toBe(false);
    expect(isWorking(placement(session("a", starting)))).toBe(true);
    expect(isWorking(placement(session("a", running)))).toBe(false);
  });

  it("colours a status line by its lane, unless the phrase says more", () => {
    const at = (column: KanbanColumn, status: DisplayStatus) =>
      placement(session("a", running, placed(column, status)));
    expect(tone(at("building", "working"))).toBe("lane");
    expect(tone(at("building", "blocked"))).toBe("needs-you");
    expect(tone(at("ready", "mergeable"))).toBe("ready");
    expect(tone(at("ready", "merged"))).toBe("merged");
    expect(tone(at("ready", "closed"))).toBe("danger");
    expect(tone(at("archive", "terminated"))).toBe("quiet");
    // An ended session of an older node shows its own word; a failure
    // stays visible in the archive.
    expect(tone(placement(session("a", failed)))).toBe("danger");
    expect(tone(placement(session("a", exited)))).toBe("quiet");
  });

  it("deals rows into lanes: the waiting first, then the most recent", () => {
    const rows = [
      row(
        session(
          "old",
          running,
          placed("building", "working", "2026-11-07T10:00:00Z"),
        ),
      ),
      row(
        session(
          "new",
          running,
          placed("building", "idle", "2026-11-07T12:00:00Z"),
        ),
      ),
      row(
        session(
          "stuck",
          running,
          placed("building", "blocked", "2026-11-07T08:00:00Z"),
        ),
      ),
      row(session("review", running, placed("needs_review", "commented"))),
      row(
        session(
          "done",
          exited,
          placed("archive", "terminated", "2026-11-07T11:00:00Z"),
        ),
      ),
      row(session("gone", exited)),
    ];
    const dealt = board(rows);
    expect(dealt.lanes.building.map((card) => card.session.id)).toEqual([
      "stuck",
      "new",
      "old",
    ]);
    expect(dealt.lanes.needs_review.map((card) => card.session.id)).toEqual([
      "review",
    ]);
    expect(dealt.lanes.validating).toEqual([]);
    expect(dealt.lanes.ready).toEqual([]);
    // The archive is newest first; an older node's ended session sorts by
    // its start when it reports no end.
    expect(dealt.archive.map((card) => card.session.id)).toEqual([
      "done",
      "gone",
    ]);
  });

  it("writes how long ago, never in weeks", () => {
    const now = Date.parse("2026-11-07T12:00:00Z");
    expect(sinceLabel("2026-11-07T11:59:40Z", now)).toBe("just now");
    expect(sinceLabel("2026-11-07T12:05:00Z", now)).toBe("just now");
    expect(sinceLabel("2026-11-07T11:54:00Z", now)).toBe("6m ago");
    expect(sinceLabel("2026-11-07T09:00:00Z", now)).toBe("3h ago");
    expect(sinceLabel("2026-10-20T12:00:00Z", now)).toBe("18d ago");
    expect(sinceLabel("not a time", now)).toBe("just now");
  });
});
