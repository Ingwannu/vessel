// Toasts (DESIGN.md §6.10): one at a time below the header, clear of the
// lower composer and dock, as Paseo's ToastHost and ToastCard
// (toast-host.tsx 81-130, 230-275). A new toast replaces the one up and
// runs its own life (toast-life.ts); the card has no close button.

import { useStore } from "../data/store";
import { Icon, type IconName } from "./kit";
import {
  holdToast,
  releaseToast,
  shownToast,
  type ToastVariant,
} from "./toast-life";
import { useToastClearance } from "./use-toast-clearance";

export {
  ERROR_TOAST_LIFE_MS,
  TOAST_LIFE_MS,
  toast,
  toastCopied,
} from "./toast-life";

/** Paseo's leading mark per variant (CheckCircle2, Info, AlertTriangle);
 * its default variant has none. */
const MARKS: Readonly<Record<ToastVariant, IconName | null>> = {
  default: null,
  info: "info",
  success: "circle-check",
  warning: "warn",
  error: "warn",
};

export function Toasts() {
  const shown = useStore(shownToast, (value) => value);
  const host = useToastClearance(shown !== null);
  const mark = shown === null ? null : MARKS[shown.variant];
  const urgent = shown?.variant === "error" || shown?.variant === "warning";
  // One card for whichever toast is up, keyed by its role and not by
  // toast: a replacement changes the words of the live region in place,
  // as Paseo's single surface (a new node per toast may go unannounced);
  // a change between status and alert is a new region.
  return (
    <div ref={host} className="toast-host">
      {shown === null ? null : (
        <div
          key={urgent ? "alert" : "status"}
          className={`toast ${shown.variant}${shown.copied ? " copied" : ""}`}
          role={urgent ? "alert" : "status"}
          // Paseo holds the life on hover only (web); a touch is not a rest.
          onPointerEnter={(event) => {
            if (event.pointerType !== "touch") holdToast(shown.id);
          }}
          onPointerLeave={(event) => {
            if (event.pointerType !== "touch") releaseToast(shown.id);
          }}
        >
          {mark === null ? null : (
            <span className="toast-mark">
              <Icon name={mark} size={18} />
            </span>
          )}
          <span className="toast-text">{shown.text}</span>
        </div>
      )}
    </div>
  );
}
