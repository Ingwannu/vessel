// `npm run screens`: the code on the page (milestone 11, track B-page):
// a session's Changes and Files, a file open, the git menu and the commit
// dialog, the projects in the sidebar and Settings, the new-session
// page's places and a worktree's setup, light, dark and at 390px wide.
// Each step only waits for what the picture needs; behaviour is
// code.spec.ts's.

import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { newSessionPage } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

/** Start a structured session on laptop in `folder`, and open it. */
async function sessionIn(page: Page, folder: string): Promise<void> {
  const box = await newSessionPage(page, "laptop", "Structured");
  await page.getByRole("combobox", { name: "Folder" }).fill(folder);
  await page.keyboard.press("Escape");
  await box.getByRole("textbox", { name: "Prompt" }).fill("Look at the cart");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
}

/** The session panel's tab named `name`. */
function tab(page: Page, name: string) {
  return page
    .getByRole("tablist", { name: "Session panel" })
    .getByRole("tab", { name });
}

test("the code on the page", async ({ page }) => {
  test.setTimeout(240_000);
  const env = loadEnv();
  await openUi(page);
  const shop = join(env.root, "repos", "shop");
  await sessionIn(page, shop);

  await tab(page, "Changes").click();
  const list = page.getByRole("region", { name: "Changed files" });
  await expect(list.getByText("cart.ts")).toBeVisible();
  await expect(list.locator(".d-line.add").first()).toBeVisible();
  await shot(page, "20-changes");

  await page.getByRole("button", { name: "Git action options" }).click();
  await shot(page, "21-git-menu");
  await page.keyboard.press("Escape");

  await tab(page, "Files").click();
  await expect(page.getByRole("region", { name: "Files" })).toBeVisible();
  await page.getByRole("button", { name: "src" }).click();
  await page.getByRole("button", { name: "cart.ts" }).click();
  await expect(page.locator(".f-line").first()).toBeVisible();
  await shot(page, "22-file");

  await page.getByRole("button", { name: "All files" }).click();
  await page.getByRole("searchbox", { name: "Go to file" }).fill("pri");
  await expect(page.getByRole("list", { name: "Files found" })).toBeVisible();
  await shot(page, "23-find");

  await page.emulateMedia({ colorScheme: "dark" });
  await tab(page, "Changes").click();
  await expect(list.locator(".d-line.add").first()).toBeVisible();
  await shot(page, "24-changes-dark");
  await page.emulateMedia({ colorScheme: "light" });

  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .getByRole("button", { name: /^Project shop: show changes$/ })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Session panel" }),
  ).toBeVisible();
  await shot(page, "25-changes-phone");
});
