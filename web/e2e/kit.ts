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

/** A new session on `node` with Claude Code, in `mode`, from the dialog
 * the topbar's New opens; the dialog is returned still open. */
export async function newSessionDialog(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
): Promise<Locator> {
  await page.getByRole("button", { name: "New", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "New session" });
  await choose(page, dialog.getByLabel("Node"), new RegExp(`^${node}\\b`));
  await choose(page, dialog.getByLabel("Harness"), /^claude/);
  await dialog.getByRole("radio", { name: mode }).click();
  return dialog;
}
