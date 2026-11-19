// FlowSheet: Paseo's paged picker panel (its Add project sheet,
// `add-project-flow.tsx`, pinned 0.15.4): a panel near the top with a
// title row (a back arrow on later pages, the title, a muted context
// after it), an optional field under it, a list area, and a footer of
// key hints.
//
// [Decision Log]
// - Purpose: TRACKER 234's Add project sheet; Base UI is reached through
//   the kit only (ADR 0021), so the panel is a kit part.
// - Constraint: Paseo's Esc goes one page back (closing on the first),
//   and a press outside closes; the kit's Dialog closes on both and is
//   centred, 520 wide, in agent-orchestrator's values.
// - Choice: Base UI's dialog (focus trap, scroll lock, inert page) with
//   Paseo's panel: 48 from the top, 640 wide (at most 92%), at most 80%
//   high, a hairline, radius 8, shadow lg; the header 16 in, 8 top, 12
//   bottom over a rule; the footer 16 in, 8 high, 16 apart under a rule;
//   the page's own colours. Esc calls `onEscape` (the caller pops a page
//   or closes); an outside press calls `onClose`.
// - Cost: no exit animation (as the kit's Dialog, mounted while open).

import "./flow-sheet.css";

import { Dialog as Base } from "@base-ui/react/dialog";
import type { KeyboardEvent, ReactNode, Ref } from "react";

/** The page's field under the title (Paseo's unboxed input, 14). */
export interface FlowField {
  value: string;
  placeholder: string;
  disabled: boolean;
  onChange: (value: string) => void;
}

import { Icon } from "./icon";

export function FlowSheet({
  label,
  title,
  context,
  field,
  fieldRef,
  footer,
  children,
  popupRef,
  initialFocus,
  onBack,
  onEscape,
  onClose,
  onKeyDown,
}: {
  /** What the panel is, for assistive technology. */
  label: string;
  title: string;
  /** Muted, after the title (Paseo's host label). */
  context?: string;
  /** The field under the title, when the page has one. */
  field?: FlowField | undefined;
  /** The field's element, for the caller's focus (apart from `field`: an
   * object holding a ref would be read as one while drawing). */
  fieldRef?: Ref<HTMLInputElement>;
  footer: ReactNode;
  children: ReactNode;
  popupRef?: Ref<HTMLDivElement>;
  initialFocus?: () => HTMLElement | null;
  /** Shown as the back arrow when given (a later page). */
  onBack?: () => void;
  onEscape: () => void;
  onClose: () => void;
  onKeyDown?: (event: KeyboardEvent) => void;
}) {
  return (
    <Base.Root
      open
      onOpenChange={(open, details) => {
        if (open) return;
        if (details.reason === "escape-key") onEscape();
        else onClose();
      }}
    >
      <Base.Portal>
        <Base.Backdrop className="flow-sheet-backdrop" />
        <Base.Viewport className="flow-sheet-viewport">
          <Base.Popup
            ref={popupRef}
            className="flow-sheet"
            aria-label={label}
            initialFocus={initialFocus}
            onKeyDown={onKeyDown}
          >
            <div className="flow-sheet-header">
              <div className="flow-sheet-title-row">
                {onBack === undefined ? null : (
                  <button
                    type="button"
                    className="flow-sheet-back"
                    aria-label="Back"
                    onClick={onBack}
                  >
                    <Icon name="back" size={18} />
                  </button>
                )}
                {/* Not Base UI's title: the panel is named by `label`
                    (Paseo's "Add project: <page>"), the title is shown. */}
                <div className="flow-sheet-titles">
                  <span className="flow-sheet-title">{title}</span>
                  {context === undefined ? null : (
                    <span className="flow-sheet-context">{context}</span>
                  )}
                </div>
              </div>
              {field === undefined ? null : (
                <input
                  ref={fieldRef}
                  className="flow-sheet-input"
                  value={field.value}
                  placeholder={field.placeholder}
                  aria-label={field.placeholder}
                  disabled={field.disabled}
                  autoCapitalize="none"
                  autoCorrect="off"
                  spellCheck={false}
                  onChange={(event) => {
                    field.onChange(event.target.value);
                  }}
                />
              )}
            </div>
            <div className="flow-sheet-results">{children}</div>
            <div className="flow-sheet-footer">{footer}</div>
          </Base.Popup>
        </Base.Viewport>
      </Base.Portal>
    </Base.Root>
  );
}

/** One footer hint: its keys in one badge, written as Paseo's
 * `formatShortcut` writes them (`↑↓` on a Mac, `↑+↓` elsewhere, `⏎`,
 * `Esc`), then what they do (shown only where a keyboard is likely, as
 * Paseo's `useHardwareKeyboardPresent`). */
export function FlowHint({ keys, action }: { keys: string; action: string }) {
  return (
    <span className="flow-sheet-hint">
      <kbd>{keys}</kbd>
      <span>{action}</span>
    </span>
  );
}
