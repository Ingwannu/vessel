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

  test("an agent that cannot steer is never offered it, and queues", () => {
    const plain = { ...STEERS, steer: false };
    expect(
      sendPlan({ ...RUNNING, preferred: "steer", abilities: plain }),
    ).toEqual({
      kind: "deliver",
      delivery: "queue",
      alternate: "interrupt",
      fallback: "cannot-steer",
    });
    // Not yet said counts as cannot.
    expect(canSteer({ ...RUNNING, abilities: null })).toBe(false);
    // Refused by the node once: not offered again in this session.
    expect(
      sendPlan({ ...RUNNING, preferred: "steer", steerRefused: true }),
    ).toMatchObject({ delivery: "queue", fallback: "cannot-steer" });
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
  test("the preferred delivery, or the queue where the agent cannot steer", () => {
    const empty = { ...RUNNING, filled: false };
    expect(nextDelivery({ ...empty, preferred: "steer" })).toBe("steer");
    expect(nextDelivery({ ...empty, preferred: "queue" })).toBe("queue");
    expect(
      nextDelivery({ ...empty, preferred: "steer", steerRefused: true }),
    ).toBe("queue");
  });

  test("nothing between turns or before protocol 10", () => {
    expect(nextDelivery({ ...RUNNING, inTurn: false })).toBeNull();
    expect(nextDelivery({ ...RUNNING, protocol: 9 })).toBeNull();
  });
});
