// Whether a machine serves the pull request pane's reads (`pr.timeline`
// and `pr.check` come at PULL_REQUEST_PANE_PROTOCOL, ADR 0088; a
// session's summary is older, protocol 11). The node reviewer's P5b R1:
// an older machine is offered no Pull request tab and asked nothing it
// cannot answer.

import { PULL_REQUEST_PANE_PROTOCOL } from "../../../wire/proto";

export function pullPaneServed(protocol: number | null): boolean {
  return protocol !== null && protocol >= PULL_REQUEST_PANE_PROTOCOL;
}
