// What the page records about itself while it is measured (perf.ts),
// from its first script on: installed with `addInitScript`, so it runs
// before the page's own code in every document of the context.
//
// [Decision Log]
// - Purpose: times only the page knows exactly: the frame that first
//   drew a state (the board usable, a timeline's first and last entry, a
//   terminal's last line), each long task, the start of every frame, a
//   person's click or Enter.
// - Constraint: a function handed to the page is serialised; it reaches
//   nothing of this module but its argument. The lint allows no cast, so
//   a `window.__probe` object could only be read back untyped.
// - Choice: the probe writes to the page's own performance timeline,
//   marks and measures named `vessel:*`, read back through the typed
//   `performance.getEntriesByName` (`readMarks`, `waitForMark`).
// - Alternatives: a CDP trace (exact, but one of a 20,000-entry timeline
//   is hundreds of MB and needs a trace parser); polling from the test
//   (its round trips would be in every number).
// - Cost: the probe runs on the main thread it measures: one
//   animation-frame callback per frame with a few `querySelector` calls
//   (constant per frame but for the board's card count), and one mark
//   per frame (60 a second; the user-timing buffer has no limit).

import type { Page } from "@playwright/test";

import type { Task } from "./stats";

/** A state the probe marks the first frame of, named `vessel:<name>`. */
export type Watch =
  /** The board usable: `cards` session cards in the document, the
   * first lane and the first card laid out with a box, and no font face
   * still loading. */
  | { readonly kind: "board"; readonly name: string; readonly cards: number }
  /** An element matching `selector` laid out with a box. */
  | {
      readonly kind: "present";
      readonly name: string;
      readonly selector: string;
    }
  /** The element before the last child of `parent` (a timeline's last
   * entry: the column's last child is its scroll anchor) containing
   * `text`; constant time however long the column is. */
  | {
      readonly kind: "last";
      readonly name: string;
      readonly parent: string;
      readonly text: string;
    }
  /** The text of the element matching `selector` containing `text`. */
  | {
      readonly kind: "text";
      readonly name: string;
      readonly selector: string;
      readonly text: string;
    }
  /** Find's current match (the `vessel-find-active` highlight) reading
   * `text`, drawn inside the timeline's view: a match gone to and
   * shown. */
  | {
      readonly kind: "match";
      readonly name: string;
      readonly text: string;
    };

/** Install the probe in the page: run in the page, before its scripts. */
function probe(watches: readonly Watch[]): void {
  const waiting = new Set(watches);
  const laidOut = (element: Element | null): boolean =>
    element !== null && element.getBoundingClientRect().height > 0;
  const seen = (watch: Watch): boolean => {
    switch (watch.kind) {
      case "board": {
        const cards = document.querySelectorAll("li.card");
        // Reading a box lays the whole document out, every card with the
        // first, and laying text out asks for the faces it draws: the
        // fonts' status read after it covers them.
        return (
          cards.length >= watch.cards &&
          laidOut(document.querySelector("section.lane")) &&
          laidOut(cards.item(0)) &&
          document.fonts.status === "loaded"
        );
      }
      case "present":
        return laidOut(document.querySelector(watch.selector));
      case "last": {
        const last =
          document.querySelector(watch.parent)?.lastElementChild
            ?.previousElementSibling ?? null;
        return last?.textContent.includes(watch.text) ?? false;
      }
      case "text":
        return (
          document
            .querySelector(watch.selector)
            ?.textContent.includes(watch.text) ?? false
        );
      case "match": {
        const view = document.querySelector(".timeline");
        const active = CSS.highlights.get("vessel-find-active");
        if (view === null || active === undefined) {
          return false;
        }
        const bounds = view.getBoundingClientRect();
        for (const range of active) {
          if (
            range instanceof Range &&
            range.startContainer.isConnected &&
            range.toString() === watch.text
          ) {
            const box = range.getBoundingClientRect();
            return box.top >= bounds.top && box.bottom <= bounds.bottom;
          }
        }
        return false;
      }
    }
  };
  const frame = (time: number): void => {
    performance.mark("vessel:frame", { startTime: time });
    for (const watch of waiting) {
      if (seen(watch)) {
        performance.mark(`vessel:${watch.name}`, { startTime: time });
        waiting.delete(watch);
      }
    }
    window.requestAnimationFrame(frame);
  };
  window.requestAnimationFrame(frame);
  new PerformanceObserver((list) => {
    for (const task of list.getEntries()) {
      performance.measure("vessel:longtask", {
        start: task.startTime,
        duration: task.duration,
      });
    }
  }).observe({ type: "longtask", buffered: true });
  // The moments a person acts, as the page saw them.
  document.addEventListener(
    "keydown",
    (event) => {
      if (event.key === "Enter") {
        performance.mark("vessel:enter", { startTime: event.timeStamp });
      }
    },
    { capture: true },
  );
  document.addEventListener(
    "click",
    (event) => {
      performance.mark("vessel:click", { startTime: event.timeStamp });
    },
    { capture: true },
  );
  // A query typed into find: `vessel:input:<the field's text>`.
  document.addEventListener(
    "input",
    (event) => {
      const field = event.target;
      if (
        field instanceof HTMLInputElement &&
        field.getAttribute("aria-label") === "Find in session"
      ) {
        performance.mark(`vessel:input:${field.value}`, {
          startTime: event.timeStamp,
        });
      }
    },
    { capture: true },
  );
}

