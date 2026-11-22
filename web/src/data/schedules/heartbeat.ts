// A cadence edit carries the opened revision and preserves the full definition.
import type { NodeStatus } from "../../wire/bridge";
import {
  type Cadence,
  type Creator,
  HEARTBEAT_CADENCE_PROTOCOL,
  type ScheduleInfo,
  type SchedulePreview,
  type UpdateScheduleCadence,
} from "../../wire/proto";
import { isHeartbeat } from "./rows";

export type Cron = Extract<Cadence, { kind: "cron" }>;
const REVISION_LIMIT = 2 ** 32;

export function supportsHeartbeatEdit(
  connection: NodeStatus | undefined,
): boolean {
  return (
    connection?.state === "connected" &&
    connection.protocol >= HEARTBEAT_CADENCE_PROTOCOL
  );
}

export function editableHeartbeat(info: ScheduleInfo): boolean {
  return (
    isHeartbeat(info) &&
    info.definition.cadence.kind !== "unknown" &&
    info.definition.catch_up !== "unknown" &&
    info.state.state !== "unknown" &&
    !(info.state.state === "ended" && info.state.reason === "unknown")
  );
}

export function sameCron(cadence: Cadence, cron: Cron): boolean {
  return (
    cadence.kind === "cron" &&
    cadence.expr === cron.expr &&
    cadence.zone === cron.zone
  );
}

export function heartbeatRequest(
  info: ScheduleInfo,
  draft: Cron,
  preview: SchedulePreview,
): (UpdateScheduleCadence & { cadence: Cron }) | null {
  const cron = { ...draft, expr: draft.expr.trim().replace(/\s+/gu, " ") };
  return editableHeartbeat(info) &&
    preview.runs.length === 1 &&
    sameCron(preview.cadence, cron) &&
    !sameCron(info.definition.cadence, cron)
    ? { schedule: info.id, revision: info.revision, cadence: cron }
    : null;
}

function sameOwner(before: Creator, after: Creator): boolean {
  switch (before.kind) {
    case "owner":
      return after.kind === "owner";
    case "device":
      return after.kind === "device" && before.id === after.id;
    case "agent":
      return (
        after.kind === "agent" &&
        before.node === after.node &&
        before.session === after.session
      );
  }
}

export function confirmsHeartbeat(
  opened: ScheduleInfo,
  request: UpdateScheduleCadence & { cadence: Cron },
  received: ScheduleInfo,
): boolean {
  const before = opened.definition;
  const after = received.definition;
  return (
    received.id === request.schedule &&
    received.revision === (request.revision + 1) % REVISION_LIMIT &&
    sameOwner(opened.defined_by, received.defined_by) &&
    before.target.kind === "session" &&
    after.target.kind === "session" &&
    after.target.session === before.target.session &&
    sameCron(after.cadence, request.cadence) &&
    (after.name ?? null) === (before.name ?? null) &&
    after.prompt === before.prompt &&
    after.catch_up === before.catch_up &&
    (after.max_runs ?? null) === (before.max_runs ?? null) &&
    (after.ends_at ?? null) === (before.ends_at ?? null)
  );
}

export class HeartbeatSaveUnconfirmed extends Error {
  constructor() {
    super(
      "The heartbeat change could not be confirmed. Reopen it to check the saved cadence.",
    );
  }
}
