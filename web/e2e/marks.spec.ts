// A session's marks on the page (milestone 11, Track A-page), against a
// real node: pin, unread and read, start again, archive and the archive's
// restore and delete, the same in the title bar's menu, the sidebar and
// the board.

import { expect, type Page, test } from "@playwright/test";

import { startSession, timeline } from "./kit";
import { openUi } from "./open";

async function act(page: Page, action: string): Promise<void> {
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: action }).click();
}

test("pin, unread, start again, archive, restore and delete", async ({
  page,
}) => {
  test.setTimeout(120_000);
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  const url = page.url();
  const id = url.split("/").at(-1) ?? "";
  const row = page.getByRole("navigation").locator(`a[href$="${id}"]`).first();

  // Pinned: the menu offers the other way.
  await act(page, "Pin session");
  await page.getByRole("button", { name: "Session actions" }).click();
  await expect(
    page.getByRole("menuitem", { name: "Unpin session" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");

  // Unread, though it is in front: the page holds its own read mark until
  // something new arrives.
  await act(page, "Mark as unread");
  await expect(row.getByRole("img", { name: "unread" })).toBeVisible();
  await page.waitForTimeout(1500);
  await expect(row.getByRole("img", { name: "unread" })).toBeVisible();
  const text = page.getByRole("textbox", { name: "Prompt" });
  await text.fill("plain");
  await text.press("Enter");
  await expect(timeline(page).getByText("echo: plain")).toHaveCount(2);
  await expect(row.getByRole("img", { name: "unread" })).toBeHidden();

  // Ended, it offers to start again; started again, the same log goes on.
  await act(page, "Kill");
  await page.getByRole("dialog").getByRole("button", { name: "Kill" }).click();
  const ended = page.getByRole("region", { name: "Session ended" });
  await expect(ended).toContainText("History is kept.");
  await ended.getByRole("button", { name: "Start again" }).click();
  await expect(ended).toBeHidden();
  await text.fill("loaded");
  await text.press("Enter");
  await expect(timeline(page).getByText("loaded fake-session-1")).toBeVisible();

  // Archived (it runs: archiving stops it), it leaves the sidebar and
  // goes to the board's archive.
  await act(page, "Archive session");
  const confirm = page.getByRole("dialog");
  await expect(confirm).toContainText("Archiving stops the agent");
  await confirm.getByRole("button", { name: "Confirm" }).click();
  await expect(row).toBeHidden();
  await page.goto(`${new URL(url).origin}/sessions`);
  await page.locator(".archive-bar").getByRole("button").click();
  const archive = page.getByRole("dialog", { name: "Archive" });
  const card = archive.locator(`[data-session="${id}"]`);
  await expect(card).toBeVisible();

  // Deleted for good from the archive.
  await card.hover();
  await card.getByRole("button", { name: "Delete permanently" }).click();
  const sure = page.getByRole("dialog", {
    name: "Delete this session permanently?",
  });
  await sure.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByText("Session deleted")).toBeVisible();
  await expect(card).toBeHidden();
});

test("a restored session comes back to its lane", async ({ page }) => {
  await openUi(page);
  await startSession(page, "gpu-box", "Structured", "plain");
  await expect(timeline(page).getByText(/Turn ended/)).toBeVisible();
  const id = page.url().split("/").at(-1) ?? "";
  await act(page, "Archive session");
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Confirm" })
    .click();
  await page.goto(`${new URL(page.url()).origin}/sessions`);
  await page.locator(".archive-bar").getByRole("button").click();
  const card = page
    .getByRole("dialog", { name: "Archive" })
    .locator(`[data-session="${id}"]`);
  await card.hover();
  await card.getByRole("button", { name: "Restore session" }).click();
  await expect(card).toBeHidden();
  await page.keyboard.press("Escape");
  await expect(
    page
      .getByRole("region", { name: "Building" })
      .locator(`[data-session="${id}"]`),
  ).toBeVisible();
});
