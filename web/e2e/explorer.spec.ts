// The Files tab's file action menu (Paseo's file-actions-menu.tsx on its
// explorer rows), against the fleet's `shop` repository: a row's right
// click lists Paseo's rows in Paseo's order for a file, an open folder
// and a closed one; the copies put the paths on the clipboard, Download
// saves the file's exact bytes, Add to chat names the file in the
// session's draft, Collapse closes the folder.

import { readFileSync, realpathSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { openUi } from "./open";
import { panelTab, startInShop } from "./shop";

/** The menu a right click on `row` opens, and its rows' words. */
async function menuOf(page: Page, row: ReturnType<Page["getByRole"]>) {
  await row.click({ button: "right" });
  const menu = page.getByRole("menu");
  await expect(menu).toBeVisible();
  return menu;
}

function clipboard(page: Page): Promise<string> {
  return page.evaluate(() => navigator.clipboard.readText());
}

test("a tree row's right click offers Paseo's file actions, and each does its work", async ({
  page,
}) => {
  const env = loadEnv();
  const shop = realpathSync(join(env.root, "repos", "shop"));
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  await openUi(page);
  await startInShop(page, "Local", "Look at the files");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await panelTab(page, "Files").click();
  const tree = page.getByRole("region", { name: "Files" });
  const src = tree.getByRole("button", { name: "src", exact: true });
  await src.click();
  const cart = tree.getByRole("button", { name: "cart.ts", exact: true });
  await expect(cart).toBeVisible();

  // A file: the reference group (Paseo's Open to the side, Open in editor
  // and Reveal need a second pane or a desktop).
  let menu = await menuOf(page, cart);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Copy path",
    "Copy relative path",
    "Download",
    "Add to chat",
  ]);
  await menu.getByRole("menuitem", { name: "Copy relative path" }).click();
  await expect(menu).toHaveCount(0);
  await expect.poll(() => clipboard(page)).toBe("src/cart.ts");

  menu = await menuOf(page, cart);
  await menu.getByRole("menuitem", { name: "Copy path", exact: true }).click();
  await expect.poll(() => clipboard(page)).toBe(join(shop, "src", "cart.ts"));

  menu = await menuOf(page, cart);
  const saving = page.waitForEvent("download");
  await menu.getByRole("menuitem", { name: "Download" }).click();
  const saved = await saving;
  expect(saved.suggestedFilename()).toBe("cart.ts");
  expect(readFileSync(await saved.path())).toEqual(
    readFileSync(join(shop, "src", "cart.ts")),
  );

  menu = await menuOf(page, cart);
  await menu.getByRole("menuitem", { name: "Add to chat" }).click();
  await expect(page.getByRole("textbox", { name: "Prompt" })).toHaveValue(
    "@src/cart.ts ",
  );

  // An open folder: Collapse in a group of its own, no Download or Add
  // to chat.
  menu = await menuOf(page, src);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Collapse folder",
    "Copy path",
    "Copy relative path",
  ]);
  await expect(menu.getByRole("separator")).toHaveCount(1);
  await menu.getByRole("menuitem", { name: "Collapse folder" }).click();
  await expect(cart).toHaveCount(0);
  await expect(src).toHaveAttribute("aria-expanded", "false");

  // A closed folder has nothing to collapse.
  menu = await menuOf(page, src);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Copy path",
    "Copy relative path",
  ]);
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
});
