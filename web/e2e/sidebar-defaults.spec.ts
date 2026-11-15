// The default and explicit off use the real bridge file, not local-only state.
import { expect, test } from "@playwright/test";

import { closeMenu } from "./kit";
import { acknowledgePreference } from "./open";
import { openEndedSession, showEnded } from "./sidebar-defaults-flow";

test("ended sessions are visible before a choice; saved off survives reload and archived stays out", async ({
  page,
}) => {
  const row = await openEndedSession(page);
  try {
    await showEnded(page, false);
    await expect(row).toBeHidden();
    await page.reload();
    await expect(
      page.getByRole("region", { name: "Session ended" }),
    ).toBeVisible();
    await expect(row).toBeHidden();
    await showEnded(page, true);
    await expect(row).toBeVisible();
    await page.getByRole("button", { name: "Session actions" }).click();
    await page
      .getByRole("menuitem", { name: "Archive session", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Confirm", exact: true })
      .click();
    await expect(row).toBeHidden();
  } finally {
    await page.getByRole("button", { name: "Display", exact: true }).click();
    const toggle = page.getByRole("menuitemcheckbox", {
      name: "Show ended sessions",
    });
    if ((await toggle.getAttribute("aria-checked")) !== "true") {
      await acknowledgePreference(
        page,
        () => toggle.click(),
        (values) => values.sidebar.ended,
      );
    }
    await closeMenu(page);
  }
});
