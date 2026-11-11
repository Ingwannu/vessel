// The reducer against records a real node produced: a structured turn of
// `fake-acp` with its permission answered, recorded through the bridge by
// vessel-web's test `a_structured_turn_with_a_permission_reaches_the_page`
// (rerun it with VESSEL_WRITE_FIXTURES=1 to record them again).

import { describe, expect, it } from "vitest";

import { isEventRecord } from "../bridge/frames";
import recorded from "./fixtures/fake-acp-turn.json";
import { reduce } from "./timeline";
import { EMPTY } from "./timeline-model";

// JSON imports are typed loosely (strings, not the wire's literal unions):
// the records are checked by the predicate the socket checks them with.
const fixture: readonly unknown[] = recorded;
const records = fixture.filter(isEventRecord);

describe("the timeline over recorded records", () => {
  it("reads every recorded record as one", () => {
    expect(records).toHaveLength(fixture.length);
  });

  it("folds a whole turn", () => {
    const timeline = reduce(EMPTY, records);
    expect(timeline.entries.map((entry) => entry.kind)).toEqual([
      "status",
      "log",
      "ready",
      "status",
      "prompt",
      "thought",
      "message",
      "tool",
      "plan",
      "permission",
      "turn_ended",
    ]);
    const tool = timeline.entries.find((entry) => entry.kind === "tool");
    expect(tool?.kind === "tool" && tool.tool.status).toBe("completed");
    const permission = timeline.entries.find(
      (entry) => entry.kind === "permission",
    );
    expect(permission?.kind === "permission" && permission.outcome).toEqual({
      outcome: "selected",
      option: "allow-once",
    });
    expect(timeline.pending).toBeNull();
    expect(timeline.inTurn).toBe(false);
    expect(timeline.turns).toBe(1);
    expect(timeline.next).toBe(records.length);
  });

  it("gives the same timeline however the records are batched", () => {
    const whole = reduce(EMPTY, records);
    let piecewise = EMPTY;
    for (const record of records) {
      piecewise = reduce(piecewise, [record]);
    }
    expect(piecewise.entries).toEqual(whole.entries);
    expect(piecewise.plan).toEqual(whole.plan);
  });
});
