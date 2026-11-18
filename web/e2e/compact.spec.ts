// The compact breakpoint, as Paseo's (TRACKER 805): a window below 600px
// is compact (src/ui/breakpoints.ts), the sidebar a sheet from the title
// bar's menu; at 600px and wider the sidebar stays beside the page (a
// rail of icons up to 1099px). The stylesheets' `@media (--compact)` and
// the TS side's query are one constant: each width checks both agree.

import { expect, type Page, test } from "@playwright/test";

import { COMPACT } from "../src/ui/breakpoints";
import { openUi } from "./open";

function compactNow(page: Page): Promise<boolean> {
  return page.evaluate((query) => window.matchMedia(query).matches, COMPACT);
}

for (const width of [599, 600, 700]) {
  const compact = width < 600;
  test(`at ${String(width)}px the page is ${compact ? "compact" : "not compact"}, as Paseo's`, async ({
    page,
  }) => {
    // The page opens at the configured width (its sidebar is the sign it
    // is up), then narrows.
    await openUi(page);
    await page.setViewportSize({ width, height: 800 });
    expect(await compactNow(page)).toBe(compact);
    const menu = page.getByRole("button", { name: "Open sidebar" });
    const beside = page.getByRole("navigation", { name: "Sidebar" });
    if (compact) {
      await expect(menu).toBeVisible();
      await expect(beside).toBeHidden();
      await menu.click();
      await expect(page.getByRole("dialog", { name: "Sidebar" })).toBeVisible();
    } else {
      await expect(menu).toBeHidden();
      await expect(beside).toBeVisible();
    }
  });
}
