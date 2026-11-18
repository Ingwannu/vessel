// One affected-state matrix, with a real question and UI-generated feedback.
// Desktop authentication then resize, not a cold-mobile bootstrap claim.
import { expect, test } from "@playwright/test";
import { clearToastBounds, copyToast, repeatedToasts } from "./feedback-kit";
import { startSession } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";
import { shot } from "./shot";

const SIZES = [
  { width: 1280, height: 900 },
  { width: 390, height: 844 },
  { width: 390, height: 600 },
  { width: 800, height: 600 },
] as const;

for (const locale of ["en-US", "ko-KR"]) {
  for (const scheme of ["light", "dark"] as const) {
    for (const size of SIZES) {
      test(`feedback ${locale} ${scheme} ${String(size.width)}x${String(size.height)}`, async ({
        browser,
      }) => {
        const context = await browser.newContext({
          locale,
          colorScheme: scheme,
          viewport: { width: 1280, height: 900 },
        });
        const page = await context.newPage();
        const errors = watchPageErrors(page);
        try {
          const origin = await openUi(page);
          await context.grantPermissions(["clipboard-write"], { origin });
          await startSession(page, "laptop", "Structured", "ask claude");
          await expect(
            page.getByRole("region", { name: "Question from the agent" }),
          ).toBeVisible();
          await page.setViewportSize(size);
          await copyToast(page);
          const toast = await clearToastBounds(page);
          await page.mouse.move(0, 0);
          const name = `feedback-${locale}-${scheme}-${String(size.width)}x${String(size.height)}`;
          await shot(page, `${name}-toast`);
          await toast.getByRole("button", { name: "Dismiss" }).click();
          await repeatedToasts(page);
          await page.mouse.move(0, 0);
          await shot(page, `${name}-repeated`);
          expect(
            await page.evaluate(
              () => document.documentElement.scrollWidth > window.innerWidth,
            ),
          ).toBe(false);
          expect(errors()).toBe(0);
        } finally {
          await context.close();
        }
      });
    }
  }
}
