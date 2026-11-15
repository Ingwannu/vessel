// Approval projection for a block whose individual choices may be absent.
// Require every key without inventing a nullable or sentinel choice value.

import type { ServicePorts } from "../../wire/proto";

type CompleteAllocation = {
  [K in keyof Required<ServicePorts>]: ServicePorts[K];
};

/** Retain block presence and require a decision for every nested wire field. */
export function allocationForDisplay(
  ports: ServicePorts | null,
): CompleteAllocation | null {
  if (ports === null) return null;
  return { range: ports.range, port_script: ports.port_script };
}
