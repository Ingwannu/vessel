// `npm run screens`, milestone 11's Track A-page: the conversation on
// the page. The node's queue over the composer, the send button's other
// way, files in the composer and in a sent prompt, the `/` and `@` list,
// a question's dock and its answered card, the context meter's figures,
// an ended session's banner, the session's actions, the sidebar's marks,
// the archive and its confirmations; light and dark, and at 390px.

import { expect, type Page, test } from "@playwright/test";

import { openNew, startSession, timeline } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

type Scheme = "light" | "dark";

const DESK = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };

/** A capture with the pointer off the page. */
async function capture(page: Page, name: string): Promise<void> {
  const size = page.viewportSize() ?? DESK;
  await page.mouse.move(size.width - 2, size.height - 2);
  await shot(page, name);
}

function text(page: Page) {
  return page
    .getByRole("form", { name: "Prompt" })
    .getByRole("textbox", { name: "Prompt" });
}

/** A screenshot-like picture drawn by the page itself (the repository
 * holds no images): a window with a sidebar and a few lines. */
async function picture(page: Page): Promise<Buffer> {
  const data = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 480;
    canvas.height = 300;
    const context = canvas.getContext("2d");
    if (context === null) {
      return "";
    }
    context.fillStyle = "#f4f4f5";
    context.fillRect(0, 0, 480, 300);
    context.fillStyle = "#18181b";
    context.fillRect(0, 0, 120, 300);
    context.fillStyle = "#3f3f46";
    for (let row = 0; row < 6; row += 1) {
      context.fillRect(16, 24 + row * 28, 80, 10);
    }
    context.fillStyle = "#2563eb";
    context.fillRect(150, 30, 300, 120);
    context.fillStyle = "#a1a1aa";
    for (let row = 0; row < 4; row += 1) {
      context.fillRect(150, 172 + row * 26, 300 - row * 50, 10);
    }
    return canvas
      .toDataURL("image/png")
      .replace(/^data:image\/png;base64,/, "");
  });
  return Buffer.from(data, "base64");
}

async function queue(page: Page, scheme: Scheme): Promise<void> {
  await startSession(page, "laptop", "Structured", "wait");
  await expect(timeline(page).getByText("turn 1")).toBeVisible();
  // Send steers by default (ADR 0108): the other way queues.
  await text(page).fill("Also update the board's tests for the archive lane");
  await text(page).press("ControlOrMeta+Enter");
  await text(page).fill(
    "Then run the whole end-to-end suite and tell me what fails",
  );
  await text(page).press("ControlOrMeta+Enter");
  await text(page).fill("Look at the reducer's index before you change it");
  await expect(
    page.getByRole("region", { name: "2 queued messages" }),
  ).toBeVisible();
  await capture(page, `a10-queue-${scheme}`);
  await page.setViewportSize(PHONE);
  await capture(page, `a11-queue-390-${scheme}`);
  await page.setViewportSize(DESK);
  // The button's tooltip, and the other way while the modifier is held.
  await page
    .getByRole("form", { name: "Prompt" })
    .getByRole("button", { name: "Steer message" })
    .hover();
  await expect(page.locator(".tooltip")).toBeVisible();
  await shot(page, `a12-send-tooltip-${scheme}`);
  await page.keyboard.down("Control");
  await expect(
    page.getByRole("form", { name: "Prompt" }).getByRole("button", {
      name: "Queue message",
    }),
  ).toBeVisible();
  await shot(page, `a13-send-queue-held-${scheme}`);
  await page.keyboard.up("Control");
  // Editing one in the box.
  await text(page).fill("");
  await page
    .getByRole("region", { name: "2 queued messages" })
    .getByRole("button", { name: "Edit queued message" })
    .first()
    .click();
  await capture(page, `a14-queue-editing-${scheme}`);
  await page.keyboard.press("Escape");
  // Stopped: the queue is held.
  await page.getByRole("button", { name: "Stop turn" }).click();
  await expect(
    page.getByText("Paused after you stopped the agent"),
  ).toBeVisible();
  await capture(page, `a15-queue-held-${scheme}`);
}

async function files(page: Page, scheme: Scheme): Promise<void> {
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  const chooser = page.waitForEvent("filechooser");
  await page
    .getByRole("form", { name: "Prompt" })
    .getByRole("button", { name: "Attach files" })
    .click();
  await (
    await chooser
  ).setFiles([
    {
      name: "board-before.png",
      mimeType: "image/png",
      buffer: await picture(page),
    },
    {
      name: "archive-notes.md",
      mimeType: "text/markdown",
      buffer: Buffer.from("# The archive\n\nWhat moves where, and why.\n"),
    },
  ]);
  await text(page).fill(
    "describe the board before and after the archive change",
  );
  await capture(page, `a20-attached-${scheme}`);
  await page.setViewportSize(PHONE);
  await capture(page, `a21-attached-390-${scheme}`);
  await page.setViewportSize(DESK);
  await text(page).fill("describe");
  await text(page).press("Enter");
  await expect(
    timeline(page).getByText(/link archive-notes\.md/),
  ).toBeVisible();
  await capture(page, `a22-prompt-files-${scheme}`);
}

