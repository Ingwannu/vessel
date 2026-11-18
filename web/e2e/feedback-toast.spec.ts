// The toast host stays clear while an actual agent question waits and
// shows one toast at a time as Paseo's ToastHost and ToastCard: a new toast
// replaces the one up with its own life (2.2 s, an error 3.2 s), the card
// leads with its variant's mark and has no close button, a resting mouse
// holds its life; and it stays above a dialog opened while it is up
// (Paseo's planes). Each toast's life runs on a paused clock: a loaded run
// may take longer than 2.2 s to measure.
import { expect, type Locator, type Page, test } from "@playwright/test";
import {
  clearToastBounds,
  copyToast,
  FOCUS_FRAME_MS,
  pauseClock,
  repeatedToasts,
} from "./feedback-kit";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";

const HOLD_AFTER_REPEAT_MS = 500;
/** Paseo's DEFAULT_DURATION_MS and its `toast.error` duration. */
const TOAST_LIFE_MS = 2200;
const ERROR_TOAST_LIFE_MS = 3200;

/** The host's bottom is at or above the dock's top, in the current layout. */
async function clearOfDock(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const host = document.querySelector(".toast-host");
    const dock = document.querySelector(".prompt-dock");
    if (host === null || dock === null) return false;
    return (
      host.getBoundingClientRect().bottom <=
      dock.getBoundingClientRect().top + 0.5
    );
  });
}

/** A press at the control's middle lands on the control. */
async function hitsItself(control: Locator): Promise<boolean> {
  return control.evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const hit = document.elementFromPoint(
      rect.x + rect.width / 2,
      rect.y + rect.height / 2,
    );
    return hit !== null && element.contains(hit);
  });
}

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(colorScheme, () => {
    test.use({ colorScheme });
    for (const width of [390, 800, 1280]) {
      test(`a ${width}px toast clears the composer and question buttons below the header`, async ({
        page,
      }) => {
        const errors = watchPageErrors(page);
        const origin = await openUi(page);
        await page.context().grantPermissions(["clipboard-write"], { origin });
        await startSession(page, "laptop", "Structured", "ask claude");
        await expect(
          page.getByRole("region", { name: "Question from the agent" }),
        ).toBeVisible();
        await page.setViewportSize({ width, height: 844 });
        await pauseClock(page);
        await copyToast(page, "paused");
        await page.clock.runFor(FOCUS_FRAME_MS);
        await clearToastBounds(page);
        await page.clock.runFor(TOAST_LIFE_MS);
        await expect(page.locator(".toast-host")).toBeHidden();
        await repeatedToasts(page);
        await page.clock.resume();
        expect(errors()).toBe(0);
      });
    }
    test("a short phone keeps question controls clickable through resize and folding", async ({
      page,
    }) => {
      const origin = await openUi(page);
      await page.context().grantPermissions(["clipboard-write"], { origin });
      await startSession(page, "laptop", "Structured", "ask claude");
      await pauseClock(page);
      await repeatedToasts(page);
      await page.clock.runFor(HOLD_AFTER_REPEAT_MS);
      await page.setViewportSize({ width: 390, height: 600 });
      // Clear of the dock in the very layout the viewport change makes,
      // with no frame for a measurement to catch up in.
      expect(await clearOfDock(page)).toBe(true);
      await page.clock.runFor(FOCUS_FRAME_MS);
      await expect(page.locator(".toast")).toHaveCount(1);
      const collapse = page.getByRole("button", { name: "Collapse question" });
      expect(await hitsItself(collapse)).toBe(true);
      // The frozen clock holds focus frames; this still sends an actual click.
      await collapse.click({ force: true });
      await page.clock.runFor(FOCUS_FRAME_MS);
      const expand = page.getByRole("button", { name: "Expand question" });
      await expect(expand).toBeVisible();
      expect(await clearOfDock(page)).toBe(true);
      expect(await hitsItself(expand)).toBe(true);
      await expand.click({ force: true });
      await page.clock.runFor(FOCUS_FRAME_MS);
      await expect(collapse).toBeVisible();
      expect(await clearOfDock(page)).toBe(true);
      await page.clock.resume();
    });
  });
}

test("a new toast replaces the one up and runs its own 2.2 s", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await pauseClock(page);
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.clock.runFor(TOAST_LIFE_MS - 1000);
  await copyToast(page, "paused");
  // Past the first toast's 2.2 s: the second is up, alone.
  await page.clock.runFor(1500);
  await expect(page.locator(".toast")).toHaveCount(1);
  // Past the second's own 2.2 s.
  await page.clock.runFor(TOAST_LIFE_MS - 1500 + 500);
  await expect(page.locator(".toast-host")).toBeHidden();
  await page.clock.resume();
});

test("an error toast leads with its mark, has no close, and leaves at 3.2 s", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "ask claude");
  // The clipboard refuses, so the copy says so as an error.
  await page.evaluate(() => {
    Object.defineProperty(navigator.clipboard, "writeText", {
      configurable: true,
      value: () => Promise.reject(new Error("refused")),
    });
  });
  await pauseClock(page);
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  const alert = page.locator(".toast.error");
  await expect(alert).toHaveText("The clipboard refused the text");
  await expect(alert).toHaveAttribute("role", "alert");
  await expect(alert.locator(".toast-mark .ico-warn")).toBeVisible();
  await expect(alert.getByRole("button")).toHaveCount(0);
  await page.clock.runFor(TOAST_LIFE_MS);
  await expect(alert).toBeVisible();
  await page.clock.runFor(ERROR_TOAST_LIFE_MS - TOAST_LIFE_MS);
  await expect(page.locator(".toast-host")).toBeHidden();
  await page.clock.resume();
});

test("a pointer resting on the toast holds its life", async ({ page }) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await pauseClock(page);
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  const card = await page.locator(".toast").boundingBox();
  if (card === null) throw new Error("Expected the toast up");
  // Paseo pauses the toast's dismissal while hovered (toast-host.tsx 175-188).
  await page.mouse.move(card.x + card.width / 2, card.y + card.height / 2);
  await page.clock.runFor(TOAST_LIFE_MS * 2);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.mouse.move(1, 1);
  await page.clock.runFor(TOAST_LIFE_MS - 1000);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.clock.runFor(1500);
  await expect(page.locator(".toast-host")).toBeHidden();
  await page.clock.resume();
});

test("a toast stays above a dialog opened while it is up", async ({ page }) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await pauseClock(page);
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  const toast = page
    .locator(".toast")
    .filter({ hasText: "Copied the session id" });
  await expect(toast).toBeVisible();
  // The success toast's 2.2 s are held while the Kill dialog opens over it.
  await page
    .getByRole("button", { name: "Session actions" })
    .click({ force: true });
  await page.clock.runFor(FOCUS_FRAME_MS);
  await page
    .getByRole("menuitem", { name: "Kill", exact: true })
    .click({ force: true });
  await page.clock.runFor(FOCUS_FRAME_MS * 2);
  await expect(page.getByRole("dialog", { name: /^Kill / })).toBeVisible();
  // The toast's middle is the toast, not the dialog's scrim over it.
  expect(await hitsItself(toast)).toBe(true);
  await page.clock.resume();
});
