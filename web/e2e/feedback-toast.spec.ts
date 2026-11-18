// The existing toast host stays clear while an actual agent question waits,
// and stays above a dialog opened while a toast is up (Paseo's planes).
import { expect, test } from "@playwright/test";
import {
  clearToastBounds,
  copyToast,
  FOCUS_FRAME_MS,
  STACK_FIXTURE_COUNT,
  stackedToasts,
} from "./feedback-kit";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";

const HOLD_AFTER_STACK_MS = 500;

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
        await expect(page.locator(".toast-stack")).toBeHidden();
        await stackedToasts(page);
        expect(errors()).toBe(0);
      });
    }
    test("a short phone keeps question controls clickable through resize and folding", async ({
      page,
    }) => {
      const origin = await openUi(page);
      await page.context().grantPermissions(["clipboard-write"], { origin });
      await startSession(page, "laptop", "Structured", "ask claude");
      await stackedToasts(page);
      await page.clock.pauseAt(
        (await page.evaluate(() => Date.now())) + HOLD_AFTER_STACK_MS,
      );
      await page.setViewportSize({ width: 390, height: 600 });
      // Clear of the dock in the very layout the viewport change makes,
      // with no frame for a measurement to catch up in.
      expect(
        await page.evaluate(() => {
          const stack = document.querySelector(".toast-stack");
          const dock = document.querySelector(".prompt-dock");
          if (stack === null || dock === null) return false;
          return (
            stack.getBoundingClientRect().bottom <=
            dock.getBoundingClientRect().top + 0.5
          );
        }),
      ).toBe(true);
      await page.clock.runFor(FOCUS_FRAME_MS);
      const stack = page.locator(".toast-stack");
      await expect(page.locator(".toast")).toHaveCount(STACK_FIXTURE_COUNT);
      const compact = await stack.boundingBox();
      if (compact === null) throw new Error("Expected the live toast stack");
      const collapse = page.getByRole("button", { name: "Collapse question" });
      expect(
        await collapse.evaluate((element) => {
          const rect = element.getBoundingClientRect();
          const hit = document.elementFromPoint(
            rect.x + rect.width / 2,
            rect.y + rect.height / 2,
          );
          return hit !== null && element.contains(hit);
        }),
      ).toBe(true);
      // The frozen clock holds focus frames; this still sends an actual click.
      await collapse.click({ force: true });
      await page.clock.runFor(FOCUS_FRAME_MS);
      const expand = page.getByRole("button", { name: "Expand question" });
      await expect(expand).toBeVisible();
      await expect
        .poll(async () => (await stack.boundingBox())?.height ?? 0)
        .toBeGreaterThan(compact.height);
      await expand.click({ force: true });
      await page.clock.runFor(FOCUS_FRAME_MS);
      await expect(collapse).toBeVisible();
      await expect
        .poll(async () => (await stack.boundingBox())?.height ?? 0)
        .toBeCloseTo(compact.height, 0);
      await page.clock.resume();
    });
  });
}

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
    (await page.evaluate(() => Date.now())) + HOLD_AFTER_STACK_MS,
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
  expect(
    await toast.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const hit = document.elementFromPoint(
        rect.x + rect.width / 2,
        rect.y + rect.height / 2,
      );
      return hit !== null && element.contains(hit);
    }),
  ).toBe(true);
  await page.clock.resume();
});
