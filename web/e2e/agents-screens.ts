// `npm run screens`: agents added on a machine by their command (protocol
// 11, ADR 0032) where the page shows them: the new-session page's agent
// menu with laptop's added agents after the four, an added agent's start
// refused for a secret the machine lacks, a session of one with its label
// on the bar, and Settings › Agents listing them, found or not; light,
// dark and at 390px. Behaviour is agents.spec.ts's.

import { expect, type Page, test } from "@playwright/test";

import { choose, chooseAgent, openNew, timeline } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

/** The new-session page on laptop. */
async function newOnLaptop(page: Page): Promise<void> {
  await openNew(page);
  await expect(
    page.getByRole("heading", { name: "What do you want to work on?" }),
  ).toBeVisible();
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    /^laptop\b/,
  );
}

test("agents added on a machine", async ({ page }) => {
  test.setTimeout(180_000);
  const origin = await openUi(page);

  // The agent menu: the four laptop found, then its added agents.
  await newOnLaptop(page);
  await page.getByRole("button", { name: /^Agent, model and effort/ }).click();
  await page.getByRole("menuitem", { name: /^Agent/ }).click();
  await expect(
    page.getByRole("menuitemradio", { name: /Helper/ }),
  ).toBeVisible();
  await shot(page, "60-agents-menu");
  await page.emulateMedia({ colorScheme: "dark" });
  await shot(page, "60b-agents-menu-dark");
  await page.emulateMedia({ colorScheme: "light" });
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");

  // One whose secret laptop lacks: said before Send, which stays off.
  await chooseAgent(page, "Keyed agent");
  await page
    .getByRole("form", { name: "Prompt" })
    .getByRole("textbox", { name: "Prompt" })
    .fill("Summarise the open issues");
  await expect(page.getByText(/^Keyed agent cannot start/)).toBeVisible();
  await shot(page, "61-agent-refused");

  // A session of an added agent: its label on the bar.
  await chooseAgent(page, "Helper");
  const box = page.getByRole("form", { name: "Prompt" });
  await box.getByRole("textbox", { name: "Prompt" }).fill("Tidy the readme");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(timeline(page).getByText("echo: Tidy the readme")).toBeVisible();
  await shot(page, "62-added-agent-session");

  // Settings › Agents on laptop, with its added agents.
  await page.goto(
    `${origin}/settings/agents?machine=laptop&agent=added:helper`,
  );
  await expect(
    page.getByRole("heading", { name: "Helper on laptop" }),
  ).toBeVisible();
  await shot(page, "63-settings-added-agents", { fullPage: true });
  await page.emulateMedia({ colorScheme: "dark" });
  await shot(page, "63b-settings-added-agents-dark", { fullPage: true });
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize({ width: 390, height: 844 });
  await shot(page, "63c-settings-added-agents-phone");
  await page.setViewportSize({ width: 1440, height: 900 });
});
