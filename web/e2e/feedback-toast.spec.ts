// Ordinary toast geometry while the agent's actual question remains open.
// Copying its synthetic id emits the existing toast without ending the turn.
import { expect, type Locator, test } from "@playwright/test";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";

async function rectangle(element: Locator) {
  const value = await element.boundingBox();
  if (value === null) throw new Error("Expected a visible feedback boundary");
  return value;
}

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(colorScheme, () => {
    test.use({ colorScheme });
    test("a phone toast clears the composer and question buttons below the header", async ({
      page,
    }) => {
      const errors = watchPageErrors(page);
      const origin = await openUi(page);
      await page.context().grantPermissions(["clipboard-write"], { origin });
      await startSession(page, "laptop", "Structured", "ask claude");
      const question = page.getByRole("region", {
        name: "Question from the agent",
      });
      await expect(question).toBeVisible();
      await page.setViewportSize({ width: 390, height: 844 });
      await page.getByRole("button", { name: "Session actions" }).click();
      await page.getByRole("menuitem", { name: "Copy session id" }).click();
      const toast = page
        .locator(".toast")
        .filter({ hasText: "Copied the session id" });
      await expect(toast).toBeVisible();
      const notice = await rectangle(toast);
      const header = await rectangle(page.getByRole("banner"));
      const prompt = await rectangle(
        page.getByRole("form", { name: "Prompt" }),
      );
      const next = await rectangle(
        question.getByRole("button", { name: "Next", exact: true }),
      );
      expect(notice.y).toBeGreaterThanOrEqual(header.y + header.height);
      expect(notice.y + notice.height).toBeLessThanOrEqual(prompt.y);
      expect(notice.y + notice.height).toBeLessThanOrEqual(next.y);
      expect(notice.x).toBeGreaterThanOrEqual(0);
      expect(notice.x + notice.width).toBeLessThanOrEqual(390);
      await toast.getByRole("button", { name: "Dismiss" }).click();
      expect(errors()).toBe(0);
    });
  });
}
