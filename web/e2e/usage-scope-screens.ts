// The new scope choice, including full long names, with actual node totals.
// Desktop bootstrap followed by resize; this is not cold-mobile evidence.
import { expect, test } from "@playwright/test";
import { choose } from "./kit";
import { openUi } from "./open";
import { watchPageErrors } from "./page-errors";
import { shot } from "./shot";
import { LONG_USAGE_PROFILE, seedNamedUsage } from "./usage-scope-seed";

for (const locale of ["en-US", "ko-KR"]) {
  for (const scheme of ["light", "dark"] as const) {
    for (const width of [1280, 390]) {
      test(`usage scope ${locale} ${scheme} ${String(width)}`, async ({
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
          await seedNamedUsage();
          await page.goto(`${origin}/settings/usage?machine=laptop`);
          const scope = page.getByRole("combobox", {
            name: "Usage agent or profile",
          });
          await expect(scope).toBeVisible();
          await page.setViewportSize({
            width,
            height: width === 390 ? 844 : 900,
          });
          await scope.click();
          const option = page.getByRole("option", {
            name: `Claude Code · Profile: ${LONG_USAGE_PROFILE}`,
            exact: true,
          });
          await option.scrollIntoViewIfNeeded();
          await expect(option).toBeVisible();
          await expect
            .poll(() =>
              option
                .locator(".label")
                .evaluate(
                  (label) =>
                    label.scrollWidth <= label.clientWidth &&
                    label.scrollHeight <= label.clientHeight,
                ),
            )
            .toBe(true);
          const name = `usage-scope-${locale}-${scheme}-${String(width)}`;
          await shot(page, `${name}-choices`);
          await page.keyboard.press("Escape");
          await expect(page.getByRole("listbox")).toBeHidden();
          await choose(
            page,
            scope,
            new RegExp(`^Claude Code · Profile: ${LONG_USAGE_PROFILE}$`),
          );
          await expect(
            page.getByRole("button", { name: "Refresh", exact: true }),
          ).toBeEnabled();
          await expect(scope).toContainText(LONG_USAGE_PROFILE);
          await expect
            .poll(() =>
              scope
                .locator(".label")
                .evaluate(
                  (label) =>
                    label.scrollWidth <= label.clientWidth &&
                    label.scrollHeight <= label.clientHeight,
                ),
            )
            .toBe(true);
          expect(
            await page.evaluate(
              () => document.documentElement.scrollWidth > window.innerWidth,
            ),
          ).toBe(false);
          await shot(page, `${name}-selected`);
          await page
            .getByRole("list", { name: "Accounting coverage" })
            .scrollIntoViewIfNeeded();
          await shot(page, `${name}-coverage`);
          expect(errors()).toBe(0);
        } finally {
          await context.close();
        }
      });
    }
  }
}
