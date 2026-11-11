import { describe, expect, it } from "vitest";

import type { Creator, SessionInfo, SessionStatus } from "../wire/proto";
import { board, laneLabel, laneOf, LANES, laneTone } from "./board";
import type { Row } from "./tree";

function session(id: string, status: SessionStatus): SessionInfo {
  const created_by: Creator = { kind: "owner" };
  return {
    id,
    spec: { harness: "claude-code", mode: "structured" },
    created_by,
    status,
    created_at: "2026-11-07T10:00:00Z",
    next_seq: 0,
  };
}

const running: SessionStatus = { state: "running" };
const starting: SessionStatus = { state: "starting" };
const exited: SessionStatus = { state: "exited", code: 0 };
const failed: SessionStatus = { state: "failed", message: "no login" };
const interrupted: SessionStatus = { state: "interrupted" };

/** A tree row: `parent` is the row it nests under, if any. */
function row(
  id: string,
  status: SessionStatus,
  parent: Row | null = null,
): Row {
  return {
    node: "laptop",
    session: session(id, status),
    depth: parent === null ? 0 : parent.depth + 1,
    parent: parent === null ? null : parent.session,
    parentNode: parent === null ? null : parent.node,
  };
}

describe("the sessions board", () => {
  it("gives every status one lane, starting with the running ones", () => {
    expect(laneOf(starting)).toBe("running");
    expect(laneOf(running)).toBe("running");
    expect(laneOf(interrupted)).toBe("interrupted");
    expect(laneOf(failed)).toBe("failed");
    expect(laneOf(exited)).toBe("exited");
    expect(laneOf({ state: "exited", code: 3 })).toBe("exited");
  });

  it("names and tones each lane", () => {
    expect(LANES.map(laneLabel)).toEqual([
      "Running",
      "Interrupted",
      "Failed",
      "Exited",
    ]);
    expect(LANES.map(laneTone)).toEqual([
      "running",
      "interrupt",
      "danger",
      "offline",
    ]);
  });

  it("deals rows into lanes in tree order", () => {
    const one = row("one", running);
    const two = row("two", exited);
    const three = row("three", starting);
    const lanes = board([one, two, three]);
    expect(lanes.running.map((card) => card.session.id)).toEqual([
      "one",
      "three",
    ]);
    expect(lanes.exited.map((card) => card.session.id)).toEqual(["two"]);
    expect(lanes.failed).toEqual([]);
    expect(lanes.interrupted).toEqual([]);
  });

  it("indents a child under its parent only inside the same lane", () => {
    const parent = row("parent", running);
    const child = row("child", running, parent);
    const grandchild = row("grandchild", running, child);
    const deeper = row("deeper", running, grandchild);
    const ended = row("ended", exited, parent);
    const lanes = board([parent, child, grandchild, deeper, ended]);
    expect(lanes.running.map((card) => [card.session.id, card.indent])).toEqual(
      [
        ["parent", 0],
        ["child", 1],
        ["grandchild", 2],
        // Deeper than two is not indented further (ADR 0013 caps depth at 3).
        ["deeper", 2],
      ],
    );
    // Its parent is in another lane: not indented, though it is a child.
    expect(lanes.exited.map((card) => [card.session.id, card.indent])).toEqual([
      ["ended", 0],
    ]);
  });

  it("does not indent under a parent that is not on the board", () => {
    const parent = row("parent", running);
    const orphan = row("orphan", running, parent);
    expect(board([orphan]).running[0]?.indent).toBe(0);
  });
});
