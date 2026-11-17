// The context meter beside the agent's control (agent-orchestrator's
// `ContextMeter`): a 20px ring in a 28px box, its arc how much of the
// context the agent says is used, in the building colour, then the
// needs-you colour from 70% and the error colour from 90%; its figures on
// hover. Nothing before the agent reports a window (data/context-meter.ts).

import "./context-ring.css";

import { meterOf } from "../../data/context-meter";
import { HoverCard } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { ContextUsage, SessionId } from "../../wire/proto";
import { SessionUsage } from "../usage/session-usage";

/** The ring's radius and stroke on its 24-unit box (the reference's). */
const RADIUS = 9;
const AROUND = 2 * Math.PI * RADIUS;

export function ContextRing({
  usage,
  node,
  id,
}: {
  usage: ContextUsage | null;
  /** The machine and the session the ring describes. */
  node: NodeKey;
  id: SessionId;
}) {
  const meter = meterOf(usage);
  if (meter === null) {
    return null;
  }
  return (
    // Paseo's context window hover card: the figures, then the session's
    // plan usage (its cards may be refreshed, so not a tooltip).
    <HoverCard
      trigger={
        <span
          className="context-ring"
          data-fullness={meter.fullness}
          role="img"
          tabIndex={0}
          aria-label={`Context ${String(meter.percent)}% used`}
        >
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
            <circle className="context-ring-track" cx="12" cy="12" r={RADIUS} />
            <circle
              className="context-ring-arc"
              cx="12"
              cy="12"
              r={RADIUS}
              strokeDasharray={`${String(meter.arc * AROUND)} ${String(AROUND)}`}
            />
          </svg>
        </span>
      }
    >
      <p className="context-details-title">{meter.title}</p>
      <p className="context-details-used">{meter.used}</p>
      {meter.details.map((line) => (
        <p key={line} className="context-details-line">
          {line}
        </p>
      ))}
      <SessionUsage node={node} session={id} />
    </HoverCard>
  );
}
