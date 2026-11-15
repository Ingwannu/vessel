// The window's watch on its scroller (use-window.ts): the reader's scroll
// and the scroller's size into the window's view once a frame, whether the
// reader is at the end, a pin let go when the reader takes the scroll
// back, a held row put back when a measured height moved it, and the
// `<details>` toggled in the rows. Divided from use-window.ts, which
// works out what is drawn; this follows what the reader and the browser
// did.

import type { Rows } from "./use-window";
import { followEnd } from "./window-follow";
import { hold, markPinned, ownScroll, type Pin, TAKE_BACK } from "./window-pin";

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

/** Follow scrolling and layout into the window once a frame. End intent
 * survives layout's queued scroll; a pin and an upward reader still win. */
export function watch(
  element: HTMLElement,
  rows: Rows,
  reading: Reading,
  setView: (view: { top: number; height: number }) => void,
): () => void {
  let frame: number | null = null;
  const following = followEnd(element, reading);
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
    if (reading.pin !== null && !ownScroll(element, reading.pin)) takeBack();
    following.scrolled();
    frame ??= window.requestAnimationFrame(read);
  };
  // Container resize and row wrapping can settle at different deliveries.
  // Both must preserve an end reader before a layout scroll can mark them
  // as away; merely watching the container misses later row measurements.
  rows.settle = () => {
    if (reading.pin !== null) hold(element, reading.pin);
    following.settle();
  };
  const undo = listen(element, rows, takeBack, onScroll);
  read();
  return () => {
    undo();
    rows.settle = () => undefined;
    if (frame !== null) window.cancelAnimationFrame(frame);
  };
}

/** Keep the event/observer lifetime together so closing a timeline owns
 * every listener; none of these registrations decides follow intent. */
function listen(
  element: HTMLElement,
  rows: Rows,
  takeBack: () => void,
  onScroll: () => void,
): () => void {
  const toggled = (event: Event) => {
    rememberToggle(rows, event);
  };
  const resized = new ResizeObserver(() => {
    rows.settle();
    onScroll();
  });
  resized.observe(element);
  element.addEventListener("scroll", onScroll, { passive: true });
  for (const name of TAKE_BACK)
    element.addEventListener(name, takeBack, { passive: true });
  // Toggle does not bubble, so remember details on the capture path.
  element.addEventListener("toggle", toggled, { capture: true });
  return () => {
    element.removeEventListener("scroll", onScroll);
    for (const name of TAKE_BACK) element.removeEventListener(name, takeBack);
    element.removeEventListener("toggle", toggled, { capture: true });
    resized.disconnect();
  };
}

function rememberToggle(rows: Rows, event: Event): void {
  const row =
    event.target instanceof Element ? event.target.closest("[data-key]") : null;
  const key = row?.getAttribute("data-key") ?? null;
  if (row !== null && key !== null) {
    rows.opened.set(
      key,
      Array.from(row.querySelectorAll("details"), (details) => details.open),
    );
  }
}
