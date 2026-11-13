// A row the timeline's window went to, held in place (use-window.ts).
//
// [Decision Log] (milestone 11: tracks P and W together)
// - Purpose: find and the outline (track W) go to an entry by its key; the
//   window (track P) draws only the rows near the view, so the row may not
//   exist. Going there means scrolling to where the row is estimated to
//   be; once drawn, the row and those around it are measured, the mean
//   that every undrawn row is guessed at changes, and the spacer above
//   grows or shrinks by thousands of pixels. Without a hold, the row the
//   reader asked for slides away as the heights settle.
// - Alternatives: the browser's scroll anchoring alone (it keeps whichever
//   node it picked still, not the one asked for, and Chromium applies it
//   at its own lifecycle step, so a correction made from script on top of
//   it could count the same shift twice); scrolling again on a timer until
//   the position stops moving (visible steps).
// - Choice: the pin. After every commit of the window (a layout effect,
//   before paint) the pinned row's box (or the match's range in it) is
//   read and `scrollTop` corrected by what `offBy` says, to the nearest
//   whole pixel (`holdStep`), so every painted frame shows it where it
//   was put, within half a pixel. While a pin is held the scroller
//   carries `data-pinned`, which turns the browser's anchoring off
//   (timeline.css), so only this correction moves the view. The pin is
//   let go when the reader takes the scroll back (a wheel, a touch, a
//   press in the timeline, a key), when anything else scrolls the view
//   (a screen reader moving to an element, a script: the pin remembers
//   where it put the view, and any scroll that lands elsewhere is not its
//   own), or when another target replaces it.
// - Cost: a `getBoundingClientRect` of one box per commit while pinned,
//   which forces the layout the browser was about to do anyway.

import { type Align, holdStep, offBy } from "../../data/window";

/** What the window was asked to show. */
export interface Target {
  /** The row's key (data/rows.ts turns an entry's key into it). */
  key: string;
  /** A new number for each request, so asking again goes again. */
  seq: number;
  align: Align;
  /** Called when the row is first drawn (and again if it is drawn anew):
   * mark or focus what was asked for; what to hold in place. */
  show: (row: Element) => Element | Range;
}

/** A target being held. */
export interface Pin {
  readonly target: Target;
  /** What `show` gave, once the row was drawn. */
  anchor: Element | Range | null;
  /** The `scrollTop` the pin last left the view at. */
  at: number;
}

/** Whether a scroll that left `scroller` where it is now was the pin's
 * own: one that lands anywhere else was someone else's, but for the
 * browser holding the view at the end of content that shrank under it
 * (measured rows smaller than guessed, near the log's end). */
export function ownScroll(scroller: HTMLElement, pin: Pin): boolean {
  const top = scroller.scrollTop;
  const clamped =
    top < pin.at && top >= scroller.scrollHeight - scroller.clientHeight - 1;
  return Math.abs(top - pin.at) <= 1 || clamped;
}

/** Whether `anchor` still marks what it marked: in the document, and
 * for a range, not collapsed (React rewriting a text node's data, as a
 * streaming message grows, collapses the ranges inside it). */
function current(anchor: Element | Range): boolean {
  return anchor instanceof Range
    ? anchor.startContainer.isConnected && !anchor.collapsed
    : anchor.isConnected;
}

/** Hold `pin` in `scroller`: show it once its row is drawn, then correct
 * the scroll so it sits where it was put. Whether the scroll moved. */
export function hold(scroller: HTMLElement, pin: Pin): boolean {
  const row = scroller.querySelector(
    `[data-key="${CSS.escape(pin.target.key)}"]`,
  );
  if (row === null) {
    return false;
  }
  if (pin.anchor === null || !current(pin.anchor)) {
    pin.anchor = pin.target.show(row);
  }
  const box = pin.anchor.getBoundingClientRect();
  const view = scroller.getBoundingClientRect();
  const by = offBy(
    box.top - view.top,
    box.height,
    scroller.clientHeight,
    pin.target.align,
  );
  const before = scroller.scrollTop;
  const step = holdStep(by);
  if (step !== 0) {
    scroller.scrollTop = before + step;
  }
  pin.at = scroller.scrollTop;
  return pin.at !== before;
}

/** Mark `scroller` as holding a pin, or not: the browser's own scroll
 * anchoring is off while it does (timeline.css). */
export function markPinned(scroller: HTMLElement, pinned: boolean): void {
  if (pinned) {
    scroller.dataset["pinned"] = "";
  } else {
    delete scroller.dataset["pinned"];
  }
}

/** The events by which a reader takes the scroll back. */
export const TAKE_BACK: readonly string[] = [
  "wheel",
  "touchstart",
  "pointerdown",
  "keydown",
];
