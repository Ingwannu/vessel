// Driving the kit's controls (src/ui/kit) as a person does: a select is a
// button that opens a list of options, a segmented control is a group of
// radios. Shared by the specs and the screens.

import { expect, type Locator, type Page } from "@playwright/test";

/** Open the select named `field` and choose the option whose name
 * matches `option` (the label, then any detail after it). */
export async function choose(
  page: Page,
  field: Locator,
  option: RegExp,
): Promise<void> {
  await field.click();
  await page.getByRole("option", { name: option }).click();
  await expect(page.getByRole("listbox")).toBeHidden();
}

/** The option `option` in the open list of `field`, after opening it. */
export async function optionOf(
  page: Page,
  field: Locator,
  option: RegExp,
): Promise<Locator> {
  await field.click();
  return page.getByRole("option", { name: option });
}

/** The new-session page, opened from the title bar's New, on `node`
 * with Claude Code; opened as a terminal for `"Terminal"`. Nothing is
 * sent yet: the prompt box is returned, its text empty. */
export async function newSessionPage(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
): Promise<Locator> {
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "What do you want to work on?" }),
  ).toBeVisible();
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    new RegExp(`^${node}\\b`),
  );
  await chooseAgent(page, "Claude Code");
  if (mode === "Terminal") {
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("switch", { name: "Open as terminal" }).click();
    await page.keyboard.press("Escape");
  }
  return page.getByRole("form", { name: "Prompt" });
}

/** A session's timeline: its entries, apart from the live region that
 * repeats the newest one's text for screen readers. */
export function timeline(page: Page): Locator {
  return page.getByRole("region", { name: "Timeline" });
}

/** Choose `agent` in the agent, model and effort menu. */
export async function chooseAgent(page: Page, agent: string): Promise<void> {
  await page.getByRole("button", { name: /^Agent, model and effort/ }).click();
  await page.getByRole("menuitem", { name: /^Agent/ }).click();
  await page.getByRole("menuitemradio", { name: agent }).click();
  await expect(page.getByRole("menu")).toHaveCount(0);
}

/** Start a session from the new-session page with `prompt`, and wait for
 * its page. */
export async function startSession(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
  prompt = "",
): Promise<void> {
  const box = await newSessionPage(page, node, mode);
  if (prompt !== "") {
    await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  }
  await box
    .getByRole("button", {
      name: mode === "Terminal" ? "Open the terminal" : "Send",
    })
    .click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
}
