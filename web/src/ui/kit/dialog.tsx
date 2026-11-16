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
// - Header back and header search (Paseo's SheetHeader, for the label
//   manager): a back arrow before the title, and a search under it,
//   unboxed (a 16px muted icon and the text, no fill, no clear button)
//   over a rule. A dialog with either is a sheet: its header is ruled off
//   from the body and its footer too, as Paseo's sheet draws them. Paseo
//   does not focus the search on open, so a sheet with a search takes
//   focus itself; a field there would hide the first rows on a phone
//   behind the keyboard. The search is controlled, so the box and the
//   list it filters always agree.

import "./dialog.css";

import { Dialog as Base } from "@base-ui/react/dialog";
import { type ReactNode, useRef } from "react";

import { IconButton } from "./button";
import { Icon } from "./icon";

/** `md` is the reference's 512px; `lg` holds a grid of cards. */
export type DialogSize = "md" | "lg";

/** A search in the dialog's header (Paseo's sheet header search). */
export interface DialogSearch {
  value: string;
  onChange: (value: string) => void;
  /** Also its name for a screen reader. */
  placeholder: string;
}

/** A back arrow before the title, for a dialog with a second view. */
export interface DialogBack {
  label: string;
  onBack: () => void;
}

/** What takes focus when `popup` opens. */
function firstFocus(
  popup: HTMLElement | null,
  searched: boolean,
): HTMLElement | null {
  if (popup === null || searched) {
    return popup;
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
  back,
  search,
}: {
  title: string;
  /** Escape, the scrim and the close button all call it. */
  onClose: () => void;
  children: ReactNode;
  /** Actions, right-aligned: a ghost Cancel, then one primary. */
  footer?: ReactNode;
  size?: DialogSize;
  back?: DialogBack;
  search?: DialogSearch;
}) {
  const popup = useRef<HTMLDivElement>(null);
  const sheet = back !== undefined || search !== undefined;
  const look = `dialog${size === "lg" ? " lg" : ""}${sheet ? " sheet" : ""}`;
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
            className={look}
            initialFocus={() => firstFocus(popup.current, search !== undefined)}
          >
            <div className="dialog-h">
              {back === undefined ? null : (
                <IconButton
                  label={back.label}
                  icon="back"
                  size="sm"
                  onClick={back.onBack}
                />
              )}
              <Base.Title className="dialog-title">{title}</Base.Title>
              <Base.Close
                render={<IconButton label="Close" icon="close" size="sm" />}
              />
            </div>
            {search === undefined ? null : (
              <label className="dialog-search">
                <Icon name="search" size={16} />
                <input
                  className="dialog-search-input"
                  type="search"
                  value={search.value}
                  placeholder={search.placeholder}
                  aria-label={search.placeholder}
                  autoComplete="off"
                  spellCheck={false}
                  onChange={(event) => {
                    search.onChange(event.currentTarget.value);
                  }}
                />
              </label>
            )}
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
