// Geometry-only assertions and the controlled layout/scroll interleaving
// on an actual question dock; no timers or changes to the original case.
import { expect, type Page } from "@playwright/test";

import { NEAR_END } from "../src/data/follow-end";
import { timeline } from "./kit";

const NARROW_WIDTH = 390;
const DOCK_HEIGHT = 680;
export const PHONE_VIEWPORT = { width: NARROW_WIDTH, height: 844 };

export async function expectWaitingAboveDock(page: Page): Promise<void> {
  const dock = page.getByRole("region", { name: "Question from the agent" });
  const waiting = timeline(page).getByText(/^Waiting on an answer/);
  await expect(async () => {
    const line = await waiting.boundingBox();
    const over = await dock.boundingBox();
    expect(line).not.toBeNull();
    expect(over).not.toBeNull();
    if (line !== null && over !== null)
      expect(line.y + line.height).toBeLessThanOrEqual(over.y);
  }).toPass();
}

export async function resizeBeforeDelivery(page: Page): Promise<void> {
  const geometry = await page.locator(".timeline").evaluate(
    (element, size) => {
      const before = element.clientHeight;
      const main = element.closest(".session-main");
      const dock = main?.querySelector(".session-dock");
      if (!(main instanceof HTMLElement) || !(dock instanceof HTMLElement))
        throw new Error("Expected session layout");
      main.style.width = `${String(size.width)}px`;
      dock.style.height = `${String(size.dock)}px`;
      // Reading geometry commits layout synchronously; scroll can already
      // be queued while the browser has not delivered its resize observer.
      const height = element.clientHeight;
      const gap = element.scrollHeight - element.scrollTop - height;
      element.dispatchEvent(new Event("scroll"));
      return { before, height, gap };
    },
    { width: NARROW_WIDTH, dock: DOCK_HEIGHT },
  );
  expect(geometry.height).toBeGreaterThan(0);
  expect(geometry.height).toBeLessThan(geometry.before);
  expect(geometry.gap).toBeGreaterThan(NEAR_END);
}

export async function leaveEndThenResize(page: Page): Promise<void> {
  expect(
    await page.locator(".timeline").evaluate((element) => element.scrollTop),
  ).toBeGreaterThan(0);
  await page.locator(".timeline").evaluate(async (element) => {
    element.scrollTop = 0;
    element.dispatchEvent(new Event("scroll"));
    const dock = element
      .closest(".session-main")
      ?.querySelector(".session-dock");
    if (!(dock instanceof HTMLElement))
      throw new Error("Expected session dock");
    await new Promise<void>((resolve) => {
      const resized = new ResizeObserver(() => {
        resized.disconnect();
        requestAnimationFrame(() => {
          resolve();
        });
      });
      resized.observe(element);
      dock.style.height = `${String(dock.clientHeight + 1)}px`;
    });
  });
  await expect
    .poll(() =>
      page.locator(".timeline").evaluate((element) => element.scrollTop),
    )
    .toBe(0);
}
