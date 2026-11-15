// A fixed toast host must fit above the live composer, including its docks.
// Resize alone misses a later-mounted prompt; observing every chat mutation
// would do needless layout work. Rebind only when a boundary enters/leaves,
// and let ResizeObserver own its size changes (question folds, input growth).
import { type RefObject, useLayoutEffect, useRef } from "react";

const BOUNDARIES = ".prompt-dock, .prompt-menu";

function changesBoundary(records: readonly MutationRecord[]): boolean {
  return records.some((record) =>
    [...record.addedNodes, ...record.removedNodes].some(
      (node) =>
        node instanceof Element &&
        (node.matches(BOUNDARIES) || node.querySelector(BOUNDARIES) !== null),
    ),
  );
}

/** One host owns its observers; closing the last toast releases them. */
export function useToastClearance(
  active: boolean,
): RefObject<HTMLDivElement | null> {
  const host = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const stack = host.current;
    if (!active || stack === null) return;
    let stopped = false;
    const measure = () => {
      if (stopped) return;
      const box = stack.getBoundingClientRect();
      let bottom = window.innerHeight;
      for (const boundary of document.querySelectorAll(BOUNDARIES)) {
        const rect = boundary.getBoundingClientRect();
        if (
          rect.width > 0 &&
          rect.height > 0 &&
          rect.right > box.left &&
          rect.left < box.right
        ) {
          bottom = Math.min(bottom, rect.top);
        }
      }
      stack.style.setProperty(
        "--toast-available-height",
        `${Math.max(0, bottom - box.top)}px`,
      );
    };
    const resized = new ResizeObserver(measure);
    const bind = () => {
      if (stopped) return;
      resized.disconnect();
      resized.observe(document.documentElement);
      for (const boundary of document.querySelectorAll(BOUNDARIES))
        resized.observe(boundary);
      measure();
    };
    const mounted = new MutationObserver((records) => {
      if (changesBoundary(records)) bind();
    });
    mounted.observe(document.body, { childList: true, subtree: true });
    window.addEventListener("resize", measure);
    document.addEventListener("scroll", measure, {
      capture: true,
      passive: true,
    });
    bind();
    return () => {
      stopped = true;
      mounted.disconnect();
      resized.disconnect();
      window.removeEventListener("resize", measure);
      document.removeEventListener("scroll", measure, { capture: true });
      stack.style.removeProperty("--toast-available-height");
    };
  }, [active]);
  return host;
}
