// `npm run screens`: every main screen, captured into artifacts/screens/
// (untracked) from a real fleet with some work on it, for review. Not a
// test of behaviour (app.spec.ts is); each step only waits for what the
// picture needs. The kit's gallery is captured by kit-screens.ts.

import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionPage, timeline } from "./kit";
import { openUi } from "./open";
import { loadEnv } from "./fleet";
import { fleetHome } from "./fleet-home";
import { shot } from "./shot";

/** Start a session from the new-session page, in `cwd`, under the
 * profile named `profile` (one on a model route) when given. */
async function newSession(
  page: Page,
  node: string,
  mode: "Structured" | "Terminal",
  cwd: string,
  { profile = "", prompt = "" }: { profile?: string; prompt?: string } = {},
): Promise<void> {
  const box = await newSessionPage(page, node, mode);
  if (profile !== "") {
    await box.getByRole("button", { name: "Add" }).click();
    await choose(
      page,
      page
        .getByRole("dialog", { name: "Start with" })
        .getByRole("combobox", { name: "Profile" }),
      new RegExp(`^${profile}\\b`),
    );
    await page.keyboard.press("Escape");
  }
  await page.getByRole("combobox", { name: "Folder" }).fill(cwd);
  await page.keyboard.press("Escape");
  if (prompt !== "") {
    await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  }
  await box
    .getByRole("button", {
      name: mode === "Terminal" ? "Open the terminal" : "Send",
    })
    .click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
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
  // `model <text>` is one call through laptop's proxy to fake-provider.
  const questions = [
    "What does the fleet overview's generation column mean?",
    "Which member reported last?",
  ];
  await newSession(page, "laptop", "Structured", project, {
    profile: "Claude Code on Qwen",
    prompt: `model ${questions[0] ?? ""}`,
  });
  const prompt = page.getByRole("textbox", { name: "Prompt" });
  await expect(timeline(page).getByText("Agent ready")).toBeVisible();
  await expect(
    timeline(page).getByText(`model fake reply to: ${questions[0] ?? ""}`),
  ).toBeVisible();
  await prompt.fill(`model ${questions[1] ?? ""}`);
  await prompt.press("Enter");
  await expect(
    timeline(page).getByText(`model fake reply to: ${questions[1] ?? ""}`),
  ).toBeVisible();
  await expect(page.locator(".t-item.model-call")).toHaveCount(2);
  await prompt.fill(
    "The fleet overview draws stale generation numbers. Check how members report what they applied, then fix the display.",
  );
  await prompt.press("Enter");
  await expect(
    page.getByRole("group", { name: "Permission request" }),
  ).toBeVisible();
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

  // The new-session page, its options read from the agent.
  const box = await newSessionPage(page, "laptop", "Structured");
  await expect(
    box.getByRole("button", { name: /^Permissions:/ }),
  ).toBeVisible();
  await box
    .getByRole("textbox", { name: "Prompt" })
    .fill("Check how members report what they applied, then fix the display.");
  await shot(page, "05-new-session");

  // Settings (Machines, Agents, Models, Devices, Sync, Appearance, About)
  // and the welcome are captured by settings-screens.ts.

  // The dark theme: the same system inverted (DESIGN.md §4.2).
  await page.goto(`${origin}/settings/appearance`);
  await page.getByRole("radio", { name: "Dark" }).click();
  await page.goto(structured);
  await expect(
    page.getByRole("group", { name: "Permission request" }),
  ).toBeVisible();
  await shot(page, "10-session-structured-dark");
  await page.goto(`${origin}/sessions`);
  await expect(
    page.getByRole("group", { name: "Sessions by status" }),
  ).toBeVisible();
  await shot(page, "11-sessions-dark");
});