async function newFiles(
  page: Page,
  origin: string,
  scheme: Scheme,
): Promise<void> {
  // The new-session page's "+" holds the files for the first message.
  await openNew(page);
  const box = page.getByRole("form", { name: "Prompt" });
  await box
    .getByRole("textbox", { name: "Prompt" })
    .fill("Sketch the archive's empty state from these notes");
  await box.getByRole("button", { name: "Add", exact: true }).click();
  await shot(page, `a23-new-add-menu-${scheme}`);
  const chooser2 = page.waitForEvent("filechooser");
  await page
    .getByRole("button", { name: "Attach files to this message" })
    .click();
  await (
    await chooser2
  ).setFiles({
    name: "archive-notes.md",
    mimeType: "text/markdown",
    buffer: Buffer.from("# The archive\n"),
  });
  await page.keyboard.press("Escape");
  await capture(page, `a24-new-attached-${scheme}`);
  // Leave nothing for the next page to send.
  await box.getByRole("button", { name: "Remove archive-notes.md" }).click();
  await box.getByRole("textbox", { name: "Prompt" }).fill("");
  await page.goto(`${origin}/sessions`);
}

async function menus(page: Page, scheme: Scheme): Promise<void> {
  await text(page).fill("/");
  await expect(page.getByRole("listbox", { name: "Commands" })).toBeVisible();
  await capture(page, `a30-commands-${scheme}`);
  await text(page).fill("Look at @board");
  await expect(
    page.getByText("This session has no project to search."),
  ).toBeVisible();
  await capture(page, `a31-files-none-${scheme}`);
  await text(page).fill("");
}

async function question(page: Page, scheme: Scheme): Promise<void> {
  await startSession(page, "laptop", "Structured", "ask claude");
  const dock = page.getByRole("region", { name: "Question from the agent" });
  await expect(dock).toBeVisible();
  await dock.getByRole("radio", { name: /SQLite/ }).click();
  await capture(page, `a40-question-${scheme}`);
  await page.setViewportSize(PHONE);
  await capture(page, `a41-question-390-${scheme}`);
  await page.setViewportSize(DESK);
  await dock.getByRole("button", { name: "Next" }).click();
  await dock.getByRole("checkbox", { name: /Metrics/ }).click();
  await dock.getByRole("textbox", { name: "Other" }).fill("and alerts");
  await capture(page, `a42-question-step-2-${scheme}`);
  await dock.getByRole("button", { name: "Submit answer" }).click();
  await expect(timeline(page).locator(".question-card")).toBeVisible();
  await capture(page, `a43-question-answered-${scheme}`);
}

async function meter(page: Page, scheme: Scheme): Promise<void> {
  await text(page).fill("usage");
  await text(page).press("Enter");
  const ring = page.getByRole("img", { name: /^Context \d+% used$/ });
  await expect(ring).toBeVisible();
  await ring.hover();
  await expect(page.getByText(/tokens \(\d+%\)/)).toBeVisible();
  await shot(page, `a50-context-${scheme}`);
}

async function marks(
  page: Page,
  origin: string,
  scheme: Scheme,
): Promise<void> {
  await startSession(page, "gpu-box", "Structured", "plain");
  await expect(timeline(page).getByText(/Turn ended/)).toBeVisible();
  await page.getByRole("button", { name: "Session actions" }).click();
  await expect(
    page.getByRole("menuitem", { name: "Pin session" }),
  ).toBeVisible();
  await shot(page, `a60-actions-${scheme}`);
  await page.getByRole("menuitem", { name: "Pin session" }).click();
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: "Mark as unread" }).click();
  // The sidebar: the machine's sessions set in from its row, this one
  // pinned first and marked unread.
  await expect(page.locator(".nav-item.sub.unread")).toBeVisible();
  await capture(page, `a66-sidebar-marks-${scheme}`);
  await page.getByRole("button", { name: "Session actions" }).click();
  await page.getByRole("menuitem", { name: "Kill" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Kill" }).click();
  await expect(
    page.getByRole("region", { name: "Session ended" }),
  ).toBeVisible();
  await capture(page, `a61-ended-${scheme}`);
  await page.getByRole("button", { name: "Session actions" }).click();
  await expect(
    page.getByRole("menuitem", { name: "Archive session" }),
  ).toBeVisible();
  await shot(page, `a62-ended-actions-${scheme}`);
  await page.keyboard.press("Escape");
  // The board keeps an ended session in its lane until it is archived.
  await page.goto(`${origin}/sessions`);
  const card = page
    .locator(".card")
    .filter({ has: page.getByRole("button", { name: "Archive session" }) })
    .first();
  await card.hover();
  await shot(page, `a65-board-ended-${scheme}`);
  await card.getByRole("button", { name: "Archive session" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await shot(page, `a67-archive-confirm-${scheme}`);
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Confirm" })
    .click();
  await page.locator(".archive-bar").getByRole("button").click();
  const archive = page.getByRole("dialog", { name: "Archive" });
  await expect(
    archive.getByRole("list", { name: "Archived sessions" }),
  ).toBeVisible();
  await archive.locator(".card").first().hover();
  await shot(page, `a63-archive-${scheme}`);
  await archive
    .locator(".card")
    .first()
    .getByRole("button", { name: "Delete permanently" })
    .click();
  await expect(
    page.getByRole("dialog", { name: "Delete this session permanently?" }),
  ).toBeVisible();
  await shot(page, `a64-delete-confirm-${scheme}`);
  await page
    .getByRole("dialog", { name: "Delete this session permanently?" })
    .getByRole("button", { name: "Cancel" })
    .click();
  await page.keyboard.press("Escape");
  await expect(archive).toBeHidden();
}

test("the conversation on the page", async ({ page }) => {
  test.setTimeout(300_000);
  const origin = await openUi(page);
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize(DESK);
    await queue(page, scheme);
    await files(page, scheme);
    await menus(page, scheme);
    await meter(page, scheme);
    await newFiles(page, origin, scheme);
    await question(page, scheme);
    await marks(page, origin, scheme);
  }
  await page.emulateMedia({ colorScheme: "light" });
});
