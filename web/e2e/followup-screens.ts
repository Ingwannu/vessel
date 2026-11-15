// Content-only evidence; shared shot masks the own machine name.
import { expect, test } from "@playwright/test";

import { startSession } from "./kit";
import { shot } from "./shot";
import { openEndedSession } from "./sidebar-defaults-flow";
import { expectWaitingAboveDock, PHONE_VIEWPORT } from "./window-layout-flow";

const DESKTOP_VIEWPORT = { width: 1280, height: 900 };
const VIEWPORTS = [
  DESKTOP_VIEWPORT,
  PHONE_VIEWPORT,
  { width: 800, height: 600 },
  { width: 1024, height: 700 },
];

for (const theme of ["light", "dark"] as const) {
  test(`ended sidebar and question dock ${theme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: theme });
    await page.setViewportSize(DESKTOP_VIEWPORT);
    await openEndedSession(page);
    await shot(page, `followup-ended-${theme}`);
    await startSession(page, "laptop", "Structured", "ask claude");
    await expect(
      page.getByRole("region", { name: "Question from the agent" }),
    ).toBeVisible();
    for (const viewport of VIEWPORTS) {
      await page.setViewportSize(viewport);
      await expectWaitingAboveDock(page);
      await shot(
        page,
        `followup-dock-${theme}-${String(viewport.width)}x${String(viewport.height)}`,
      );
    }
  });
}
