import { describe, expect, it, vi } from "vitest";

import { Bridge } from "../../bridge/client";
import { fixtureId } from "../../testing/ids";
import type { NodeStatus } from "../../wire/bridge";
import {
  HEARTBEAT_CADENCE_PROTOCOL,
  type ScheduleInfo,
  type ScheduleList,
} from "../../wire/proto";
import { Fleet } from "../fleet";
import { bookNode, connectedAs } from "../fixtures/machines";
import { aggregate } from "./aggregate";
import { HeartbeatSaveUnconfirmed, heartbeatRequest } from "./heartbeat";
import { MachineSchedules } from "./machine-schedules";
import { isHeartbeat, rowActions, scheduleMeta } from "./rows";

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

function connected(name: string, protocol = 45): NodeStatus {
  const status = connectedAs(name);
  if (status.state !== "connected")
    throw new Error("Expected a connected fixture");
  return { ...status, protocol };
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

const target = fixtureId("SessionId", "heartbeat");
const heartbeat: ScheduleInfo = {
  ...schedule,
  definition: {
    ...schedule.definition,
    target: { kind: "session", session: target },
    cadence: { kind: "every", seconds: 90, anchor: when },
  },
  defined_by: {
    kind: "agent",
    node: fixtureId("NodeId", "node"),
    session: target,
  },
};
const cron = { kind: "cron", expr: "0 9 * * *", zone: "Asia/Seoul" } as const;
const forecast = { after: when, cadence: cron, runs: [when] };

async function heartbeatSetup() {
  const { controller, state, call } = setup();
  const supported = {
    ...state,
    statuses: {
      [laptop.key]: connected("laptop", HEARTBEAT_CADENCE_PROTOCOL),
    },
  };
  call
    .mockResolvedValueOnce({ schedules: [heartbeat] })
    .mockResolvedValueOnce({ scopes: ["operate", "read"] });
  controller.sync(supported);
  await Promise.resolve();
  await Promise.resolve();
  const row = aggregate(controller.store.get()).rows[0];
  if (row === undefined) throw new Error("Heartbeat fixture missing");
  return { controller, state: supported, call, row };
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

  it("puts the machine first when the schedule list spans several hosts", () => {
    const now = Date.parse(when);
    const singleHost = scheduleMeta(schedule, null, now, undefined);
    expect(singleHost[0]).toBe("Every hour");
    expect(scheduleMeta(schedule, "laptop", now, undefined)).toEqual([
      "laptop",
      ...singleHost,
    ]);
  });

  it("labels existing-session targets as heartbeats and offers only delete for either owner", () => {
    const id = fixtureId("SessionId", "target");
    const person: ScheduleInfo = {
      ...schedule,
      definition: {
        ...schedule.definition,
        target: { kind: "session", session: id },
      },
    };
    const agent: ScheduleInfo = {
      ...person,
      defined_by: {
        kind: "agent",
        node: fixtureId("NodeId", "node"),
        session: id,
      },
    };
    for (const info of [person, agent]) {
      expect(isHeartbeat(info)).toBe(true);
      expect(rowActions(info)).toEqual(["delete"]);
    }
  });

  it("keeps old and read-only connection rows out of the editor", async () => {
    const { controller, state, call } = setup();
    call.mockResolvedValueOnce({ schedules: [heartbeat] });
    controller.sync(state);
    await Promise.resolve();
    expect(call).toHaveBeenCalledTimes(1);
    expect(
      aggregate(controller.store.get()).rows[0]?.heartbeatRoute,
    ).toBeUndefined();

    call
      .mockResolvedValueOnce({ schedules: [heartbeat] })
      .mockResolvedValueOnce({ scopes: ["admin", "read"] });
    controller.sync({
      ...state,
      statuses: {
        [laptop.key]: connected("laptop", HEARTBEAT_CADENCE_PROTOCOL),
      },
    });
    await Promise.resolve();
    await Promise.resolve();
    expect(call).toHaveBeenLastCalledWith(laptop.key, "device.self", {});
    expect(
      aggregate(controller.store.get()).rows[0]?.heartbeatRoute,
    ).toBeUndefined();
  });

  it("writes only a previewed changed cadence and does not race another action", async () => {
    const { controller, call, row } = await heartbeatSetup();
    expect(row.heartbeatRoute).toBeDefined();
    const receipt = deferred<ScheduleInfo>();
    call.mockReturnValueOnce(receipt.promise);
    const saving = controller.saveHeartbeat(row, cron, forecast);
    await controller.act(laptop.key, heartbeat.id, "delete");
    expect(call).toHaveBeenCalledTimes(3);
    expect(call).toHaveBeenLastCalledWith(
      laptop.key,
      "schedule.update_cadence",
      {
        schedule: heartbeat.id,
        revision: heartbeat.revision,
        cadence: cron,
      },
    );
    const saved = {
      ...heartbeat,
      revision: 1,
      definition: { ...heartbeat.definition, cadence: cron },
    };
    call.mockResolvedValueOnce({ schedules: [saved] });
    receipt.resolve(saved);
    await saving;
    expect(aggregate(controller.store.get()).rows[0]?.info).toEqual(saved);
    expect(heartbeatRequest(saved, cron, forecast)).toBeNull();
    expect(
      heartbeatRequest(heartbeat, { ...cron, expr: "0 10 * * *" }, forecast),
    ).toBeNull();
  });

  it("treats a changed owner in the reply as unconfirmed and refreshes", async () => {
    const { controller, call, row } = await heartbeatSetup();
    call
      .mockResolvedValueOnce({
        ...heartbeat,
        revision: 1,
        defined_by: { kind: "owner" },
        definition: { ...heartbeat.definition, cadence: cron },
      })
      .mockResolvedValueOnce({ schedules: [heartbeat] });
    await expect(
      controller.saveHeartbeat(row, cron, forecast),
    ).rejects.toBeInstanceOf(HeartbeatSaveUnconfirmed);
    expect(call).toHaveBeenLastCalledWith(laptop.key, "schedule.list", {});
    expect(aggregate(controller.store.get()).rows[0]?.info).toEqual(heartbeat);
  });

  it("keeps an open draft tied to the original connection after reconnect", async () => {
    const { controller, state, call, row } = await heartbeatSetup();
    call
      .mockResolvedValueOnce({ schedules: [heartbeat] })
      .mockResolvedValueOnce({ scopes: ["read", "operate"] });
    controller.sync({
      ...state,
      statuses: { [laptop.key]: { state: "connecting" } },
    });
    controller.sync(state);
    await Promise.resolve();
    await Promise.resolve();
    expect(controller.heartbeatAvailable(row)).toBe(false);
    await expect(controller.saveHeartbeat(row, cron, forecast)).rejects.toThrow(
      "original heartbeat connection",
    );
    expect(call).toHaveBeenLastCalledWith(laptop.key, "device.self", {});
  });
});
