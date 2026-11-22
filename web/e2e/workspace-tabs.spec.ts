import { expect, test } from "@playwright/test";

import { openUi } from "./open";
import { expectEditorText, openPanelTab, panelTab, startInShop } from "./shop";
import { shown, watchTyped } from "./terminal-frames";

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
  await expectEditorText(editor, "export const RATE = 0.73;\n");
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
  const bulk = page.getByRole("dialog", { name: "Close tabs to the left?" });
  await expect(bulk).toContainText("unsaved changes");
  await bulk.getByRole("button", { name: "Cancel" }).click();
  await expectEditorText(editor, "export const RATE = 0.73;\n");
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
  // Simulate an old agent tab surviving retention in a saved layout.
  await page.evaluate(() => {
    for (const key of Object.keys(localStorage)) {
      if (!key.startsWith("vessel.workspace-tabs.v1.")) {
        continue;
      }
      const value: unknown = JSON.parse(localStorage.getItem(key) ?? "null");
      if (
        typeof value === "object" &&
        value !== null &&
        "active" in value &&
        value.active === "diff" &&
        "tabs" in value &&
        Array.isArray(value.tabs)
      ) {
        const tabs: readonly unknown[] = value.tabs;
        const stale = {
          kind: "agent",
          id: "00000000-0000-4000-8000-000000000117",
        };
        localStorage.setItem(
          key,
          JSON.stringify({ ...value, tabs: [...tabs, stale] }),
        );
      }
    }
  });
  await page.reload();
  const stale = panelTab(page, "Agent");
  await expect(stale).toBeVisible();
  await stale.hover();
  await page.getByRole("button", { name: "Close · Agent" }).click();
  await expect(stale).toHaveCount(0);
  await expect(page.getByRole("dialog")).toHaveCount(0);
});

test("a shell this page opened answers a live cursor query after focus has left", async ({
  page,
}) => {
  // TRACKER 826, ADR 0073's duty: the view that opened a terminal answers
  // until another qualifies, so a program started there is not left
  // waiting (nvim asks at once).
  const typed = watchTyped(page, "terminal.send");
  await openUi(page);
  await startInShop(page, "Local", "Answer a terminal query");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  // From the New tab menu, not the shortcut: right after the session
  // opens, the shortcut can land before the page takes keys.
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  await page.getByRole("menuitem", { name: "Terminal", exact: true }).click();
  const terminal = page.locator(".workspace-terminal .xterm-host");
  await expect(terminal).toBeVisible();
  await terminal.click();
  await page.keyboard.insertText("sleep 3; printf '\\033[6n'; echo asked");
  await page.keyboard.press("Enter");
  // Focus leaves the terminal before the shell asks.
  const workspaceTerminal = page.locator(".workspace-terminal");
  await workspaceTerminal.locator("textarea").evaluate((node) => {
    if (node instanceof HTMLElement) node.blur();
  });
  await expect
    .poll(() =>
      workspaceTerminal.evaluate(
        (node) => !node.contains(document.activeElement),
      ),
    )
    .toBe(true);
  // The typed line, then exactly one answer, sent without focus.
  await expect
    .poll(() => shown(typed.join("")))
    .toMatch(/echo asked\rESC\[\d+;\d+R$/);
  // The shell goes, so the shop's next terminal test counts its own.
  await page.getByRole("button", { name: "Close · Terminal" }).click();
  await page
    .getByRole("dialog", { name: "Close terminal?" })
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(panelTab(page, "Terminal")).toHaveCount(0);
});

test("a standalone terminal accepts input and closes through the compact switcher", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Use a terminal tab");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  await expect(
    page
      .getByRole("menuitem", { name: "Terminal", exact: true })
      .locator(".menu-shortcut"),
  ).toHaveText("Alt+Shift+T");
  await page.keyboard.press("Escape");
  await page.keyboard.press("Alt+Shift+T");
  const terminal = page.locator(".workspace-terminal .xterm-host");
  await expect(terminal).toBeVisible();
  await terminal.click();
  await page.keyboard.insertText("printf '%s%s\\n' work- space-ready");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("work-space-ready");
  await page.keyboard.insertText(
    "printf '%s%s\\n' sw- catalogue sw- catalogue",
  );
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("sw-catalogue");
  await page.keyboard.press("Control+f");
  const find = page.getByRole("search", { name: "Find in terminal" });
  const query = find.getByRole("textbox");
  const result = find.getByRole("status");
  await expect(query).toBeFocused();
  await query.fill("sw-catalogue");
  await expect(result).toHaveText(/^[12] of 2$/);
  const initial = await result.textContent();
  await query.press("Enter");
  await expect(result).not.toHaveText(initial ?? "");
  await query.press("Shift+Enter");
  await expect(result).toHaveText(initial ?? "");
  await query.fill("sw-no-such-match");
  await expect(result).toHaveText("No matches");
  await query.press("Escape");
  await expect(find).toHaveCount(0);
  await expect(terminal.locator("textarea")).toBeFocused();
  const heading = page.getByRole("heading", {
    name: "Use a terminal tab",
    exact: true,
  });
  await expect(heading).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^Project shop: show changes$/ }),
  ).toBeVisible();
  await openPanelTab(page, "Files");
  await expect(heading).toBeVisible();
  await panelTab(page, "Files").click({ button: "right" });
  await page.getByRole("menuitem", { name: "Close other tabs" }).click();
  const bulk = page.getByRole("dialog", { name: "Close other tabs?" });
  await expect(bulk).toContainText("close 1 terminal(s)");
  await expect(bulk).toContainText(
    "Any running process in a closed terminal will be stopped immediately.",
  );
  await bulk.getByRole("button", { name: "Cancel" }).click();
  await panelTab(page, "Terminal").click();

  await page.setViewportSize({ width: 390, height: 844 });
  await expect(heading).toBeVisible();
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
