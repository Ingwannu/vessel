import { expect, test } from "@playwright/test";

import { openUi } from "./open";
import { openPanelTab, panelTab, startInShop } from "./shop";

test("workspace files retain a dirty edit, reorder, and ask once on a bulk close", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Review workspace tabs");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Files");
  await page.getByRole("searchbox", { name: "Go to file" }).fill("tax");
  await page
    .getByRole("list", { name: "Files found" })
    .getByRole("button", { name: /tax\.ts/ })
    .click();
  const file = panelTab(page, "tax.ts");
  await expect(file).toHaveAttribute("aria-selected", "true");
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  const editor = page.getByRole("textbox", { name: "Edit src/tax.ts" });
  await editor.fill("export const RATE = 0.73;\n");
  await openPanelTab(page, "Diff");
  await file.click();
  await expect(editor).toHaveValue("export const RATE = 0.73;\n");
  await file.press("Alt+ArrowLeft");
  const strip = page.getByRole("tablist", { name: "Workspace tabs" });
  const order = await strip.getByRole("tab").allTextContents();
  expect(order.indexOf("tax.ts")).toBeLessThan(order.indexOf("Files"));
  const diff = panelTab(page, "Diff");
  await diff.dragTo(file, { targetPosition: { x: 4, y: 12 } });
  const dragged = await strip.getByRole("tab").allTextContents();
  expect(dragged.indexOf("Diff")).toBeLessThan(dragged.indexOf("tax.ts"));
  await panelTab(page, "Files").click({ button: "right" });
  await page.getByRole("menuitem", { name: "Close tabs to the left" }).click();
  const bulk = page.getByRole("dialog", { name: "Close tabs to the left" });
  await expect(bulk).toContainText("unsaved changes");
  await bulk.getByRole("button", { name: "Cancel" }).click();
  await expect(editor).toHaveValue("export const RATE = 0.73;\n");
  await page.getByRole("button", { name: "Close · tax.ts" }).click();
  const unsaved = page.getByRole("dialog", { name: "Close unsaved tab?" });
  await unsaved
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(file).toHaveCount(0);
  await openPanelTab(page, "Diff");
  await page.reload();
  await expect(panelTab(page, "Diff")).toHaveAttribute("aria-selected", "true");
});

test("a standalone terminal accepts input and closes through the compact switcher", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Use a terminal tab");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await openPanelTab(page, "Terminal");
  const terminal = page.locator(".workspace-terminal .xterm-host");
  await expect(terminal).toBeVisible();
  await terminal.click();
  await page.keyboard.insertText("printf '%s%s\\n' work- space-ready");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("work-space-ready");

  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: /Switch tabs/ }).click();
  const switcher = page.getByRole("dialog", {
    name: "Workspace tabs",
    exact: true,
  });
  await expect(
    switcher.getByRole("button", { name: "Terminal", exact: true }),
  ).toHaveAttribute("aria-current", "true");
  await switcher
    .getByRole("button", { name: "Tab actions · Terminal" })
    .click();
  await page.getByRole("menuitem", { name: "Close", exact: true }).click();
  const confirm = page.getByRole("dialog", { name: "Close terminal?" });
  await expect(confirm).toContainText("stopped immediately");
  await confirm
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(
    switcher.getByRole("button", { name: "Terminal", exact: true }),
  ).toHaveCount(0);
});
