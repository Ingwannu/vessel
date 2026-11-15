// The existing toast host stays clear while an actual agent question waits.
import { expect, test } from "@playwright/test";
import { clearToastBounds, copyToast, stackedToasts } from "./feedback-kit";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";

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
  });
}
