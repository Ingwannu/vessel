// The toast host stays clear while an actual agent question waits, shows
// one toast at a time (Paseo's ToastHost: a new one replaces it with its own
// life), and stays above a dialog opened while a toast is up (Paseo's planes).
import { expect, type Locator, type Page, test } from "@playwright/test";
import {
  clearToastBounds,
  copyToast,
  FOCUS_FRAME_MS,
  repeatedToasts,
} from "./feedback-kit";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";

const HOLD_AFTER_REPEAT_MS = 500;
const OK_LIFE_MS = 4000;

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
        await copyToast(page);
        const toast = await clearToastBounds(page);
        await toast.getByRole("button", { name: "Dismiss" }).click();
        await expect(page.locator(".toast-host")).toBeHidden();
        await repeatedToasts(page);
        expect(errors()).toBe(0);
      });
    }
    test("a short phone keeps question controls clickable through resize and folding", async ({
      page,
    }) => {
      const origin = await openUi(page);
      await page.context().grantPermissions(["clipboard-write"], { origin });
      await startSession(page, "laptop", "Structured", "ask claude");
      await repeatedToasts(page);
      await page.clock.pauseAt(
        (await page.evaluate(() => Date.now())) + HOLD_AFTER_REPEAT_MS,
      );
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

test("a new toast replaces the one up and runs its own 4 s", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 1000));
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.clock.runFor(OK_LIFE_MS - 1000);
  await copyToast(page, "paused");
  // Past the first toast's 4 s: the second is up, alone.
  await page.clock.runFor(2000);
  await expect(page.locator(".toast")).toHaveCount(1);
  // Past the second's own 4 s.
  await page.clock.runFor(OK_LIFE_MS - 2000 + 500);
  await expect(page.locator(".toast-host")).toBeHidden();
  await page.clock.resume();
});

test("a pointer resting on the toast holds its life", async ({ page }) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await page.clock.install();
  await page.clock.pauseAt(new Date(Date.now() + 1000));
  await copyToast(page, "paused");
  await page.clock.runFor(FOCUS_FRAME_MS);
  const card = await page.locator(".toast").boundingBox();
  if (card === null) throw new Error("Expected the toast up");
  // Paseo pauses the toast's dismissal while hovered (toast-host.tsx 175-188).
  await page.mouse.move(card.x + card.width / 3, card.y + card.height / 2);
  await page.clock.runFor(OK_LIFE_MS * 2);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.mouse.move(1, 1);
  await page.clock.runFor(OK_LIFE_MS - 1000);
  await expect(page.locator(".toast")).toHaveCount(1);
  await page.clock.runFor(1500);
  await expect(page.locator(".toast-host")).toBeHidden();
  await page.clock.resume();
});

test("a toast stays above a dialog opened while it is up", async ({ page }) => {
  const origin = await openUi(page);
  await page.context().grantPermissions(["clipboard-write"], { origin });
  await startSession(page, "laptop", "Structured", "ask claude");
  await copyToast(page);
  const toast = page
    .locator(".toast")
    .filter({ hasText: "Copied the session id" });
  await expect(toast).toBeVisible();
  // Hold the success toast's 4 s while the Kill dialog opens over it.
  await page.clock.pauseAt(
    (await page.evaluate(() => Date.now())) + HOLD_AFTER_REPEAT_MS,
  );
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
