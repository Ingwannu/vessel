// What the measurements (perf.ts) do to a page and read from it once it
// has settled: scrolling a list in fixed steps, resting, and its memory.

import type { CDPSession, Page } from "@playwright/test";

/** A stretch of the page's clock and what each step of it took. */
export interface Stretch {
  readonly start: number;
  readonly end: number;
  /** Each step, from its scroll to the start of the frame after the one
   * that drew it. */
  readonly stepsMs: readonly number[];
}

/** Scroll the element `selector` from one end to the other (`up`: from
 * the bottom to the top) in `steps` equal steps. A step sets
 * `scrollTop`, then waits two animation frames: the first frame draws
 * the new position, the second begins once it has. */
export async function scrollThrough(
  page: Page,
  selector: string,
  steps: number,
  direction: "up" | "down",
): Promise<Stretch> {
  return page.evaluate(
    async ({ selector, steps, up }) => {
      const box = document.querySelector(selector);
      if (box === null) {
        throw new Error(`${selector} is not on the page`);
      }
      const frame = async (): Promise<number> =>
        new Promise((resolve) => {
          window.requestAnimationFrame(resolve);
        });
      const span = box.scrollHeight - box.clientHeight;
      const at = (step: number): number =>
        up ? span - (span * step) / steps : (span * step) / steps;
      box.scrollTop = at(0);
      await frame();
      await frame();
      const start = performance.now();
      const stepsMs: number[] = [];
      for (let step = 1; step <= steps; step += 1) {
        const before = performance.now();
        box.scrollTop = at(step);
        await frame();
        await frame();
        stepsMs.push(performance.now() - before);
      }
      return { start, end: performance.now(), stepsMs };
    },
    { selector, steps, up: direction === "up" },
  );
}

/** Leave the page alone for `ms`; the stretch, in its clock. The wait is
 * the measurement (what the page does at rest), not a wait for an event. */
export async function rest(page: Page, ms: number): Promise<Stretch> {
  return page.evaluate(async (ms) => {
    const start = performance.now();
    await new Promise((resolve) => window.setTimeout(resolve, ms));
    return { start, end: performance.now(), stepsMs: [] };
  }, ms);
}

/** A settled page's size. */
export interface Footprint {
  /** `JSHeapUsedSize` after a full garbage collection. */
  readonly heapBytes: number;
  /** Elements in the document. */
  readonly elements: number;
}

/** Collect garbage, then read the heap and count the elements. */
export async function pageFootprint(
  page: Page,
  cdp: CDPSession,
): Promise<Footprint> {
  await cdp.send("HeapProfiler.enable", {});
  await cdp.send("HeapProfiler.collectGarbage", {});
  await cdp.send("Performance.enable", {});
  const { metrics } = await cdp.send("Performance.getMetrics", {});
  const heap = metrics.find((metric) => metric.name === "JSHeapUsedSize");
  const elements = await page.evaluate(
    () => document.getElementsByTagName("*").length,
  );
  return { heapBytes: heap?.value ?? Number.NaN, elements };
}
