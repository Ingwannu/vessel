// The compact breakpoint, as Paseo's (TRACKER 805): a window below 600px
// is compact (src/ui/breakpoints.ts), the sidebar a sheet from the title
// bar's menu; at 600px and wider the whole sidebar stays beside the page,
// as Paseo's (TRACKER 806, desktop-sidebar-layout.ts): 240 wide, narrowed
// to 200 at least so the page keeps 400, never a rail of icons. The
// stylesheets' `@media (--compact)` and the TS side's query are one
// constant: each width checks both agree.

import { expect, type Page, test } from "@playwright/test";

import { COMPACT } from "../src/ui/breakpoints";
import { openUi } from "./open";

function compactNow(page: Page): Promise<boolean> {
  return page.evaluate((query) => window.matchMedia(query).matches, COMPACT);
}

/** Paseo's sidebar width beside the page: its 240, at least 200, at most
 * the window less a 400 centre. */
function sidebarWidth(window: number): number {
  return Math.max(200, Math.min(window - 400, 240));
}

for (const width of [599, 600, 620, 700, 1024]) {
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
      // The whole sidebar, its words and all, at Paseo's width.
      await expect(
        beside.getByText("New session", { exact: true }),
      ).toBeVisible();
      await expect(async () => {
        const box = await beside.boundingBox();
        expect(box?.width).toBeCloseTo(sidebarWidth(width), 0);
      }).toPass();
    }
  });
}
