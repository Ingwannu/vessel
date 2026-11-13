// The window's watch on its scroller (use-window.ts): the reader's scroll
// and the scroller's size into the window's view once a frame, whether the
// reader is at the end, a pin let go when the reader takes the scroll
// back, a held row put back when a measured height moved it, and the
// `<details>` toggled in the rows. Divided from use-window.ts, which
// works out what is drawn; this follows what the reader and the browser
// did.

import type { Rows } from "./use-window";
import { hold, markPinned, ownScroll, type Pin, TAKE_BACK } from "./window-pin";

/** How close to the end counts as reading the end, in pixels. */
const NEAR_END = 48;

/** Where the reader is: following the end, holding a pin, and how many
 * passes this frame's pin has had. */
export interface Reading {
  /** Whether this list follows its end at all (`useWindow`'s `follow`). */
  follows: boolean;
  following: boolean;
  pin: Pin | null;
  passes: number;
  /** The last target taken, by `seq`. */
  taken: number | null;
}

/** `element`'s view as the window reads it. */
export function viewOf(element: HTMLElement): { top: number; height: number } {
  return { top: element.scrollTop, height: element.clientHeight };
}

/** Follow `element`'s scrolling and size into `setView`, once a frame;
 * keep `reading` told whether the reader is at the end and let its pin go
 * when the reader takes the scroll back; and remember the `<details>`
 * toggled in its rows. Returns the undo. */
export function watch(
  element: HTMLElement,
  rows: Rows,
  reading: Reading,
  setView: (view: { top: number; height: number }) => void,
): () => void {
  let frame: number | null = null;
  const read = () => {
    frame = null;
    reading.passes = 0;
    setView(viewOf(element));
  };
  const takeBack = () => {
    if (reading.pin !== null) {
      reading.pin = null;
      markPinned(element, false);
    }
  };
  const onScroll = () => {
    if (reading.pin !== null && !ownScroll(element, reading.pin)) {
      takeBack();
    }
    if (reading.pin === null && reading.follows) {
      reading.following =
        element.scrollHeight - element.scrollTop - element.clientHeight <
        NEAR_END;
    }
    frame ??= window.requestAnimationFrame(read);
  };
  const onToggle = (event: Event) => {
    const row =
      event.target instanceof Element
        ? event.target.closest("[data-key]")
        : null;
    const key = row?.getAttribute("data-key") ?? null;
    if (row !== null && key !== null) {
      rows.opened.set(
        key,
        Array.from(row.querySelectorAll("details"), (details) => details.open),
      );
    }
  };
  // A row's height is measured after layout and before paint, and React
  // draws the new heights a frame later: a held row is put back now, so
  // the frame between does not show it moved (1.28 px under load).
  rows.settle = () => {
    if (reading.pin !== null) {
      hold(element, reading.pin);
    }
  };
  // The view growing shorter (a question or the queue opening over the
  // composer, a phone's keyboard) does not take a reader at the end off
  // it: read as a scroll, the dock's height would count as scrolling up
  // past `NEAR_END`, and the last row would stay under the dock.
  const resized = new ResizeObserver(() => {
    if (reading.pin === null && reading.following) {
      element.scrollTop = element.scrollHeight;
    }
    onScroll();
  });
  resized.observe(element);
  read();
  element.addEventListener("scroll", onScroll, { passive: true });
  for (const name of TAKE_BACK) {
    element.addEventListener(name, takeBack, { passive: true });
  }
  // `toggle` does not bubble; it is seen on its way down.
  element.addEventListener("toggle", onToggle, { capture: true });
  return () => {
    element.removeEventListener("scroll", onScroll);
    for (const name of TAKE_BACK) {
      element.removeEventListener(name, takeBack);
    }
    element.removeEventListener("toggle", onToggle, { capture: true });
    resized.disconnect();
    rows.settle = () => undefined;
    if (frame !== null) {
      window.cancelAnimationFrame(frame);
    }
  };
}
