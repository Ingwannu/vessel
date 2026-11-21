// The Files tab's file action menu (Paseo's file-actions-menu.tsx on its
// explorer rows), against the fleet's `shop` repository: a row's right
// click lists Paseo's rows in Paseo's order for a file, an open folder
// and a closed one; the copies put the paths on the clipboard, Download
// saves the file's exact bytes, Add to chat puts a file pill in the
// session's composer, Collapse closes the folder; and the rows that change
// files (New file, New folder, Rename typed in the tree, Duplicate,
// Delete after Paseo's question), leaving `shop` as it was.

import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";

import { expect, type Locator, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { openUi } from "./open";
import { openPanelTab, startInShop } from "./shop";

/** The menu a right click on `row` opens, and its rows' words. */
async function menuOf(page: Page, row: Locator) {
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
  await openPanelTab(page, "Files");
  const tree = page.getByRole("region", { name: "Files" });
  const src = tree.getByRole("button", { name: "src", exact: true });
  await src.click();
  const cart = tree.getByRole("button", { name: "cart.ts", exact: true });
  await expect(cart).toBeVisible();

  // A file: Paseo's rows and groups (its Open to the side, Open in editor
  // and Reveal need a second pane or a desktop).
  let menu = await menuOf(page, cart);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "New file",
    "New folder",
    "Copy path",
    "Copy relative path",
    "Download",
    "Add to chat",
    "Rename",
    "Duplicate",
    "Delete",
  ]);
  await expect(menu.getByRole("separator")).toHaveCount(3);
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
  // Paseo's workspace file pill: the name, the path under it; the draft's
  // text is left as it was (TRACKER 833).
  const pills = page.getByRole("list", { name: "Attachments" });
  await expect(pills.getByRole("listitem")).toHaveCount(1);
  await expect(pills.getByRole("listitem")).toContainText("cart.ts");
  await expect(pills.getByRole("listitem")).toContainText("src/cart.ts");
  await expect(page.getByRole("textbox", { name: "Prompt" })).toHaveValue("");
  await pills.getByRole("button", { name: "Remove file attachment" }).click();
  await expect(pills).toHaveCount(0);

  await openPanelTab(page, "Files");
  // An open folder: Collapse in a group of its own, no Download or Add
  // to chat.
  menu = await menuOf(page, src);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "New file",
    "New folder",
    "Collapse folder",
    "Copy path",
    "Copy relative path",
    "Rename",
    "Duplicate",
    "Delete",
  ]);
  await expect(menu.getByRole("separator")).toHaveCount(4);
  await menu.getByRole("menuitem", { name: "Collapse folder" }).click();
  await expect(cart).toHaveCount(0);
  await expect(src).toHaveAttribute("aria-expanded", "false");

  // A closed folder has nothing to collapse.
  menu = await menuOf(page, src);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "New file",
    "New folder",
    "Copy path",
    "Copy relative path",
    "Rename",
    "Duplicate",
    "Delete",
  ]);
  await page.keyboard.press("Escape");
  await expect(menu).toHaveCount(0);
});

