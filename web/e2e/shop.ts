// A session in the fleet's `shop` repository (e2e/repos.ts) and its code
// panel's tabs, shared by the specs that read the panel.

import { type Locator, type Page } from "@playwright/test";

import { choose, newSessionPage } from "./kit";

/** The session panel's tab named `name`. */
export function panelTab(page: Page, name: string): Locator {
  return page
    .getByRole("tablist", { name: "Session panel" })
    .getByRole("tab", { name });
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
