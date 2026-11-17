import { describe, expect, test } from "vitest";

import {
  canSteer,
  hasConversation,
  nextDelivery,
  sendNowDelivery,
  sendPlan,
  type SendState,
} from "./delivery";

const STEERS = {
  steer: true,
  resume: true,
  image: true,
  audio: false,
  fork: false,
};

const RUNNING: SendState = {
  protocol: 10,
  live: true,
  inTurn: true,
  filled: true,
  abilities: STEERS,
  queue: { entries: [] },
  preferred: "queue",
  steerRefused: false,
  permissionWaits: false,
};

describe("the composer's button", () => {
  test("an ended session does nothing", () => {
    expect(sendPlan({ ...RUNNING, live: false })).toEqual({ kind: "idle" });
  });

  test("an empty box stops a turn, and does nothing between turns", () => {
    expect(sendPlan({ ...RUNNING, filled: false })).toEqual({ kind: "stop" });
    expect(sendPlan({ ...RUNNING, filled: false, inTurn: false })).toEqual({
      kind: "idle",
    });
  });

  test("a node before protocol 10 keeps Send and Stop", () => {
    expect(sendPlan({ ...RUNNING, protocol: 9 })).toEqual({ kind: "stop" });
    expect(sendPlan({ ...RUNNING, protocol: 9, inTurn: false })).toEqual({
      kind: "send",
    });
    expect(hasConversation(null)).toBe(false);
  });

  test("between turns it sends, behind what waits unless that is held", () => {
    expect(sendPlan({ ...RUNNING, inTurn: false })).toEqual({
      kind: "deliver",
      delivery: "queue",
      alternate: null,
      fallback: null,
    });
    const held = sendPlan({
      ...RUNNING,
      inTurn: false,
      queue: { entries: [], held: "cancelled" },
    });
    expect(held).toMatchObject({ kind: "deliver", delivery: "start" });
  });

  test("during a turn the person's choice, the other way a modifier away", () => {
    expect(sendPlan({ ...RUNNING, preferred: "steer" })).toEqual({
      kind: "deliver",
      delivery: "steer",
      alternate: "queue",
      fallback: null,
    });
    expect(sendPlan({ ...RUNNING, preferred: "interrupt" })).toMatchObject({
      delivery: "interrupt",
      alternate: "queue",
    });
    expect(sendPlan(RUNNING)).toMatchObject({
      delivery: "queue",
      alternate: "steer",
    });
  });

  test("a Steer the agent cannot take replaces the turn (Paseo's steerOrReplaceActiveTurn)", () => {
    const plain = { ...STEERS, steer: false };
    expect(
      sendPlan({ ...RUNNING, preferred: "steer", abilities: plain }),
    ).toEqual({
      kind: "deliver",
      delivery: "interrupt",
      alternate: "queue",
      fallback: "cannot-steer",
    });
    // Not yet said counts as cannot.
    expect(canSteer({ ...RUNNING, abilities: null })).toBe(false);
    // Refused by the node once: not offered again in this session.
    expect(
      sendPlan({ ...RUNNING, preferred: "steer", steerRefused: true }),
    ).toMatchObject({ delivery: "interrupt", fallback: "cannot-steer" });
    // Where it can, a Queue's other way is still the steer.
    expect(sendPlan({ ...RUNNING, abilities: plain })).toMatchObject({
      delivery: "queue",
      alternate: "interrupt",
      fallback: null,
    });
  });

  test("a Queue while a permission waits interrupts (Paseo's resolveActiveSendBehavior)", () => {
    const waiting = { ...RUNNING, permissionWaits: true };
    expect(sendPlan(waiting)).toEqual({
      kind: "deliver",
      delivery: "interrupt",
      alternate: "queue",
      fallback: "permission-waits",
    });
    // Steer and Interrupt go as chosen: the turn takes the steer.
    expect(sendPlan({ ...waiting, preferred: "steer" })).toMatchObject({
      delivery: "steer",
      fallback: null,
    });
    expect(sendPlan({ ...waiting, preferred: "interrupt" })).toMatchObject({
      delivery: "interrupt",
      fallback: null,
    });
    // Between turns nothing waits on the turn: it sends as ever.
    expect(sendPlan({ ...waiting, inTurn: false })).toMatchObject({
      delivery: "queue",
      fallback: null,
    });
  });
});

test("a queued prompt's Send now: start, steer where it can, else interrupt", () => {
  expect(sendNowDelivery({ ...RUNNING, inTurn: false })).toBe("start");
  expect(sendNowDelivery(RUNNING)).toBe("steer");
  expect(
    sendNowDelivery({ ...RUNNING, abilities: { ...STEERS, steer: false } }),
  ).toBe("interrupt");
});

describe("what the empty box says the next message will do", () => {
  test("the delivery a message would take, by the same rules", () => {
    const empty = { ...RUNNING, filled: false };
    expect(nextDelivery({ ...empty, preferred: "steer" })).toBe("steer");
    expect(nextDelivery({ ...empty, preferred: "queue" })).toBe("queue");
    expect(
      nextDelivery({ ...empty, preferred: "steer", steerRefused: true }),
    ).toBe("interrupt");
    expect(nextDelivery({ ...empty, permissionWaits: true })).toBe("interrupt");
  });

  test("nothing between turns or before protocol 10", () => {
    expect(nextDelivery({ ...RUNNING, inTurn: false })).toBeNull();
    expect(nextDelivery({ ...RUNNING, protocol: 9 })).toBeNull();
  });
});
