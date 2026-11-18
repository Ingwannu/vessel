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
//   over a rule. A dialog with either is paged: its header is ruled off
//   from the body and its footer too, as Paseo's sheet draws them. Paseo
//   does not focus the search on open, so a dialog with a search takes
//   focus itself; a field there would hide the first rows on a phone
//   behind the keyboard. The search is controlled, so the box and the
//   list it filters always agree. It is a text input with the searchbox
//   role, not `type="search"`: Chromium's search input clears itself on
//   Escape, which here belongs to closing the dialog.
// - Compact (TRACKER 787): below 600px (ui/breakpoints.ts) the same dialog
//   is Paseo's AdaptiveModalSheet, a sheet from the bottom: 65% of the
//   window, raised to 90% by a drag up or by a text field taking focus
//   (Gorhom's keyboard "extend"), closed by a drag down; a handle on top,
//   the top corners at 16, black at 45% behind (sheet-drag.ts). `sheet:
//   "content"` is Paseo's ContextWindowSheet instead: as tall as what it
//   holds, at most 80%. Wider, the card is centred, at most Paseo's 520.
//   A sheet takes focus itself, not its first field: Paseo's sheet opens
//   no keyboard until a field is pressed, and a field focused on open
//   would raise every sheet with one to 90% at once.

import "./dialog.css";

import { Dialog as Base } from "@base-ui/react/dialog";
import { type FocusEvent, type ReactNode, useRef, useState } from "react";

import { COMPACT } from "../breakpoints";
import { IconButton } from "./button";
import { Icon } from "./icon";
import { type SheetFit, type SheetSnap, useSheetDrag } from "./sheet-drag";

/** `md` is Paseo's 520px; `lg` holds a grid of cards. */
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
  actions,
  back,
  search,
  sheet = "snap",
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
  back?: DialogBack;
  search?: DialogSearch;
  /** At a compact width: a sheet at Paseo's snap points (a dialog), or
   * as tall as its content (Paseo's ContextWindowSheet). */
  sheet?: SheetFit;
}) {
  const popup = useRef<HTMLDivElement>(null);
  const [snap, setSnap] = useState<SheetSnap>("low");
  const drag = useSheetDrag(popup, sheet, setSnap, onClose);
  // `paged`, not `sheet`: the kit Sheet owns that class (kit/sheet.css).
  const paged = back !== undefined || search !== undefined;
  const look = `dialog${size === "lg" ? " lg" : ""}${paged ? " paged" : ""}${
    sheet === "content" ? " fit" : ""
  }`;
  // Gorhom's keyboard "extend": a field taking focus (a phone's keyboard
  // opening) raises a snapping sheet to its upper point.
  const extend = (event: FocusEvent<HTMLDivElement>) => {
    const field = event.target;
    if (
      sheet === "snap" &&
      (field instanceof HTMLInputElement ||
        field instanceof HTMLTextAreaElement ||
        (field instanceof HTMLElement && field.isContentEditable)) &&
      window.matchMedia(COMPACT).matches
    ) {
      setSnap("high");
    }
  };
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
        <Base.Backdrop className="dialog-backdrop adaptive" />
        <Base.Viewport className="dialog-viewport adaptive">
          <Base.Popup
            ref={popup}
            className={look}
            data-snap={snap}
            initialFocus={() =>
              window.matchMedia(COMPACT).matches
                ? popup.current
                : firstFocus(popup.current, search !== undefined)
            }
            onFocus={extend}
          >
            {/* The sheet's handle (drawn at a compact width only). */}
            <div className="dialog-handle" aria-hidden="true" {...drag} />
            <div className="dialog-h" {...drag}>
              {back === undefined ? null : (
                <IconButton
                  label={back.label}
                  icon="back"
                  size="sm"
                  onClick={back.onBack}
                />
              )}
              <Base.Title className="dialog-title">{title}</Base.Title>
              {actions === undefined ? null : (
                <div className="dialog-actions">{actions}</div>
              )}
              <Base.Close
                render={<IconButton label="Close" icon="close" size="sm" />}
              />
            </div>
            {search === undefined ? null : (
              <label className="dialog-search">
                <Icon name="search" size={16} />
                <input
                  className="dialog-search-input"
                  type="text"
                  role="searchbox"
                  enterKeyHint="search"
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
