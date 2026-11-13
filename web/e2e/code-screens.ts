// `npm run screens`: the code on the page (milestone 11, track B-page):
// the new-session page's places, the approval of a project's commands
// and a worktree's setup as it runs; a session's Changes and Files, a
// file open, the git menu and the commit dialog; the projects in the
// sidebar and in Settings; light, dark and at 390px wide. Each step only
// waits for what the picture needs; behaviour is code.spec.ts's.

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionPage } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

/** The session panel's tab named `name`. */
function tab(page: Page, name: string) {
  return page
    .getByRole("tablist", { name: "Session panel" })
    .getByRole("tab", { name });
}

/** On the new-session page: the project `name`, then where it runs. */
async function place(
  page: Page,
  name: string,
  where: RegExp | null,
): Promise<void> {
  await choose(
    page,
    page.getByRole("combobox", { name: "Project" }),
    new RegExp(`^${name}\\b`),
  );
  if (where !== null) {
    await choose(
      page,
      page.getByRole("combobox", { name: "Where it runs" }),
      where,
    );
  }
}

test("the code on the page", async ({ page }) => {
  test.setTimeout(300_000);
  await openUi(page);

  // Where it runs: the project and a new worktree from its default branch.
  const box = await newSessionPage(page, "laptop", "Structured");
  await place(page, "shop", /^New worktree/);
  await box
    .getByRole("textbox", { name: "Prompt" })
    .fill("Add a discount to the cart");
  await shot(page, "40-new-session-place");
  await page.getByRole("combobox", { name: "Where it runs" }).click();
  await shot(page, "41-new-session-modes");
  await page.keyboard.press("Escape");

  // Send: the commands are asked about first, then setup runs.
  await box.getByRole("button", { name: "Send" }).click();
  const approval = page.getByRole("dialog", {
    name: "Run this project's commands?",
  });
  await expect(approval).toBeVisible();
  await shot(page, "42-approve-commands");
  await page.emulateMedia({ colorScheme: "dark" });
  await shot(page, "42b-approve-commands-dark");
  await page.emulateMedia({ colorScheme: "light" });
  await approval.getByRole("button", { name: "Approve and run" }).click();
  const setup = page.getByRole("region", { name: "Worktree setup" });
  await expect(setup).toContainText("installing packages");
  await shot(page, "42c-setup-running");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);

  // The session in its worktree: the branch on its bar opens Changes.
  await page
    .getByRole("button", { name: /^Branch vessel\/\w+: show changes$/ })
    .click();
  await expect(tab(page, "Changes")).toHaveAttribute("aria-selected", "true");
  await shot(page, "43-session-worktree");

  // A session in the project's own folder, with work not committed.
  const again = await newSessionPage(page, "laptop", "Structured");
  await place(page, "shop", null);
  await again.getByRole("textbox", { name: "Prompt" }).fill("Look at the cart");
  await again.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  const folderSession = page.url();
  await tab(page, "Changes").click();
  const list = page.getByRole("region", { name: "Changed files" });
  await expect(list.locator(".d-line.add").first()).toBeVisible();
  await shot(page, "20-changes");

  await page.getByRole("button", { name: "Git action options" }).click();
  await shot(page, "21-git-menu");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Commit", exact: true }).click();
  await expect(
    page.getByRole("dialog", { name: "Commit changes" }),
  ).toBeVisible();
  await shot(page, "21b-commit-dialog");
  await page.keyboard.press("Escape");

  await tab(page, "Files").click();
  await page.getByRole("button", { name: "src" }).click();
  await page.getByRole("button", { name: "cart.ts" }).click();
  await expect(page.locator(".f-line").first()).toBeVisible();
  await shot(page, "22-file");
  await page.getByRole("button", { name: "Edit" }).click();
  await shot(page, "22b-file-edit");
  await page.getByRole("button", { name: "Cancel" }).click();

  await page.getByRole("button", { name: "All files" }).click();
  await page.getByRole("searchbox", { name: "Go to file" }).fill("pri");
  await expect(page.getByRole("list", { name: "Files found" })).toBeVisible();
  await shot(page, "23-find");

  await page.emulateMedia({ colorScheme: "dark" });
  await tab(page, "Changes").click();
  await expect(list.locator(".d-line.add").first()).toBeVisible();
  await shot(page, "24-changes-dark");

  // The projects in the sidebar, open; and Settings › Projects.
  await page
    .getByRole("region", { name: "Projects" })
    .getByRole("button", { name: /^shop\b/ })
    .click();
  await shot(page, "26-sidebar-projects-dark");
  await page.emulateMedia({ colorScheme: "light" });
  await shot(page, "26b-sidebar-projects");
  await page.getByRole("button", { name: "Add project" }).click();
  await expect(
    page.getByRole("heading", { name: /^Projects on / }),
  ).toBeVisible();
  await page
    .getByRole("list", { name: "Projects" })
    .getByRole("button", { name: "shop", exact: true })
    .click();
  await expect(page.getByRole("heading", { name: "Worktrees" })).toBeVisible();
  await shot(page, "27-settings-projects", { fullPage: true });
  await page.getByRole("button", { name: "Add project" }).last().click();
  await page.getByRole("menuitem", { name: "Clone a repository…" }).click();
  await shot(page, "28-clone-dialog");
  await page.keyboard.press("Escape");

  // A phone: the branch's chip opens the panel as a sheet.
  await page.goto(folderSession);
  await page.setViewportSize({ width: 390, height: 844 });
  await page
    .getByRole("button", { name: /^Project shop: show changes$/ })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Session panel" }),
  ).toBeVisible();
  await shot(page, "25-changes-phone");
  await page.emulateMedia({ colorScheme: "dark" });
  await shot(page, "25b-changes-phone-dark");
  await page.emulateMedia({ colorScheme: "light" });
});
