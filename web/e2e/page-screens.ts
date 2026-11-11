// `npm run screens`, milestone 11's track W: Settings' General,
// Appearance, Shortcuts (its recorder and the reassign dialog) and
// Notifications; a session with the find bar open, the outline's card,
// tool calls collapsed; the sidebar's Display menu and its status
// grouping; at a phone's 390px the sidebar's sheet and Shortcuts. Light
// and dark. Everything changed is put back at the end.

import { expect, type Page, test } from "@playwright/test";

import { startSession } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

type Scheme = "light" | "dark";

/** A capture with the pointer off the page, at `height` when the page
 * runs past one screen. */
async function capture(page: Page, name: string, height?: number) {
  const size = page.viewportSize() ?? { width: 1440, height: 900 };
  if (height !== undefined) {
    await page.setViewportSize({ width: size.width, height });
  }
  await page.mouse.move(size.width - 2, 2);
  await shot(page, name);
  if (height !== undefined) {
    await page.setViewportSize(size);
  }
}

async function settings(page: Page, origin: string, scheme: Scheme) {
  await page.goto(`${origin}/settings/general`);
  await expect(page.getByText("Send shortcut")).toBeVisible();
  await capture(page, `90-general-${scheme}`);
  await page.goto(`${origin}/settings/appearance`);
  await expect(page.getByText("Interface font")).toBeVisible();
  await capture(page, `91-appearance-${scheme}`);
  await page.goto(`${origin}/settings/shortcuts`);
  const session = page.getByRole("region", { name: "Session" });
  await expect(session).toBeVisible();
  await capture(page, `92-shortcuts-${scheme}`, 1150);
  await session
    .getByRole("button", { name: "Change Find in the session" })
    .click();
  await capture(page, `92b-shortcut-recording-${scheme}`);
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("dialog", { name: "Shortcut already in use" }),
  ).toBeVisible();
  await capture(page, `92c-shortcut-reassign-${scheme}`);
  await page.keyboard.press("Escape");
  await page.goto(`${origin}/settings/notifications`);
  await expect(page.getByText("Session notifications")).toBeVisible();
  await capture(page, `93-notifications-${scheme}`);
}

async function conversation(page: Page, origin: string, scheme: Scheme) {
  await startSession(page, "laptop", "Structured", "tidy the overview");
  const permission = page.getByRole("group", { name: "Permission request" });
  await permission.getByRole("button", { name: "Allow" }).click();
  await expect(permission).toBeHidden();
  const text = page.getByRole("textbox", { name: "Prompt" });
  await text.fill("plain");
  await text.press("Enter");
  await expect(page.getByText("echo: plain", { exact: true })).toBeVisible();
  const session = page.url();
  // Find, with a match marked.
  await text.press("Control+f");
  await page.getByRole("textbox", { name: "Find in session" }).fill("echo");
  await expect(
    page.getByRole("search", { name: "Find in session" }),
  ).toContainText("1/2");
  await capture(page, `94-find-${scheme}`);
  await page.keyboard.press("Escape");
  // The outline's card for the first prompt.
  await page.getByRole("button", { name: /^Jump to prompt 1 of 2/ }).hover();
  await shot(page, `95-outline-${scheme}`);
  // Tool calls collapsed.
  await page.goto(`${origin}/settings/appearance`);
  await page.getByRole("combobox", { name: "Conversation display" }).click();
  await page.getByRole("option", { name: "Collapse tool calls" }).click();
  await page.goto(session);
  await expect(page.getByText("echo: plain", { exact: true })).toBeVisible();
  await capture(page, `96-collapsed-${scheme}`);
  await page.goto(`${origin}/settings/appearance`);
  await page.getByRole("combobox", { name: "Conversation display" }).click();
  await page.getByRole("option", { name: "Full", exact: true }).click();
}

async function sidebar(page: Page, origin: string, scheme: Scheme) {
  await page.goto(`${origin}/sessions`);
  const side = page.getByRole("navigation", { name: "Sidebar" });
  await side.getByRole("button", { name: "Display" }).click();
  await expect(
    page.getByRole("menuitemradio", { name: "Status" }),
  ).toBeVisible();
  await shot(page, `97-display-menu-${scheme}`);
  await page.getByRole("menuitemradio", { name: "Status" }).click();
  await page.keyboard.press("Escape");
  await expect(side.getByText("Sessions", { exact: true })).toBeVisible();
  await capture(page, `98-sidebar-status-${scheme}`);
  await side.getByRole("button", { name: "Display" }).click();
  await page.getByRole("menuitemradio", { name: "Machine" }).click();
  await page.keyboard.press("Escape");
}

test("track W's screens, light and dark, wide and on a phone", async ({
  page,
}) => {
  test.setTimeout(300_000);
  const origin = await openUi(page);
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await settings(page, origin, scheme);
    await conversation(page, origin, scheme);
    await sidebar(page, origin, scheme);
  }
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${origin}/settings/shortcuts`);
    await capture(page, `99b-shortcuts-390-${scheme}`, 1400);
    await page.getByRole("button", { name: "Open sidebar" }).click();
    await expect(page.getByRole("dialog", { name: "Sidebar" })).toBeVisible();
    await shot(page, `99-sheet-390-${scheme}`);
    await page.keyboard.press("Escape");
  }
});
