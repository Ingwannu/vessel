// Tooltip (DESIGN.md §6.13): the name of an icon-only control, or the
// whole of a value cut short. Never a paragraph, never the only place a
// fact lives. Replaces the browser's own `title` tooltip, which this page
// does not use (scripts/check-controls.mjs).
//
// agent-orchestrator's: 12px on the popup surface, radius 8, 6px by 12px,
// its shadow, an arrow in the same fill, after a 400ms rest, not hoverable,
// with the shortcut in muted keys at the right. Keyboard focus shows it
// too (Base UI).

import "./tooltip.css";

import { Tooltip as Base } from "@base-ui/react/tooltip";
import type { ReactElement, ReactNode } from "react";

/** One per page, around the app: tooltips share the 400ms delay, and the
 * next one opens at once while one is showing. */
export function TooltipProvider({ children }: { children: ReactNode }) {
  return <Base.Provider delay={400}>{children}</Base.Provider>;
}

export function Tooltip({
  label,
  shortcut,
  side = "top",
  children,
}: {
  label: string;
  /** The keys, written as the platform shows them: "⌘K". */
  shortcut?: string;
  side?: "top" | "bottom" | "right" | "left";
  /** The element it names: a kit control, or text that may be cut. */
  children: ReactElement;
}) {
  return (
    <Base.Root disableHoverablePopup>
      <Base.Trigger render={children} />
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          side={side}
          sideOffset={8}
          collisionPadding={8}
        >
          <Base.Popup className="tooltip">
            <span>{label}</span>
            {shortcut === undefined ? null : (
              <span className="tooltip-keys">{shortcut}</span>
            )}
            <Base.Arrow className="tooltip-arrow" />
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