/** Install the probe watching `watches` in every document `page` opens
 * from now on. */
export async function installProbe(
  page: Page,
  watches: readonly Watch[],
): Promise<void> {
  await page.addInitScript(probe, watches);
}

/** What the probe recorded so far. */
export interface Marks {
  /** Each watched state and act, by name: when it was first seen. */
  readonly at: ReadonlyMap<string, number>;
  /** The start of every frame, in order. */
  readonly frames: readonly number[];
  readonly tasks: readonly Task[];
  /** First contentful paint, if the document has painted. */
  readonly fcp: number | null;
}

export async function readMarks(page: Page): Promise<Marks> {
  const raw = await page.evaluate(() => {
    const named = performance
      .getEntriesByType("mark")
      .filter(
        (mark) =>
          mark.name.startsWith("vessel:") && mark.name !== "vessel:frame",
      )
      .map((mark) => ({
        name: mark.name.slice("vessel:".length),
        at: mark.startTime,
      }));
    return {
      named,
      frames: performance
        .getEntriesByName("vessel:frame")
        .map((mark) => mark.startTime),
      tasks: performance
        .getEntriesByName("vessel:longtask")
        .map((task) => ({ start: task.startTime, duration: task.duration })),
      fcp:
        performance.getEntriesByName("first-contentful-paint")[0]?.startTime ??
        null,
    };
  });
  return {
    // The first of a name wins: a second click or Enter is a later act.
    at: new Map(raw.named.reverse().map((mark) => [mark.name, mark.at])),
    frames: raw.frames,
    tasks: raw.tasks,
    fcp: raw.fcp,
  };
}

/** When `vessel:<name>` was marked, waiting up to `ms` for it. */
export async function waitForMark(
  page: Page,
  name: string,
  ms: number,
): Promise<number> {
  return page.evaluate(
    async ({ mark, ms }) =>
      new Promise<number>((resolve, reject) => {
        const observer = new PerformanceObserver((list) => {
          const hit = list.getEntriesByName(mark)[0];
          if (hit !== undefined) {
            observer.disconnect();
            window.clearTimeout(timer);
            resolve(hit.startTime);
          }
        });
        const timer = window.setTimeout(() => {
          observer.disconnect();
          reject(new Error(`${mark} was not seen within ${String(ms)} ms`));
        }, ms);
        observer.observe({ type: "mark", buffered: true });
      }),
    { mark: `vessel:${name}`, ms },
  );
}

/** The page's frame interval at rest: the median gap over `count`
 * frames, measured now. */
export async function restingFrame(page: Page, count = 60): Promise<number> {
  return page.evaluate(async (count) => {
    const starts: number[] = [];
    await new Promise<void>((resolve) => {
      const frame = (time: number): void => {
        starts.push(time);
        if (starts.length > count) {
          resolve();
        } else {
          window.requestAnimationFrame(frame);
        }
      };
      window.requestAnimationFrame(frame);
    });
    const gaps = starts
      .slice(1)
      .map((start, index) => start - (starts[index] ?? start))
      .sort((a, b) => a - b);
    return gaps[Math.floor(gaps.length / 2)] ?? Number.NaN;
  }, count);
}
