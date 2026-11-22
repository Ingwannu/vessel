// List snapshots stay separate from pending mutations and capability reads.
import type { NodeStatus } from "../../wire/bridge";
import type { ScheduleId, ScheduleInfo } from "../../wire/proto";

export type ScheduleRead =
  | { kind: "loading" }
  | { kind: "offline" }
  | { kind: "failed"; reason: string }
  | { kind: "ready"; schedules: readonly ScheduleInfo[] };

export function supportsSchedules(connection: NodeStatus | undefined): boolean {
  return connection?.state === "connected" && connection.protocol >= 11;
}

export function initialRead(connection: NodeStatus | undefined): ScheduleRead {
  switch (connection?.state) {
    case "failed":
    case "reconnecting":
      // A host that could not be asked is distinct from one that refused
      // the request. The pane keeps the actual errors from connected hosts.
      return { kind: "offline" };
    case "connected":
      return supportsSchedules(connection)
        ? { kind: "loading" }
        : {
            kind: "failed",
            reason: "Schedules are unavailable on this machine",
          };
    case undefined:
    case "idle":
    case "connecting":
      return { kind: "loading" };
  }
}

export function actionReply(
  read: ScheduleRead,
  id: ScheduleId,
  replace: ScheduleInfo | undefined,
  deleted: boolean,
): ScheduleRead {
  if (read.kind !== "ready") return read;
  if (deleted) {
    return {
      kind: "ready",
      schedules: read.schedules.filter((info) => info.id !== id),
    };
  }
  return replace === undefined
    ? read
    : {
        kind: "ready",
        schedules: read.schedules.map((info) =>
          info.id === id ? replace : info,
        ),
      };
}
