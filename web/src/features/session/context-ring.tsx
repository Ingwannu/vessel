// The context meter beside the agent's control (agent-orchestrator's
// `ContextMeter`): a 20px ring in a 28px box, its arc how much of the
// context the agent says is used, in the building colour, then the
// needs-you colour from 70% and the error colour from 90%; its figures on
// hover. Nothing before the agent reports a window (data/context-meter.ts).

import "./context-ring.css";

import { meterOf } from "../../data/context-meter";
import { Tooltip } from "../../ui/kit";
import type { ContextUsage } from "../../wire/proto";

/** The ring's radius and stroke on its 24-unit box (the reference's). */
const RADIUS = 9;
const AROUND = 2 * Math.PI * RADIUS;

export function ContextRing({ usage }: { usage: ContextUsage | null }) {
  const meter = meterOf(usage);
  if (meter === null) {
    return null;
  }
  const [heading = "", ...lines] = meter.lines;
  return (
    <Tooltip label={heading} lines={lines}>
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
    </Tooltip>
  );
}
