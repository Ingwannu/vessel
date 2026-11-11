// `npm run screens`: every main screen, captured into artifacts/screens/
// (untracked) from a real fleet with some work on it, for review. Not a
// test of behaviour (app.spec.ts is); each step only waits for what the
// picture needs. The kit's gallery is captured by kit-screens.ts.

import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionDialog } from "./kit";
import { openUi } from "./open";
import { loadEnv } from "./fleet";
import { fleetHome } from "./fleet-home";
import { shot } from "./shot";

async function newSession(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
  cwd: string,
  route = "",
): Promise<void> {
  const dialog = await newSessionDialog(page, node, mode);
  if (route !== "") {
    // Picks the profile on the route (e2e/fleet.ts: claude-qwen).
    await choose(page, dialog.getByLabel("Model"), new RegExp(`^${route}\\b`));
  }
  await dialog.getByLabel("Working directory").fill(cwd);
  await dialog.getByRole("button", { name: "Create" }).click();
  await expect(dialog).toBeHidden();
}

test("every main screen", async ({ page }) => {
  test.setTimeout(240_000);
  const env = loadEnv();

  try {
    await fleetHome(env);
  } catch {
    throw new Error("Fleet sync setup failed");
  }
  const origin = await openUi(page);

  // Some work on the fleet: a structured session on laptop's model route
  // with two model calls and a pending permission, terminals on gpu-box
  // and build-server.
  const project = join(env.root, "laptop-home");
  await newSession(page, "gpu-box", "Terminal", join(env.root, "gpu-box-home"));
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("ready");
  await terminal.click();
  // Seed screenshots only after the PTY is ready and acknowledges each
  // command. A screenshot setup should not race a new harness's startup.
  await page.keyboard.insertText("cwd");
  await expect(terminal).toContainText("cwd");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("cwd /");
  await page.keyboard.insertText("args");
  await expect(terminal).toContainText("args");
  await page.keyboard.press("Enter");
  await expect(terminal).toContainText("args [");
  await newSession(
    page,
    "build-server",
    "Terminal",
    join(env.root, "build-server-home"),
  );
  await newSession(page, "laptop", "Structured", project, "qwen-coder");
  const prompt = page.getByRole("textbox", { name: "Prompt" });
  await expect(page.getByText(/Ready · harness session/)).toBeVisible();
  // `model <text>` is one call through laptop's proxy to fake-provider.
  for (const question of [
    "What does the fleet overview's generation column mean?",
    "Which member reported last?",
  ]) {
    await prompt.fill(`model ${question}`);
    await prompt.press("Control+Enter");
    await expect(
      page.getByText(`model fake reply to: ${question}`),
    ).toBeVisible();
  }
  await expect(page.locator(".t-item.model-call")).toHaveCount(2);
  await prompt.fill(
    "The fleet overview draws stale generation numbers. Check how members report what they applied, then fix the display.",
  );
  await prompt.press("Control+Enter");
  await expect(page.locator(".perm-sticky")).toBeVisible();
  await shot(page, "03-session-structured");
  const structured = page.url();
  // The same session scrolled back to its model calls.
  await page
    .locator(".t-item.model-call")
    .first()
    .evaluate((row) => {
      row.scrollIntoView({ block: "start" });
    });
  await shot(page, "03b-session-model-calls");

  await page.goto(`${origin}/sessions`);
  const board = page.getByRole("group", { name: "Sessions by status" });
  await expect(board).toBeVisible();
  await shot(page, "02-sessions");

  // A card's link names the session, its status and its node.
  await board
    .getByRole("link", { name: /^Open .* on gpu-box$/ })
    .first()
    .click();
  await expect(page.getByLabel("Session terminal")).toContainText("args [");
  await shot(page, "04-session-terminal");

  const dialog = await newSessionDialog(page, "laptop", "Structured");
  await choose(page, dialog.getByLabel("Model"), /^qwen-coder\b/);
  await expect(dialog.getByLabel("Profile")).toContainText(
    "Claude Code on Qwen",
  );
  await shot(page, "05-new-session");
  await page.keyboard.press("Escape");

  // Settings (Machines, Agents, Models, Devices, Sync, Appearance, About)
  // and the welcome are captured by settings-screens.ts.

  // The dark theme: the same system inverted (DESIGN.md §4.2).
  await page.goto(`${origin}/settings/appearance`);
  await page.getByRole("radio", { name: "Dark" }).click();
  await page.goto(structured);
  await expect(page.locator(".perm-sticky")).toBeVisible();
  await shot(page, "10-session-structured-dark");
  await page.goto(`${origin}/sessions`);
  await expect(
    page.getByRole("group", { name: "Sessions by status" }),
  ).toBeVisible();
  await shot(page, "11-sessions-dark");
});
