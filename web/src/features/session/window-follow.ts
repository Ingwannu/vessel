// The timeline's end-following geometry, shared by scroll, container
// resize and row measurement. Pins stay owned by window-pin.ts.
import { followAfterScroll, type EndView } from "../../data/follow-end";
import type { Reading } from "./window-watch";

function view(element: HTMLElement): EndView {
  return {
    top: element.scrollTop,
    height: element.clientHeight,
    width: element.clientWidth,
    content: element.scrollHeight,
  };
}

/** Preserve end intent across layout with the existing near-end reading rule. */
export function followEnd(element: HTMLElement, reading: Reading) {
  let before = view(element);
  const changed = (source: "scroll" | "layout") => {
    const after = view(element);
    if (reading.pin === null && reading.follows) {
      const decision = followAfterScroll({
        before,
        after,
        following: reading.following,
      });
      switch (decision) {
        case "away":
          reading.following = false;
          break;
        case "end":
        case "near":
          reading.following = true;
          if (decision === "end" || source === "layout") {
            element.scrollTop = element.scrollHeight;
          }
          break;
      }
    }
    before = view(element);
  };
  return {
    scrolled: () => {
      changed("scroll");
    },
    settle: () => {
      changed("layout");
    },
  };
}
