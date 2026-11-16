// Sheet (DESIGN.md §6.13): a panel that slides in from the window's left
// edge over a scrim, for what has no room on a phone (GOAT's mobile
// sidebar: a sheet as wide as the window less 12px, opened from a button
// in the title bar). Base UI's Dialog underneath, as the kit's Dialog:
// focus trapped, the page behind inert, Escape and the scrim close it,
// focus goes back to the button.

import "./dialog.css";
import "./sheet.css";

import { Dialog as Base } from "@base-ui/react/dialog";
import type { ReactNode } from "react";

export function Sheet({
  label,
  onClose,
  side = "left",
  children,
}: {
  /** The sheet's name for assistive technology. */
  label: string;
  onClose: () => void;
  /** `right`: a side panel's sheet (a session's code panel), as wide as
   * the window less 12px, at most 560px, from the right edge. */
  side?: "left" | "right";
  children: ReactNode;
}) {
  return (
    <Base.Root
      open
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <Base.Portal>
        <Base.Backdrop className="dialog-backdrop sheet-backdrop" />
        <Base.Popup
          className={side === "right" ? "sheet right" : "sheet"}
          aria-label={label}
        >
          {children}
        </Base.Popup>
      </Base.Portal>
    </Base.Root>
  );
}