test("New file and folder, Rename, Duplicate and Delete change the files as Paseo's explorer does", async ({
  page,
}) => {
  const env = loadEnv();
  const shop = realpathSync(join(env.root, "repos", "shop"));
  const readme = readFileSync(join(shop, "README.md"), "utf8");
  await openUi(page);
  await startInShop(page, "Local", "Change the files");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Files");
  const tree = page.getByRole("region", { name: "Files" });
  await expect(
    tree.getByRole("button", { name: "src", exact: true }),
  ).toBeVisible();
  const space = page.locator(".tree-space");

  // The empty space: New file and New folder at the root.
  let menu = await menuOf(page, space);
  await expect(menu.getByRole("menuitem")).toHaveText([
    "New file",
    "New folder",
  ]);
  await menu.getByRole("menuitem", { name: "New folder" }).click();
  // Escape drops the name row.
  await expect(
    page.getByRole("textbox", { name: "Folder name" }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("textbox", { name: "Folder name" })).toHaveCount(
    0,
  );

  // A name Paseo refuses, in its words; a taken one is refused by the
  // node, and the file there keeps its content.
  menu = await menuOf(page, space);
  await menu.getByRole("menuitem", { name: "New file" }).click();
  await page.getByRole("textbox", { name: "File name" }).fill("a/b");
  await page.keyboard.press("Enter");
  await expect(page.locator(".toast")).toContainText(
    "Name cannot contain path separators",
  );
  menu = await menuOf(page, space);
  await menu.getByRole("menuitem", { name: "New file" }).click();
  await page.getByRole("textbox", { name: "File name" }).fill("README.md");
  await page.keyboard.press("Enter");
  await expect(page.locator(".toast.error")).toContainText("exists already");
  expect(readFileSync(join(shop, "README.md"), "utf8")).toBe(readme);

  // A folder, then a file in it, which opens (names the shop fixture
  // does not hold: it has notes/todo.md, e2e/repos.ts).
  menu = await menuOf(page, space);
  await menu.getByRole("menuitem", { name: "New folder" }).click();
  await page.getByRole("textbox", { name: "Folder name" }).fill("sketches");
  await page.keyboard.press("Enter");
  const sketches = tree.getByRole("button", { name: "sketches", exact: true });
  await expect(sketches).toBeVisible();
  expect(existsSync(join(shop, "sketches"))).toBe(true);
  menu = await menuOf(page, sketches);
  await menu.getByRole("menuitem", { name: "New file" }).click();
  await page.getByRole("textbox", { name: "File name" }).fill("todo.md");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("button", { name: "All files" })).toBeVisible();
  expect(readFileSync(join(shop, "sketches", "todo.md"), "utf8")).toBe("");
  await page.getByRole("button", { name: "All files" }).click();

  // Rename, typed in the row: the whole name selected to retype.
  const todo = tree.getByRole("button", { name: "todo.md", exact: true });
  menu = await menuOf(page, todo);
  await menu.getByRole("menuitem", { name: "Rename" }).click();
  const name = page.getByRole("textbox", { name: "File name" });
  await expect(name).toHaveValue("todo.md");
  await name.fill("plan.md");
  await page.keyboard.press("Enter");
  await expect(
    tree.getByRole("button", { name: "plan.md", exact: true }),
  ).toBeVisible();
  expect(readdirSync(join(shop, "sketches"))).toEqual(["plan.md"]);

  // Duplicate: the node's copy beside it.
  menu = await menuOf(
    page,
    tree.getByRole("button", { name: "plan.md", exact: true }),
  );
  await menu.getByRole("menuitem", { name: "Duplicate" }).click();
  await expect(
    tree.getByRole("button", { name: "plan copy.md", exact: true }),
  ).toBeVisible();
  expect(readdirSync(join(shop, "sketches")).sort()).toEqual([
    "plan copy.md",
    "plan.md",
  ]);

  // An open folder renamed onto a taken name: the node refuses, and the
  // tree stays as it was (the folder open under its name, the other one
  // untouched).
  menu = await menuOf(page, sketches);
  await menu.getByRole("menuitem", { name: "Rename" }).click();
  await page.getByRole("textbox", { name: "Folder name" }).fill("src");
  await page.keyboard.press("Enter");
  await expect(page.locator(".toast.error")).toContainText("exists already");
  await expect(sketches).toHaveAttribute("aria-expanded", "true");
  await expect(
    tree.getByRole("button", { name: "plan.md", exact: true }),
  ).toBeVisible();
  const src = tree.getByRole("button", { name: "src", exact: true });
  await expect(src).toHaveAttribute("aria-expanded", "false");
  expect(existsSync(join(shop, "src", "cart.ts"))).toBe(true);
  expect(existsSync(join(shop, "src", "plan.md"))).toBe(false);

  // A folder renamed while open stays open, its files under it.
  menu = await menuOf(page, sketches);
  await menu.getByRole("menuitem", { name: "Rename" }).click();
  await page.getByRole("textbox", { name: "Folder name" }).fill("sketches-2");
  await page.keyboard.press("Enter");
  const renamed = tree.getByRole("button", { name: "sketches-2", exact: true });
  await expect(renamed).toHaveAttribute("aria-expanded", "true");
  await expect(
    tree.getByRole("button", { name: "plan.md", exact: true }),
  ).toBeVisible();

  // Delete asks first, in Paseo's words; Cancel keeps it.
  menu = await menuOf(page, renamed);
  await menu.getByRole("menuitem", { name: "Delete" }).click();
  const ask = page.getByRole("dialog", { name: "Delete folder?" });
  await expect(ask).toContainText('"sketches-2" will be permanently deleted.');
  await ask.getByRole("button", { name: "Cancel" }).click();
  await expect(ask).toHaveCount(0);
  expect(existsSync(join(shop, "sketches-2"))).toBe(true);
  menu = await menuOf(page, renamed);
  await menu.getByRole("menuitem", { name: "Delete" }).click();
  await page
    .getByRole("dialog", { name: "Delete folder?" })
    .getByRole("button", { name: "Delete" })
    .click();
  await expect(renamed).toHaveCount(0);
  expect(existsSync(join(shop, "sketches-2"))).toBe(false);
});

test("the sort trigger orders a folder by name, last change or size, as Paseo's", async ({
  page,
}) => {
  const env = loadEnv();
  const shop = realpathSync(join(env.root, "repos", "shop"));
  // Three files whose names, times and sizes each order differently.
  const folder = join(shop, "sorting");
  mkdirSync(folder, { recursive: true });
  const made = [
    { name: "a.txt", size: 3000, at: 1_700_000_100 },
    { name: "b.txt", size: 10, at: 1_700_000_300 },
    { name: "c.txt", size: 500, at: 1_700_000_200 },
  ];
  for (const file of made) {
    const path = join(folder, file.name);
    writeFileSync(path, "x".repeat(file.size));
    utimesSync(path, file.at, file.at);
  }
  try {
    await openUi(page);
    await startInShop(page, "Local", "Sort the files");
    await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
    await openPanelTab(page, "Files");
    const tree = page.getByRole("region", { name: "Files" });
    await tree.getByRole("button", { name: "sorting", exact: true }).click();
    const order = () =>
      tree.getByRole("button", { name: /^[abc]\.txt$/ }).allInnerTexts();
    const sort = page.getByRole("button", { name: /^Sort: / });
    await expect(sort).toHaveText("Name");
    await expect.poll(order).toEqual(["a.txt", "b.txt", "c.txt"]);
    await sort.click();
    await expect(sort).toHaveText("Modified");
    await expect.poll(order).toEqual(["b.txt", "c.txt", "a.txt"]);
    await sort.click();
    await expect(sort).toHaveText("Size");
    await expect.poll(order).toEqual(["a.txt", "c.txt", "b.txt"]);
    // Kept by the bridge: the next Files tab opens in the same order.
    await openPanelTab(page, "Changes");
    await openPanelTab(page, "Files");
    await expect(page.getByRole("button", { name: /^Sort: / })).toHaveText(
      "Size",
    );
    await page.getByRole("button", { name: /^Sort: / }).click();
    await expect(page.getByRole("button", { name: /^Sort: / })).toHaveText(
      "Name",
    );
  } finally {
    rmSync(folder, { recursive: true, force: true });
  }
});
