// Dialog (DESIGN.md §6.9): a modal decision over the current page.
//
// [Decision Log]
// - Purpose: one dialog for every screen and for the command palette, so
//   a scrim, a focus trap and Escape behave the same everywhere.
// - Before: ui/dialog.tsx, a hand-written overlay that moved focus to the
//   first field and listened for Escape on the window. It did not trap
//   Tab, did not make the page behind it inert, and a second dialog's
//   Escape reached the first.
// - Choice: Base UI's Dialog (focus trap, inert background, scroll lock,
//   Escape and an outside click to close, focus back to where it was).
//   A screen mounts <Dialog> while it is open and unmounts it to close,
//   as before; the cost is that a closing dialog does not play its 70ms
//   exit, which the reference's barely shows.
// - Values: agent-orchestrator's base dialog: radius 10, the popup surface
//   and shadow, 16px of padding, a 15px semibold title, the close button
//   at the top right, the footer's actions at the right; its scrim (the
//   ink at 45%, dark: the page at 85%) and its entry (opacity and a scale
//   from .95 over 120ms; the scrim 80ms).
// - Focus on open goes to the first field of the body, else to the
//   footer's last action (the primary one), never to the close button.

import "./dialog.css";

import { Dialog as Base } from "@base-ui/react/dialog";
import { type ReactNode, useRef } from "react";

import { IconButton } from "./button";

/** `md` is the reference's 512px; `lg` holds a grid of cards. */
export type DialogSize = "md" | "lg";

/** What takes focus when `popup` opens. */
function firstFocus(popup: HTMLElement | null): HTMLElement | null {
  if (popup === null) {
    return null;
  }
  const field = popup.querySelector<HTMLElement>(
    ".dialog-b input:not([disabled]), .dialog-b textarea:not([disabled]), .dialog-b [role='combobox']:not([data-disabled]), .dialog-b button:not([disabled])",
  );
  if (field !== null) {
    return field;
  }
  const actions = popup.querySelectorAll<HTMLElement>(
    ".dialog-f button:not([disabled])",
  );
  return actions.item(actions.length - 1);
}

export function Dialog({
  title,
  onClose,
  children,
  footer,
  size = "md",
  actions,
}: {
  title: string;
  /** Escape, the scrim and the close button all call it. */
  onClose: () => void;
  children: ReactNode;
  /** Actions, right-aligned: a ghost Cancel, then one primary. */
  footer?: ReactNode;
  size?: DialogSize;
  /** Controls in the title row, before Close (Paseo's modal header
   * actions: a host filter, Refresh all, a settings cog). */
  actions?: ReactNode;
}) {
  const popup = useRef<HTMLDivElement>(null);
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
        <Base.Backdrop className="dialog-backdrop" />
        <Base.Viewport className="dialog-viewport">
          <Base.Popup
            ref={popup}
            className={size === "lg" ? "dialog lg" : "dialog"}
            initialFocus={() => firstFocus(popup.current)}
          >
            <div className="dialog-h">
              <Base.Title className="dialog-title">{title}</Base.Title>
              {actions === undefined ? null : (
                <div className="dialog-actions">{actions}</div>
              )}
              <Base.Close
                render={<IconButton label="Close" icon="close" size="sm" />}
              />
            </div>
            <div className="dialog-b">{children}</div>
            {footer === undefined ? null : (
              <div className="dialog-f">{footer}</div>
            )}
          </Base.Popup>
        </Base.Viewport>
      </Base.Portal>
    </Base.Root>
  );
}
