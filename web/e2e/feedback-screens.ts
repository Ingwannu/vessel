// One affected-state matrix, with a real question and UI-generated feedback.
// Desktop authentication then resize, not a cold-mobile bootstrap claim.
import { expect, test } from "@playwright/test";
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
          // Paused clock: the toast's 2.2 s would not outlast a loaded run.
          await pauseClock(page);
          await copyToast(page, "paused");
          await page.clock.runFor(FOCUS_FRAME_MS);
          await clearToastBounds(page);
          await page.mouse.move(0, 0);
          const name = `feedback-${locale}-${scheme}-${String(size.width)}x${String(size.height)}`;
          await shot(page, `${name}-toast`);
          await repeatedToasts(page);
          await page.mouse.move(0, 0);
          await shot(page, `${name}-repeated`);
          await page.clock.resume();
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
