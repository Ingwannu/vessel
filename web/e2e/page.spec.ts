// Track W (milestone 11) end to end: what the bridge keeps for the page
// (ADR 0025) and what the page builds on it, against the test fleet's
// real `vessel ui` and fake agents. Every test puts back what it changed:
// the fleet's one `vessel ui` keeps preferences across the run.

import { expect, type Page, test } from "@playwright/test";

import { startSession } from "./kit";
import { openUi } from "./open";

const SETTINGS = (origin: string, section: string): string =>
  `${origin}/settings/${section}`;

/** The theme the bridge wrote on the served index, by fetching it as a
 * new launch's first load would. */
async function paintedTheme(
  page: Page,
  origin: string,
): Promise<string | null> {
  const response = await page.request.get(`${origin}/sessions`);
  const html = await response.text();
  return /<html lang="en"[^>]*data-theme="(\w+)"/.exec(html)?.[1] ?? null;
}

test("a theme is kept by the bridge and painted on the index before any script", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(SETTINGS(origin, "appearance"));
  const scheme = page.getByRole("radiogroup", { name: "Colour scheme" });
  await scheme.getByRole("radio", { name: "Dark" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  expect(await paintedTheme(page, origin)).toBe("dark");
  // A reload is drawn dark from the first paint.
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  // Settings says where the choices are kept.
  await page.goto(SETTINGS(origin, "general"));
  await expect(
    page.getByRole("list", { name: "Kept" }).getByText(/ui-preferences\.json$/),
  ).toBeVisible();
  await page.goto(SETTINGS(origin, "appearance"));
  await page
    .getByRole("radiogroup", { name: "Colour scheme" })
    .getByRole("radio", { name: "System" })
    .click();
  await expect(page.locator("html")).not.toHaveAttribute("data-theme", /./);
  expect(await paintedTheme(page, origin)).toBeNull();
});

test("a draft comes back, Up recalls a sent prompt, and find finds it", async ({
  page,
}) => {
  const origin = await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(page.getByText("echo: plain", { exact: true })).toBeVisible();
  const session = page.url();
  const text = page.getByRole("textbox", { name: "Prompt" });
  // Up in the empty box recalls the prompt sent; Down empties it again.
  await text.click();
  await text.press("ArrowUp");
  await expect(text).toHaveValue("plain");
  await text.press("ArrowDown");
  await expect(text).toHaveValue("");
  // What is typed is the session's draft: it survives leaving and a
  // reload (the bridge keeps it, not the tab).
  await text.fill("half a thought");
  await page.goto(`${origin}/sessions`);
  await page.goto(session);
  await expect(page.getByRole("textbox", { name: "Prompt" })).toHaveValue(
    "half a thought",
  );
  await page.reload();
  await expect(page.getByRole("textbox", { name: "Prompt" })).toHaveValue(
    "half a thought",
  );
  await page.getByRole("textbox", { name: "Prompt" }).fill("");
  // Find in the session (Mod+F): the count, and Escape to close.
  await page.getByRole("textbox", { name: "Prompt" }).press("Control+f");
  const find = page.getByRole("textbox", { name: "Find in session" });
  await find.fill("echo");
  await expect(
    page.getByRole("search", { name: "Find in session" }),
  ).toContainText("1/1");
  await find.press("Escape");
  await expect(
    page.getByRole("search", { name: "Find in session" }),
  ).toHaveCount(0);
});

test("a shortcut is rebound in Settings, a clash is offered to move, and Reset all puts them back", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(SETTINGS(origin, "shortcuts"));
  const session = page.getByRole("region", { name: "Session" });
  await session
    .getByRole("button", { name: "Change Find in the session" })
    .click();
  await expect(page.getByText("Press shortcut · Esc to cancel")).toBeVisible();
  // A key without a modifier is refused in place.
  await page.keyboard.press("j");
  await expect(
    page.getByText("Use Ctrl or Alt with another key."),
  ).toBeVisible();
  await page.keyboard.press("Control+j");
  await expect(session.getByText("Ctrl+J")).toBeVisible();
  await expect(session.getByText("Modified")).toBeVisible();
  // A key the palette holds: "Shortcut already in use", Reassign.
  await session
    .getByRole("button", { name: "Change Find in the session" })
    .click();
  await page.keyboard.press("Control+k");
  const dialog = page.getByRole("dialog", { name: "Shortcut already in use" });
  await expect(dialog).toContainText(
    "Ctrl+K is assigned to Open the command palette. Reassign it to Find in the session?",
  );
  await dialog.getByRole("button", { name: "Reassign" }).click();
  await expect(session.getByText("Ctrl+K")).toBeVisible();
  await page.getByRole("button", { name: "Reset all" }).click();
  await page
    .getByRole("dialog", { name: "Reset all" })
    .getByRole("button", { name: "Reset all" })
    .click();
  await expect(session.getByText("Ctrl+F")).toBeVisible();
  await expect(page.getByText("Modified", { exact: true })).toHaveCount(0);
});

test("with Mod+Enter chosen, Enter starts a line and Ctrl+Enter sends", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(SETTINGS(origin, "general"));
  await page.getByRole("combobox", { name: "Send shortcut" }).click();
  await page.getByRole("option", { name: "Ctrl+Enter" }).click();
  try {
    await startSession(page, "laptop", "Structured", "plain");
    await expect(page.getByText("echo: plain", { exact: true })).toBeVisible();
    const text = page.getByRole("textbox", { name: "Prompt" });
    await text.fill("options");
    await text.press("Enter");
    await expect(text).toHaveValue("options\n");
    await text.fill("options");
    await text.press("Control+Enter");
    await expect(page.getByText(/^options mode=/)).toBeVisible();
  } finally {
    await page.goto(SETTINGS(origin, "general"));
    await page.getByRole("combobox", { name: "Send shortcut" }).click();
    await page.getByRole("option", { name: "Enter", exact: true }).click();
  }
});

test("the sidebar groups by status from its Display menu", async ({ page }) => {
  await openUi(page);
  const sidebar = page.getByRole("navigation", { name: "Sidebar" });
  await sidebar.getByRole("button", { name: "Display" }).click();
  await page.getByRole("menuitemradio", { name: "Status" }).click();
  await page.keyboard.press("Escape");
  await expect(sidebar.getByText("Sessions", { exact: true })).toBeVisible();
  await sidebar.getByRole("button", { name: "Display" }).click();
  await page.getByRole("menuitemradio", { name: "Machine" }).click();
  await page.keyboard.press("Escape");
  await expect(sidebar.getByText("Machines", { exact: true })).toBeVisible();
});

test("on a phone, Settings' sections open from the title bar's menu", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(SETTINGS(origin, "general"));
  await page.getByRole("button", { name: "Open sidebar" }).click();
  const sheet = page.getByRole("dialog", { name: "Sidebar" });
  await sheet.getByRole("link", { name: "Notifications" }).click();
  await expect(sheet).toHaveCount(0);
  await expect(page).toHaveURL(/\/settings\/notifications$/);
  await expect(page.getByText("Session notifications")).toBeVisible();
});

test("an address that names no session says so, before any call", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/sessions/laptop/not-a-session`);
  await expect(page.getByText("No session has this address.")).toBeVisible();
  await page.goto(`${origin}/sessions/..%2Fsettings/${"0".repeat(8)}`);
  await expect(page.getByText("No session has this address.")).toBeVisible();
});
