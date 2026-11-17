// Popover (DESIGN.md §6.13): non-modal content anchored to a trigger, for
// a short form or facts that do not need a dialog. agent-orchestrator's:
// the popup surface (radius 10, its edge and shadow) with the popup
// motion; GOAT's widths and padding, since the reference sets none.

import "./popup.css";
import "./popover.css";

import { Popover as Base } from "@base-ui/react/popover";
import type { ReactElement, ReactNode } from "react";

/** GOAT's: 256, 320 and 384px; `settings` is Paseo's 340px settings
 * popover (usage/options.tsx, DropdownMenuContent width 340). */
export type PopoverWidth = "sm" | "md" | "settings" | "lg";

export function Popover({
  trigger,
  title,
  children,
  width = "md",
  side = "bottom",
  align = "center",
  showTitle = true,
}: {
  /** A kit Button or IconButton; Base UI adds its handlers and ARIA. */
  trigger: ReactElement;
  /** Names the popup for a screen reader, and heads it. */
  title: string;
  children: ReactNode;
  width?: PopoverWidth;
  side?: "bottom" | "top" | "right" | "left";
  align?: "start" | "center" | "end";
  /** Off where the reference shows no heading (Paseo's settings popovers
   * on wide screens): the title still names the popup. */
  showTitle?: boolean;
}) {
  return (
    <Base.Root>
      <Base.Trigger render={trigger} />
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          side={side}
          align={align}
          sideOffset={6}
          collisionPadding={8}
        >
          <Base.Popup className={`popup popover ${width}`}>
            <Base.Title
              className={showTitle ? "popover-title" : "popover-title unseen"}
            >
              {title}
            </Base.Title>
            {children}
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
