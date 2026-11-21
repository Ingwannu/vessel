// Terminal profiles on the page (ADR 0117): New tab's group runs a profile's
// program (the fleet's `claude` is the testkit's fake-tui, never a real
// agent), and Settings › Terminals adds, checks, moves, edits and removes a
// profile, every change kept by the node.

import { expect, test } from "@playwright/test";

import { openUi } from "./open";
import { openPanelTab, panelTab, startInShop } from "./shop";

test("a shipped profile from New tab runs its program in a terminal named after it", async ({
  page,
}) => {
  await openUi(page);
  await startInShop(page, "Local", "Run a profile");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  const group = page.getByRole("group", { name: "Terminal profiles" });
  await expect(group.getByRole("menuitem")).toHaveText([
    "Claude Code",
    "Codex",
    "OpenCode",
    "Pi",
  ]);
  await expect(
    page.getByRole("button", { name: "Edit profiles", exact: true }),
  ).toBeVisible();
  await group.getByRole("menuitem", { name: "Claude Code" }).click();
  const terminal = page.locator(".workspace-terminal .xterm-host");
  await expect(terminal).toContainText("ready");
  // No prompt: the placeholder's argument is left out, and no shell ran it.
  await terminal.click();
  await page.keyboard.insertText("args");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("args []");
  // The terminal goes, so the shop's terminal tests count their own.
  await openPanelTab(page, "Claude Code");
  await page.getByRole("button", { name: "Close · Claude Code" }).click();
  await page
    .getByRole("dialog", { name: "Close terminal?" })
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(panelTab(page, "Claude Code")).toHaveCount(0);
});

test("Settings › Terminals adds, checks, moves, edits and removes a profile", async ({
  page,
}) => {
  await openUi(page);
  await page.goto(new URL("/settings/terminals", page.url()).toString());
  const list = page.getByRole("list", { name: "Terminal profiles" });
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await page
    .getByRole("button", { name: "Add terminal profile", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Add terminal profile" });
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await expect(dialog.getByText("Name is required")).toBeVisible();
  await expect(dialog.getByText("Command is required")).toBeVisible();
  await dialog.getByLabel("Name").fill("Echo");
  await expect(dialog.getByText("Name is required")).toHaveCount(0);
  await dialog.getByLabel("Name").press("Enter");
  await expect(dialog.getByLabel("Command")).toBeFocused();
  await dialog.getByLabel("Command").fill("codex");
  await dialog.getByLabel("Command").press("Enter");
  await dialog.getByLabel("Arguments").fill("--flag   x");
  await dialog.getByLabel("Arguments").press("Enter");
  await expect(dialog).toHaveCount(0);
  const echo = list.getByRole("listitem").last();
  await expect(echo).toContainText("Echo");
  await expect(echo).toContainText("codex --flag x");
  await echo.getByRole("button", { name: "Move up" }).click();
  await expect(list.getByRole("listitem").nth(3)).toContainText("Echo");
  await expect(
    list.getByRole("listitem").first().getByRole("button", { name: "Move up" }),
  ).toBeDisabled();
  const moved = list.getByRole("listitem").nth(3);
  await moved.getByRole("button", { name: "Edit profile" }).click();
  const edit = page.getByRole("dialog", { name: "Edit terminal profile" });
  await expect(edit.getByLabel("Arguments")).toHaveValue("--flag x");
  await edit.getByLabel("Name").fill("Echo two");
  await edit.getByRole("button", { name: "Save", exact: true }).click();
  await expect(list.getByRole("listitem").nth(3)).toContainText("Echo two");
  // Kept by the node: a fresh page reads it back.
  await page.reload();
  await expect(list.getByRole("listitem").nth(3)).toContainText("Echo two");
  await list
    .getByRole("listitem")
    .nth(3)
    .getByRole("button", { name: "Remove" })
    .click();
  const confirm = page.getByRole("dialog", { name: "Remove profile?" });
  await expect(confirm).toContainText('Remove "Echo two"?');
  await confirm.getByRole("button", { name: "Remove", exact: true }).click();
  await expect(list.getByRole("listitem")).toHaveCount(4);
  await expect(list).not.toContainText("Echo two");
});
