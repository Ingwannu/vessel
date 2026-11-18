// Toasts (DESIGN.md §6.10): one at a time below the header, clear of the
// lower composer and dock. A new toast replaces the one up (toast-life.ts);
// success leaves after 4 s, an error stays until dismissed.

import { useStore } from "../data/store";
import { IconButton } from "./kit";
import {
  dismissToast,
  holdToast,
  releaseToast,
  shownToast,
} from "./toast-life";
import { useToastClearance } from "./use-toast-clearance";

export { toast } from "./toast-life";

export function Toasts() {
  const shown = useStore(shownToast, (value) => value);
  const host = useToastClearance(shown !== null);
  // One card for whichever toast is up, keyed by tone and not by toast: a
  // replacement of the same tone changes the words of the live region in
  // place, as Paseo's single surface (a new node per toast may go
  // unannounced); a change between status and alert is a new region.
  return (
    <div ref={host} className="toast-host">
      {shown === null ? null : (
        <div
          key={shown.tone}
          className={shown.tone === "error" ? "toast error" : "toast"}
          role={shown.tone === "error" ? "alert" : "status"}
          // Paseo holds the life on hover only (web); a touch is not a rest.
          onPointerEnter={(event) => {
            if (event.pointerType !== "touch") holdToast(shown.id);
          }}
          onPointerLeave={(event) => {
            if (event.pointerType !== "touch") releaseToast(shown.id);
          }}
        >
          <span>{shown.text}</span>
          <IconButton
            label="Dismiss"
            icon="close"
            size="xs"
            onClick={() => {
              dismissToast(shown.id);
            }}
          />
        </div>
      )}
    </div>
  );
}
