// Workspace labels on the page (ADR 0086, "Page consumer"), against a real
// node: a session started in a project gives its root a workspace (ADR
// 0106); grouped by workspace, the row's Labels menu creates a label and
// assigns it, the chip shows it, the view menu's filter keeps or hides the
// workspace, and the manager deletes the label. The view is put back at
// the end: the other specs read the sidebar grouped by machine.

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionPage } from "./kit";
import { openUi } from "./open";

async function startInShop(page: Page, prompt: string): Promise<void> {
  const box = await newSessionPage(page, "laptop", "Structured");
  await choose(
    page,
    page.getByRole("combobox", { name: "Project" }),
    /^shop\b/,
  );
  await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  await box.getByRole("button", { name: "Send" }).click();
}

function sidebar(page: Page) {
  return page.getByRole("navigation", { name: "Sidebar" });
}

/** Close every open menu: Escape closes one level at a time (a submenu,
 * then the menu that opened it), and a radio row has already closed its
 * menu. Each press waits for the focus to be in a menu, as kit.ts's
 * `closeMenu` explains. */
async function closeMenus(page: Page): Promise<void> {
  for (let level = 0; level < 3; level += 1) {
    if ((await page.getByRole("menu").count()) === 0) {
      return;
    }
    await expect
      .poll(() =>
        page.evaluate(
          () => document.activeElement?.closest('[role="menu"]') != null,
        ),
      )
      .toBe(true);
    await page.keyboard.press("Escape");
  }
  await expect(page.getByRole("menu")).toHaveCount(0);
}

async function display(page: Page): Promise<void> {
  await sidebar(page).getByRole("button", { name: "Display" }).click();
}

async function group(page: Page, by: "Machine" | "Workspace"): Promise<void> {
  await display(page);
  await page.getByRole("menuitemradio", { name: by }).click();
  await closeMenus(page);
}

async function filterBy(page: Page, label: string): Promise<void> {
  await display(page);
  await page.getByRole("menuitem", { name: /^Labels/ }).hover();
  await page.getByRole("menuitemcheckbox", { name: label }).click();
  await closeMenus(page);
}

test.afterEach(async ({ page }) => {
  await display(page);
  const clear = page.getByRole("menuitem", { name: /^Labels/ });
  if (await clear.isVisible()) {
    await clear.hover();
    const item = page.getByRole("menuitem", { name: "Clear filter" });
    const shown = await item.waitFor({ state: "visible", timeout: 3000 }).then(
      () => true,
      () => false,
    );
    if (shown) {
      await item.click();
      await display(page);
    }
  }
  await page.getByRole("menuitemradio", { name: "Machine" }).click();
  await closeMenus(page);
});

test("a label made on a workspace row shows as its chip, filters it, and goes when deleted", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await openUi(page);
  await startInShop(page, "plain");
  await group(page, "Workspace");
  const shop = sidebar(page)
    .locator(".side-workspace")
    .filter({ hasText: "shop" })
    .first();
  await expect(shop).toBeVisible();

  // The row's Labels menu creates the label on laptop and puts it on shop.
  await shop.hover();
  await shop.getByRole("button", { name: "Workspace actions" }).click();
  await page.getByRole("menuitem", { name: "Labels" }).hover();
  await page.getByRole("menuitem", { name: "Create label…" }).click();
  const create = page.getByRole("dialog", { name: "Create label" });
  await create.getByRole("textbox").fill("  Needs   review ");
  await create.getByRole("radio", { name: "Teal" }).click();
  await create.getByRole("button", { name: "Create", exact: true }).click();
  await expect(create).toBeHidden();
  const chip = shop.locator(".label-chip", { hasText: "Needs review" });
  await expect(chip).toBeVisible();
  await expect(chip).toHaveAttribute("data-label-color", "teal");

  // Unlabelled hides it; its own label keeps it.
  await filterBy(page, "Unlabelled");
  await expect(shop).toBeHidden();
  await filterBy(page, "Unlabelled");
  await filterBy(page, "Needs review");
  await expect(shop).toBeVisible();

  // Deleting it in the manager says how many workspaces carry it first.
  await display(page);
  await page.getByRole("menuitem", { name: "Manage labels…" }).click();
  const manager = page.getByRole("dialog", { name: "Manage labels" });
  await manager.getByRole("button", { name: "Needs review" }).click();
  await page
    .getByRole("dialog", { name: "Edit label" })
    .getByRole("button", { name: "Delete" })
    .click();
  const ask = page.getByRole("dialog", { name: "Delete Needs review?" });
  await expect(ask).toContainText("1 workspace on this machine");
  await ask.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(ask).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(chip).toBeHidden();
});
