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

/** Other props reach the trigger: a Menu or a Popover can render its
 * trigger as a Tooltip around a button (`trigger={<Tooltip …><IconButton
 * …/></Tooltip>}`), and its handlers, ARIA and ref pass through to the
 * button. */
export function Tooltip({
  label,
  shortcut,
  lines,
  side = "top",
  children,
  ...trigger
}: Omit<Base.Trigger.Props, "render" | "children"> & {
  label: string;
  /** The keys, written as the platform shows them: "⌘K". */
  shortcut?: string;
  /** Quieter lines under the label (the context meter's figures). */
  lines?: readonly string[];
  side?: "top" | "bottom" | "right" | "left";
  /** The element it names: a kit control, or text that may be cut. */
  children: ReactElement;
}) {
  return (
    <Base.Root disableHoverablePopup>
      <Base.Trigger {...trigger} render={children} />
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner tooltip-positioner"
          side={side}
          sideOffset={8}
          collisionPadding={8}
        >
          <Base.Popup
            className={lines === undefined ? "tooltip" : "tooltip lined"}
          >
            {lines === undefined ? (
              <>
                <span>{label}</span>
                {shortcut === undefined ? null : (
                  <span className="tooltip-keys">{shortcut}</span>
                )}
              </>
            ) : (
              <>
                <span className="tooltip-head">
                  <span>{label}</span>
                  {shortcut === undefined ? null : (
                    <span className="tooltip-keys">{shortcut}</span>
                  )}
                </span>
                {lines.map((line) => (
                  <span key={line} className="tooltip-line">
                    {line}
                  </span>
                ))}
              </>
            )}
            <Base.Arrow className="tooltip-arrow" />
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
