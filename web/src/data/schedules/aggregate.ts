// One row carries its host all the way to an action. Ids are local to a
// node, and a late-updating machine must leave the others available.
import type { NodeKey } from "../../wire/bridge";
import type { ScheduleReads } from "./machine-schedules";
import { bucket, type ScheduleBucket, type ScheduleRow } from "./rows";

export interface ScheduleListView {
  phase: "loading" | "failed" | "ready";
  rows: readonly ScheduleRow[];
  errors: readonly { node: NodeKey; reason: string }[];
}

export function aggregate(state: ScheduleReads): ScheduleListView {
  const rows: ScheduleRow[] = [];
  const errors: { node: NodeKey; reason: string }[] = [];
  let waiting = state.book === null && state.bookError === null;
  let successes = 0;
  for (const [node, machine] of state.machines) {
    switch (machine.read.kind) {
      case "failed":
        errors.push({ node, reason: machine.read.reason });
        break;
      case "loading":
        waiting = true;
        break;
      case "ready":
        successes += 1;
        for (const info of machine.read.schedules) {
          const pending = machine.pending.get(info.id);
          if (pending === "delete") continue;
          rows.push(
            pending === undefined ? { node, info } : { node, info, pending },
          );
        }
        break;
    }
  }
  rows.sort(
    (a, b) =>
      Date.parse(b.info.created_at) - Date.parse(a.info.created_at) ||
      a.node.localeCompare(b.node) ||
      a.info.id.localeCompare(b.info.id),
  );
  const failed =
    state.bookError !== null || (errors.length > 0 && successes === 0);
  const phase =
    rows.length === 0 && waiting ? "loading" : failed ? "failed" : "ready";
  return { phase, rows, errors };
}

export function visibleRows(
  rows: readonly ScheduleRow[],
  host: NodeKey | null,
  status: ScheduleBucket,
): readonly ScheduleRow[] {
  return rows.filter(
    (row) =>
      (host === null || row.node === host) && bucket(row.info.state) === status,
  );
}
