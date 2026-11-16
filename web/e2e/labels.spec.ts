// Workspace labels on the page (ADR 0086, "Page consumer"), against a real
// node: a session started in a project gives its root a workspace (ADR
// 0106); grouped by workspace, the row's Labels menu creates a label and
// assigns it, the chip shows it, the view menu's filter keeps or hides the
// workspace, and the manager (Paseo's: header search, host pill, pencil
// rows, header back) deletes the label. The view is put back at
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

  // The manager: the search sits in its header and is not focused on
  // open; one line for no match; the pencil opens the label, the header's
  // back arrow leaves it; deleting says how many workspaces carry it.
  await display(page);
  await page.getByRole("menuitem", { name: "Manage labels…" }).click();
  const manager = page.getByRole("dialog", { name: "Manage labels" });
  const search = manager.getByRole("searchbox", { name: "Search labels" });
  await expect(search).toBeVisible();
  await expect(search).not.toBeFocused();
  await manager.getByRole("button", { name: /^Filter: / }).click();
  await page.getByRole("menuitemradio", { name: /^laptop\b/ }).click();
  await search.fill("no such label");
  await expect(manager).toContainText("No labels on this host.");
  await search.fill("review");
  await manager.getByRole("button", { name: "Edit Needs review" }).click();
  const edit = page.getByRole("dialog", { name: "Edit label" });
  await expect(edit.getByRole("searchbox")).toHaveCount(0);
  await edit.getByRole("button", { name: "Back" }).click();
  await expect(search).toHaveValue("review");
  await manager.getByRole("button", { name: "Edit Needs review" }).click();
  await edit.getByRole("button", { name: "Delete", exact: true }).click();
  const ask = page.getByRole("dialog", { name: "Delete Needs review?" });
  await expect(ask).toContainText("1 workspace on this machine");
  await ask.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(ask).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(chip).toBeHidden();
});
