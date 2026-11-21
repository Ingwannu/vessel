import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../../bridge/client";
import { fixtureId } from "../../testing/ids";
import type { NodeStatus } from "../../wire/bridge";
import type { ScheduleInfo, ScheduleList } from "../../wire/proto";
import { Fleet } from "../fleet";
import { bookNode, connectedAs } from "../fixtures/machines";
import { aggregate } from "./aggregate";
import { MachineSchedules } from "./machine-schedules";
import { rowActions } from "./rows";

const laptop = bookNode("laptop");
const studio = bookNode("studio");
const when = "2026-11-21T00:00:00Z";
const schedule: ScheduleInfo = {
  id: fixtureId("ScheduleId", "fixture"),
  definition: {
    name: "A fixture",
    cadence: { kind: "cron", expr: "0 * * * *", zone: "UTC" },
    target: {
      kind: "new_session",
      after: "archive",
      spec: { harness: "claude-code", mode: "structured" },
    },
    prompt: "A test prompt",
    catch_up: "skip",
  },
  defined_by: { kind: "owner" },
  state: { state: "active", next_due: when },
  runs: 0,
  created_at: when,
  updated_at: when,
  revision: 0,
};

function connected(name: string): NodeStatus {
  const status = connectedAs(name);
  if (status.state !== "connected")
    throw new Error("Expected a connected fixture");
  return { ...status, protocol: 45 };
}

function deferred<T>() {
  let complete: (value: T) => void = () => {
    throw new Error("Deferred result not initialized");
  };
  let fail: (error: Error) => void = () => {
    throw new Error("Deferred result not initialized");
  };
  const promise = new Promise<T>((resolve, reject) => {
    complete = resolve;
    fail = reject;
  });
  return {
    promise,
    resolve: (value: T) => {
      complete(value);
    },
    reject: (error: Error) => {
      fail(error);
    },
  };
}

function setup() {
  const bridge = new Bridge(
    { kind: "bootstrap", key: "a".repeat(64) },
    "ws://127.0.0.1:1/api/ws",
  );
  const fleet = new Fleet(bridge);
  const controller = new MachineSchedules(bridge);
  const state = {
    ...fleet.store.get(),
    link: { kind: "open", version: "0.1.0" } as const,
    book: [laptop],
    statuses: { [laptop.key]: connected("laptop") },
  };
  return { controller, state, call: vi.spyOn(bridge, "call") };
}

describe("machine schedules", () => {
  it("renders the successful machine while another is still waiting or fails", async () => {
    const { controller, state, call } = setup();
    const waiting = deferred<ScheduleList>();
    call
      .mockResolvedValueOnce({ schedules: [schedule] })
      .mockReturnValueOnce(waiting.promise);
    controller.sync({
      ...state,
      book: [laptop, studio],
      statuses: {
        ...state.statuses,
        [studio.key]: connected("studio"),
      },
    });
    expect(aggregate(controller.store.get()).phase).toBe("loading");
    await Promise.resolve();
    expect(aggregate(controller.store.get())).toMatchObject({
      phase: "ready",
      rows: [{ node: laptop.key, info: schedule }],
    });
    waiting.reject(new Error("The fixture refused"));
    await Promise.resolve();
    expect(aggregate(controller.store.get()).errors).toEqual([
      { node: studio.key, reason: "Could not load schedules" },
    ]);
  });

  it("skips offline machines and resolves a settled offline fleet as empty", () => {
    const { controller, state, call } = setup();
    controller.sync({
      ...state,
      statuses: {
        [laptop.key]: {
          state: "reconnecting",
          message: "Offline",
          attempt: 1,
          retry_in_ms: 1000,
        },
      },
    });
    expect(call).not.toHaveBeenCalled();
    expect(aggregate(controller.store.get())).toEqual({
      phase: "ready",
      rows: [],
      errors: [],
    });
  });

  it("ignores an old connection's reply once a new connection has answered", async () => {
    const { controller, state, call } = setup();
    const old = deferred<ScheduleList>();
    call
      .mockReturnValueOnce(old.promise)
      .mockResolvedValueOnce({ schedules: [] });
    controller.sync(state);
    controller.sync({
      ...state,
      statuses: { [laptop.key]: { state: "connecting" } },
    });
    controller.sync(state);
    await Promise.resolve();
    old.resolve({ schedules: [schedule] });
    await Promise.resolve();
    expect(aggregate(controller.store.get())).toEqual({
      phase: "ready",
      rows: [],
      errors: [],
    });
  });

  it("removes only the refused deletion's overlay and restores the row", async () => {
    const { controller, state, call } = setup();
    const refusal = deferred<never>();
    call
      .mockResolvedValueOnce({ schedules: [schedule] })
      .mockReturnValueOnce(refusal.promise)
      .mockResolvedValueOnce({ schedules: [schedule] });
    controller.sync(state);
    await Promise.resolve();
    const action = controller.act(laptop.key, schedule.id, "delete");
    expect(aggregate(controller.store.get()).rows).toEqual([]);
    refusal.reject(new Error("Not allowed"));
    await expect(action).rejects.toThrow("Not allowed");
    expect(aggregate(controller.store.get()).rows).toEqual([
      { node: laptop.key, info: schedule },
    ]);
    expect(call).toHaveBeenNthCalledWith(2, laptop.key, "schedule.delete", {
      schedule: schedule.id,
    });
  });

  it("shows only delete for an agent-owned schedule targeting itself", () => {
    const id = fixtureId("SessionId", "target");
    expect(
      rowActions({
        ...schedule,
        defined_by: {
          kind: "agent",
          node: fixtureId("NodeId", "node"),
          session: id,
        },
        definition: {
          ...schedule.definition,
          target: { kind: "session", session: id },
        },
      }),
    ).toEqual(["delete"]);
  });
});
