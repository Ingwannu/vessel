// A compact dialog's sheet following a finger (Paseo's AdaptiveModalSheet
// on Gorhom's bottom sheet: snap points 65% and 90%, `enablePanDownToClose`,
// `keyboardBehavior="extend"`; ContextWindowSheet: as tall as its content,
// at most 80%, pan down to close).
//
// [Decision Log]
// - Purpose: TRACKER 787. At a compact width (ui/breakpoints.ts) every page
//   dialog was a centred card; Paseo's is a sheet from the bottom that a
//   drag on its handle or header raises to 90%, lowers to 65%, or closes.
// - Constraint: the page has no gesture library, and Base UI's Dialog owns
//   focus, the scrim and Escape; the sheet's height is CSS (dialog.css),
//   which a drag must override while the finger is down only.
// - Choice: pointer events on the handle and the header (capture, so a
//   finger leaving them keeps the drag), the sheet's height and offset
//   written straight onto it while dragging (no drawing per move), and a
//   pure `settle` that picks where it lands from where it is and how fast
//   it moves, as Gorhom's snapping does.
// - Cost: the body's own scroll does not pull the sheet down (Paseo's
//   scrolling sheets turn content panning off too); "extend" follows a
//   text field taking focus, which on a phone is when its keyboard opens.

import { type PointerEvent, useRef } from "react";

import { COMPACT } from "../breakpoints";

/** Where a snapping sheet rests: Paseo's 65% and 90% of the window. */
export type SheetSnap = "low" | "high";

/** A sheet that snaps (a dialog), or one as tall as its content. */
export type SheetFit = "snap" | "content";

/** Paseo's snap points, as fractions of the window's height. */
export const SNAP_LOW = 0.65;
export const SNAP_HIGH = 0.9;

/** A fling: faster than this (px per ms) decides by direction. */
const FLING = 0.6;

/** Where a released sheet goes. */
export type Settled = SheetSnap | "close";

export interface Release {
  fit: SheetFit;
  /** The sheet's height when the drag started, and shown now (px). */
  start: number;
  shown: number;
  /** The window's height. */
  window: number;
  /** Down is positive (px per ms). */
  velocity: number;
}

/** Where a sheet released at `release` comes to rest. */
export function settle(release: Release): Settled {
  const { fit, start, shown, velocity } = release;
  if (fit === "content") {
    return velocity > FLING || shown < start * 0.75 ? "close" : "low";
  }
  const low = release.window * SNAP_LOW;
  const high = release.window * SNAP_HIGH;
  if (velocity > FLING) {
    // From the top a fling lands on the lower snap; from there it closes.
    return start > low + 1 ? "low" : "close";
  }
  if (velocity < -FLING) {
    return "high";
  }
  if (shown >= (low + high) / 2) {
    return "high";
  }
  return shown >= low * 0.75 ? "low" : "close";
}

/** The height and the downward offset a snapping sheet is drawn at while
 * `shown` pixels of it would be above the window's foot. */
export function dragged(
  fit: SheetFit,
  start: number,
  shown: number,
  window: number,
): { height: number; offset: number } {
  if (fit === "content") {
    return { height: start, offset: Math.max(0, start - shown) };
  }
  const low = window * SNAP_LOW;
  const high = window * SNAP_HIGH;
  const height = Math.min(Math.max(shown, Math.min(start, low)), high);
  return { height, offset: Math.max(0, height - shown) };
}

interface Drag {
  pointer: number;
  y: number;
  start: number;
  shown: number;
  /** The last move's time and place, for the release's speed. */
  at: number;
  lastY: number;
  velocity: number;
}

/** The handlers a sheet's handle and header take. */
export interface SheetDragHandlers {
  onPointerDown: (event: PointerEvent<HTMLElement>) => void;
  onPointerMove: (event: PointerEvent<HTMLElement>) => void;
  onPointerUp: (event: PointerEvent<HTMLElement>) => void;
  onPointerCancel: (event: PointerEvent<HTMLElement>) => void;
}

/** Drag the sheet `sheet` (only at a compact width); a release snaps it
 * through `onSnap` or closes it through `onClose`. */
export function useSheetDrag(
  sheet: { readonly current: HTMLElement | null },
  fit: SheetFit,
  onSnap: (snap: SheetSnap) => void,
  onClose: () => void,
): SheetDragHandlers {
  const drag = useRef<Drag | null>(null);
  const clear = (element: HTMLElement) => {
    element.classList.remove("dragging");
    element.style.removeProperty("height");
    element.style.removeProperty("translate");
  };
  const end = (event: PointerEvent<HTMLElement>, released: boolean) => {
    const now = drag.current;
    const element = sheet.current;
    if (now === null || now.pointer !== event.pointerId || element === null) {
      return;
    }
    drag.current = null;
    clear(element);
    if (!released) return;
    // A finger that stopped before lifting flings nothing.
    const still = event.timeStamp - now.at > 100;
    const landed = settle({
      fit,
      start: now.start,
      shown: now.shown,
      window: window.innerHeight,
      velocity: still ? 0 : now.velocity,
    });
    if (landed === "close") onClose();
    else onSnap(landed);
  };
  return {
    onPointerDown: (event) => {
      const element = sheet.current;
      if (
        element === null ||
        event.button !== 0 ||
        !window.matchMedia(COMPACT).matches ||
        // A control in the header keeps its own press.
        (event.target instanceof Element &&
          event.target.closest("button, input, a") !== null)
      ) {
        return;
      }
      event.currentTarget.setPointerCapture(event.pointerId);
      const start = element.getBoundingClientRect().height;
      drag.current = {
        pointer: event.pointerId,
        y: event.clientY,
        start,
        shown: start,
        at: event.timeStamp,
        lastY: event.clientY,
        velocity: 0,
      };
      element.classList.add("dragging");
    },
    onPointerMove: (event) => {
      const now = drag.current;
      const element = sheet.current;
      if (now === null || now.pointer !== event.pointerId || element === null) {
        return;
      }
      const elapsed = event.timeStamp - now.at;
      if (elapsed > 0) {
        now.velocity = (event.clientY - now.lastY) / elapsed;
      }
      now.at = event.timeStamp;
      now.lastY = event.clientY;
      now.shown = now.start - (event.clientY - now.y);
      const { height, offset } = dragged(
        fit,
        now.start,
        now.shown,
        window.innerHeight,
      );
      element.style.height = `${String(height)}px`;
      element.style.translate = `0 ${String(offset)}px`;
    },
    onPointerUp: (event) => {
      end(event, true);
    },
    onPointerCancel: (event) => {
      end(event, false);
    },
  };
}
