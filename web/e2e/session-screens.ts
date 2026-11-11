// `npm run screens`: starting and running a session (milestone 10, track
// W1), captured for review beside agent-orchestrator's and GOAT's own:
// the new-session page and each of its menus, a session waiting on a
// permission with its summary, renaming, the board and the Settings
// sidebar; in light and dark (the system's scheme, which the page follows
// until a person chooses), at full width and at 390px.

import { expect, type Page, test } from "@playwright/test";

import { newSessionPage, startSession } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

const PHONE = { width: 390, height: 844 };
const DESK = { width: 1440, height: 900 };

/** The new-session page on laptop, its options read. */
async function newPage(page: Page): Promise<void> {
  const box = await newSessionPage(page, "laptop", "Structured");
  await expect(
    box.getByRole("button", { name: /^Permissions:/ }),
  ).toBeVisible();
}

test("starting and running a session", async ({ page }) => {
  test.setTimeout(240_000);
  const origin = await openUi(page);

  // A session waiting on a permission, with a plan and a tool call.
  await startSession(
    page,
    "gpu-box",
    "Structured",
    "Find where the overview reads each member's generation, and fix the stale number.",
  );
  const permission = page.getByRole("group", { name: "Permission request" });
  await expect(permission).toBeVisible();
  const session = page.url();
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize(DESK);
    await page.goto(session);
    await expect(permission).toBeVisible();
    await shot(page, `30-session-permission-${scheme}`);
    await page.setViewportSize(PHONE);
    await shot(page, `31-session-390-${scheme}`);
  }
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize(DESK);
  await page.goto(session);
  await expect(permission).toBeVisible();

  // The summary's identifiers, behind Details.
  const summary = page.getByRole("complementary", { name: "Session summary" });
  await summary.getByRole("button", { name: "Details" }).click();
  await expect(summary).toContainText("Agent's session");
  await shot(page, "32-session-details-light");

  // Its actions, and renaming it where it stands.
  await page.getByRole("button", { name: "Session actions" }).click();
  await expect(page.getByRole("menuitem", { name: "Kill" })).toBeVisible();
  await shot(page, "33-session-actions-light");
  await page.keyboard.press("Escape");
  const bar = page.getByRole("banner");
  await bar.getByRole("heading", { level: 1 }).getByRole("button").click();
  await expect(bar.getByRole("textbox", { name: "Title" })).toBeFocused();
  await shot(page, "34-session-rename-light");
  await page.keyboard.press("Escape");

  // Answered: the dock goes, the timeline's line says how it ended.
  await permission.getByRole("button", { name: "Allow" }).click();
  await expect(page.getByText(/^Allowed · /)).toBeVisible();
  await shot(page, "35-session-answered-light");

  // The new-session page and its menus, each opened from the keyboard.
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize(DESK);
    await page.goto(`${origin}/sessions`);
    await newPage(page);
    await shot(page, `40-new-${scheme}`);
    const agent = page.getByRole("button", {
      name: /^Agent, model and effort/,
    });
    await agent.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("menuitem", { name: /^Effort/ }).hover();
    await expect(
      page.getByRole("menuitemradio", { name: "Low" }),
    ).toBeVisible();
    await shot(page, `41-new-agent-menu-${scheme}`);
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
    const permissions = page.getByRole("button", { name: /^Permissions:/ });
    await permissions.focus();
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("menuitemradio", { name: "Plan" }),
    ).toBeVisible();
    await shot(page, `42-new-permissions-${scheme}`);
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await page.getByRole("switch", { name: "Open as terminal" }).click();
    await expect(page.getByLabel("Arguments")).toBeVisible();
    await shot(page, `43-new-add-${scheme}`);
    await page.getByRole("switch", { name: "Open as terminal" }).click();
    await page.keyboard.press("Escape");
    await page.getByRole("combobox", { name: "Folder" }).click();
    await shot(page, `44-new-folder-${scheme}`);
    await page.keyboard.press("Escape");
    await page.setViewportSize(PHONE);
    await shot(page, `45-new-390-${scheme}`);
  }

  // The board with titled cards, and the sidebar inside Settings.
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize(DESK);
    await page.goto(`${origin}/sessions`);
    await expect(
      page.getByRole("group", { name: "Sessions by status" }),
    ).toBeVisible();
    await shot(page, `46-board-${scheme}`);
    await page.goto(`${origin}/settings`);
    await expect(
      page.getByRole("navigation", { name: "Sidebar" }).getByRole("link", {
        name: "Back",
      }),
    ).toBeVisible();
    await shot(page, `47-settings-sidebar-${scheme}`);
  }

  // A terminal under the same title bar.
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize(DESK);
  await page.goto(`${origin}/sessions`);
  await startSession(page, "build-server", "Terminal", "Watch the build");
  await expect(page.getByLabel("Session terminal")).toContainText("ready");
  await shot(page, "48-session-terminal-light");
});
