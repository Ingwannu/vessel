// Hover card: a popover that opens on hover or focus of its trigger and may
// hold controls (Paseo's desktop context window hover card, whose usage
// cards have a Refresh button). A Tooltip holds words only; this holds a
// card. Same popup surface and widths as Popover.
import "./popup.css";
import "./popover.css";

import { PreviewCard as Base } from "@base-ui/react/preview-card";
import type { ReactElement, ReactNode } from "react";

import type { PopoverWidth } from "./popover";

export function HoverCard({
  trigger,
  children,
  width = "lg",
  side = "top",
  align = "end",
}: {
  /** The element it describes; Base UI adds its handlers and ARIA. */
  trigger: ReactElement;
  children: ReactNode;
  width?: PopoverWidth;
  side?: "bottom" | "top" | "right" | "left";
  align?: "start" | "center" | "end";
}) {
  return (
    <Base.Root>
      {/* The tooltip's 400 ms, so a pointer passing over does not open it. */}
      <Base.Trigger render={trigger} delay={400} closeDelay={150} />
      <Base.Portal>
        <Base.Positioner
          className="popup-positioner"
          side={side}
          align={align}
          sideOffset={6}
          collisionPadding={8}
        >
          <Base.Popup className={`popup popover ${width}`}>
            {children}
          </Base.Popup>
        </Base.Positioner>
      </Base.Portal>
    </Base.Root>
  );
}
