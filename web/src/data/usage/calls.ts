// Accounting is read from the retained node ledger, never reconstructed
// from context occupancy or a browser's partial conversation replay.
import type { NodeKey } from "../../wire/bridge";
import type { UsagePeriod, UsageReport } from "../../wire/proto";
import type { Fleet } from "../fleet";
import { attempt, type Outcome } from "../outcome";

/** A rolling machine interval; session-only queries belong to their own view. */
export type UsageRange = Exclude<UsagePeriod, "session">;

/** Only the visible machine is queried; this never reads vendor credentials. */
export function readUsage(
  fleet: Fleet,
  node: NodeKey,
  period: UsageRange,
): Promise<Outcome<UsageReport>> {
  return attempt(() =>
    fleet.bridge.call(node, "usage.query", {
      scope: { kind: "machine" },
      period,
    }),
  );
}
