// A session in the fleet's `shop` repository (e2e/repos.ts) and its code
// panel's tabs, shared by the specs that read the panel.

import { expect, type Locator, type Page } from "@playwright/test";

import { choose, newSessionPage } from "./kit";

/** The session panel's tab named `name`. */
export function panelTab(page: Page, name: string): Locator {
  return page
    .getByRole("tablist", { name: "Workspace tabs" })
    .getByRole("tab", { name, exact: true });
}

/** Open an existing workspace tab or add it from the launcher. */
export async function openPanelTab(page: Page, name: string): Promise<void> {
  const tab = panelTab(page, name);
  if ((await tab.count()) > 0) {
    await tab.click();
  } else {
    await page.getByRole("button", { name: "New tab", exact: true }).click();
    await page.getByRole("menuitem", { name, exact: true }).click();
  }
}

/** The active CodeMirror lines expose the short fixture's draft in order. */
export async function expectEditorText(
  editor: Locator,
  text: string,
): Promise<void> {
  await expect
    .poll(async () =>
      (await editor.locator(".cm-line").allTextContents()).join("\n"),
    )
    .toBe(text);
}

/** Start a session on laptop in the project `shop`, `where` it runs. */
export async function startInShop(
  page: Page,
  where: "Local" | "New worktree",
  prompt: string,
): Promise<void> {
  const box = await newSessionPage(page, "laptop", "Structured");
  await choose(
    page,
    page.getByRole("combobox", { name: "Project" }),
    /^shop\b/,
  );
  if (where === "New worktree") {
    await choose(
      page,
      page.getByRole("combobox", { name: "Where it runs" }),
      /^New worktree/,
    );
  }
  await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await box.getByRole("button", { name: "Send" }).click();
}
