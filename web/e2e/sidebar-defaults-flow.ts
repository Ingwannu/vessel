// Shared synthetic flow for the default-on regression and its captures;
// preference acknowledgements distinguish paint from persisted choice.
import { expect, type Locator, type Page } from "@playwright/test";

import { closeMenu, startSession, timeline } from "./kit";
import { acknowledgePreference, openUi } from "./open";

export async function openEndedSession(page: Page): Promise<Locator> {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(
    timeline(page).getByText("echo: plain", { exact: true }),
  ).toBeVisible();
  const id = new URL(page.url()).pathname.split("/").at(-1);
  if (id === undefined || id === "")
    throw new Error("Expected a session route");
  const row = page
    .getByRole("navigation", { name: "Sidebar" })
    .locator(`a[href$="/${id}"]`);
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: "Kill", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Kill", exact: true })
    .click();
  await expect(
    page.getByRole("region", { name: "Session ended" }),
  ).toBeVisible();
  await expect(row).toBeVisible();
  return row;
}

export async function showEnded(page: Page, value: boolean): Promise<void> {
  await page.getByRole("button", { name: "Display", exact: true }).click();
  const toggle = page.getByRole("menuitemcheckbox", {
    name: "Show ended sessions",
  });
  await expect(toggle).toHaveAttribute("aria-checked", String(!value));
  await acknowledgePreference(
    page,
    () => toggle.click(),
    (values) => values.sidebar.ended === value,
  );
  await closeMenu(page);
}
