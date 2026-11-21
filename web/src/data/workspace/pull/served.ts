// Whether a machine serves the pull request pane's reads (`pr.timeline`
// and `pr.check` come at PULL_REQUEST_PANE_PROTOCOL, ADR 0088; a
// session's summary is older, protocol 11). The node reviewer's P5b R1:
// an older machine is offered no Pull request tab and asked nothing it
// cannot answer. A machine not connected has no protocol yet: a tab kept
// from before says it is offline, not that it is too old (the node
// reviewer's P5b note; TRACKER 833).

import { PULL_REQUEST_PANE_PROTOCOL } from "../../../wire/proto";

/** What the pane can show: the reads, or why not. */
export type PullReach = "served" | "old" | "offline";

export function pullReach(protocol: number | null): PullReach {
  if (protocol === null) return "offline";
  return protocol >= PULL_REQUEST_PANE_PROTOCOL ? "served" : "old";
}

export function pullPaneServed(protocol: number | null): boolean {
  return pullReach(protocol) === "served";
}
