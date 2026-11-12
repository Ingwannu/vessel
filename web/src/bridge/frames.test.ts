import { describe, expect, expectTypeOf, it } from "vitest";

import type { ClientFrame } from "../wire/bridge";
import type { Call, Reply } from "../wire/proto";
import {
  answers,
  type CallFrame,
  type CallOf,
  callOf,
  isEventRecord,
  isServerFrame,
  parse,
} from "./frames";
import { fixtureId } from "../testing/ids";

describe("frames", () => {
  it("takes the bridge's frames and nothing else", () => {
    expect(isServerFrame({ type: "unreadable", message: "x" })).toBe(true);
    expect(isServerFrame({ type: "welcome" })).toBe(true);
    expect(isServerFrame({ type: "hello" })).toBe(false);
    expect(isServerFrame({ kind: "reply" })).toBe(false);
    expect(isServerFrame(null)).toBe(false);
    expect(isServerFrame("reply")).toBe(false);
  });

  it("parses a message's data into a frame or refuses it", () => {
    expect(parse('{"type":"unreadable","message":"x"}')).toEqual({
      type: "unreadable",
      message: "x",
    });
    expect(() => parse('{"type":"hello"}')).toThrow("Not a frame");
    expect(() => parse(new ArrayBuffer(2))).toThrow("Not a frame");
  });

  it("checks an event frame's record", () => {
    const record = {
      seq: 3,
      at: "2026-11-11T00:00:00Z",
      event: { type: "status" },
    };
    expect(isEventRecord(record)).toBe(true);
    expect(isServerFrame({ type: "event", stream: 1, record })).toBe(true);
    expect(
      isServerFrame({ type: "event", stream: 1, record: { seq: "3" } }),
    ).toBe(false);
    expect(isEventRecord({ seq: 3, at: "x", event: null })).toBe(false);
  });

  it("builds a call from its method and parameters", () => {
    expect(
      callOf("session.kill", { session: fixtureId("SessionId", "s") }),
    ).toEqual({
      method: "session.kill",
      params: { session: "s" },
    });
    // For a known method the call is exactly that member of `Call`, and
    // its frame is exactly the generated call frame.
    expectTypeOf<CallOf<"session.create">>().toEqualTypeOf<
      Extract<Call, { method: "session.create" }>
    >();
    expectTypeOf<CallFrame<"node.info">>().toExtend<
      Extract<ClientFrame, { type: "call" }>
    >();
  });

  it("narrows a reply by its method", () => {
    const reply: Reply = { method: "session.kill", result: null };
    expect(answers(reply, "session.kill")).toBe(true);
    expect(answers(reply, "session.get")).toBe(false);
  });
});
