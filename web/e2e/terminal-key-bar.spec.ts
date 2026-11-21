// The virtual key bar under a page terminal at compact widths (P4) and a
// long paste in sends the node takes (the P1 review carry-over), against
// the fleet's fake harness and a real shell.

import { expect, test } from "@playwright/test";

import { startSession } from "./kit";
import { openUi } from "./open";
import { openPanelTab, panelTab, startInShop } from "./shop";
import { shown, watchTyped } from "./terminal-frames";

test("the key bar sends Esc, Enter and the arrows, a set modifier with the next key", async ({
  page,
}) => {
  const typed = watchTyped(page, "session.send");
  await openUi(page);
  await startSession(page, "laptop", "Terminal");
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("ready");
  const bar = page.getByTestId("terminal-virtual-keyboard");
  // Wide: no bar (Paseo's desktop and web show it only when compact).
  await expect(bar).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(bar).toBeVisible();
  await terminal.click();
  const from = typed.length;
  const ctrl = bar.getByTestId("terminal-key-ctrl");
  await ctrl.click();
  await expect(ctrl).toHaveAttribute("aria-pressed", "true");
  await bar.getByTestId("terminal-key-up").click();
  await expect(ctrl).toHaveAttribute("aria-pressed", "false");
  // A set modifier also takes the next typed character.
  await bar.getByTestId("terminal-key-alt").click();
  await page.keyboard.type("x");
  await bar.getByTestId("terminal-key-esc").click();
  await bar.getByTestId("terminal-key-enter").click();
  await expect
    .poll(() => shown(typed.slice(from).join("")))
    .toBe("ESC[1;5AESCxESC\r");
  // The soft keyboard's toggle: the terminal holds the focus, so it hides.
  const toggle = bar.getByTestId("terminal-keyboard-toggle");
  await expect(toggle).toHaveAttribute("aria-label", "Hide keyboard");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-label", "Show keyboard");
  // No Paste on the web, as Paseo's.
  await expect(bar.getByTestId("terminal-paste")).toHaveCount(0);
});

test("a 100 KiB paste reaches a shell whole, in sends the node takes", async ({
  page,
}) => {
  const typed = watchTyped(page, "terminal.send");
  await openUi(page);
  await startInShop(page, "Local", "Paste a long text");
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await page.getByRole("button", { name: "New tab", exact: true }).click();
  await page.getByRole("menuitem", { name: "Terminal", exact: true }).click();
  const terminal = page.locator(".workspace-terminal .xterm-host");
  await expect(terminal).toBeVisible();
  await terminal.click();
  // Raw input, so the tty keeps a line longer than 4 KiB; the words the
  // test waits for are printed, never typed as such.
  await page.keyboard.insertText(
    "stty -icanon -echo; echo RE''ADY; head -c $((51200 * 2)) | wc -c; stty sane",
  );
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("READY");
  await page.keyboard.insertText("x".repeat(102400));
  await expect(terminal).toContainText("102400");
  const encoder = new TextEncoder();
  expect(typed.every((sent) => encoder.encode(sent).length <= 65536)).toBe(
    true,
  );
  // The shell goes, so the shop's terminal tests count their own.
  await openPanelTab(page, "Terminal");
  await page.getByRole("button", { name: "Close · Terminal" }).click();
  await page
    .getByRole("dialog", { name: "Close terminal?" })
    .getByRole("button", { name: "Close", exact: true })
    .last()
    .click();
  await expect(panelTab(page, "Terminal")).toHaveCount(0);
});
