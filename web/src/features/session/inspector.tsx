// Summary remains session chrome. Code views have their own workspace tabs.
import "./inspector.css";

import type { Timeline } from "../../data/timeline-model";
import type { NodeKey } from "../../wire/bridge";
import type { SessionInfo } from "../../wire/proto";
import { InspectorSummary } from "./inspector-summary";

export function Inspector({
  node,
  label,
  session,
  timeline,
}: {
  node: NodeKey;
  label: string;
  session: SessionInfo;
  timeline: Timeline;
}) {
  return (
    <aside className="inspector" aria-label="Session panel">
      <InspectorSummary
        node={node}
        label={label}
        session={session}
        timeline={timeline}
      />
    </aside>
  );
}
