// The context meter beside the agent's control (agent-orchestrator's
// `ContextMeter`): a 20px ring in a 28px box, its arc how much of the
// context the agent says is used, in the building colour, then the
// needs-you colour from 70% and the error colour from 90%; its figures on
// hover. Nothing before the agent reports a window (data/context-meter.ts).
// At a compact width a press opens them in a sheet as tall as they are
// (Paseo's ContextWindowSheet, context-window-meter.tsx 205): a phone has
// no hover, and the usage cards' Refresh needs a press (TRACKER 787).

import "./context-ring.css";

import { useState } from "react";

import { meterOf } from "../../data/context-meter";
import { COMPACT } from "../../ui/breakpoints";
import { Dialog, HoverCard } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";
import type { ContextUsage, SessionId } from "../../wire/proto";
import { useMedia } from "../shell/use-media";
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
  const compact = useMedia(COMPACT);
  const [sheet, setSheet] = useState(false);
  // Widened while open: the sheet goes, and narrowing again does not
  // bring it back unasked.
  if (sheet && !compact) {
    setSheet(false);
  }
  if (meter === null) {
    return null;
  }
  const ring = (
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
  );
  const label = `Context ${String(meter.percent)}% used`;
  if (compact) {
    return (
      <>
        <button
          type="button"
          className="context-ring"
          data-fullness={meter.fullness}
          aria-label={label}
          aria-haspopup="dialog"
          onClick={() => {
            setSheet(true);
          }}
        >
          {ring}
        </button>
        {sheet ? (
          // Paseo's sheet: its header names it, so the figures start at
          // the share used (`showTitle={false}`).
          <Dialog
            title={meter.title}
            sheet="content"
            onClose={() => {
              setSheet(false);
            }}
          >
            <div className="context-details">
              <p className="context-details-used">{meter.used}</p>
              {meter.details.map((line) => (
                <p key={line} className="context-details-line">
                  {line}
                </p>
              ))}
              <SessionUsage node={node} session={id} />
            </div>
          </Dialog>
        ) : null}
      </>
    );
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
          aria-label={label}
        >
          {ring}
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
