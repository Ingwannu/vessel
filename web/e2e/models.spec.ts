// Model routing in the UI (milestone 5): laptop's routes (e2e/fleet.ts)
// in the fleet panel and the New session picker, and a routed session's
// model calls, which go through the node's proxy to `fake-provider`.

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionDialog, optionOf } from "./kit";
import { openUi } from "./open";

/** Open New session on laptop with Claude Code on route `qwen-coder`. */
async function routed(
  page: Page,
  mode: "Structured" | "Terminal",
): Promise<void> {
  const dialog = await newSessionDialog(page, "laptop", mode);
  const model = dialog.getByLabel("Model");
  const profile = dialog.getByLabel("Profile");
  // A route no profile of this harness names is listed, not offered.
  await expect(
    await optionOf(page, model, /gpt-oss .* no profile/),
  ).toHaveAttribute("aria-disabled", "true");
  await page.keyboard.press("Escape");
  await choose(page, model, /^qwen-coder\b/);
  // Picking the route picks the profile on it, and the other way round.
  await expect(profile).toContainText("Claude Code on Qwen");
  await expect(dialog).toContainText(
    "Through profile claude-qwen, pinned to qwen/qwen3-coder and qwen/qwen3-coder-flash.",
  );
  await choose(page, profile, /^default$/);
  await expect(model).toContainText("harness default");
  await choose(page, profile, /^Claude Code on Qwen/);
  await expect(model).toContainText("qwen-coder");
  await dialog.getByRole("button", { name: "Create" }).click();
  await expect(dialog).toBeHidden();
}

test("a machine's routes and profiles show in Settings' Models", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/models?machine=laptop`);
  const routes = page.getByRole("list", { name: "Model routes on laptop" });
  const qwen = routes.getByRole("listitem").filter({ hasText: "qwen-coder" });
  await expect(qwen).toContainText("openrouter · qwen/qwen3-coder");
  await expect(qwen).toContainText("small: qwen/qwen3-coder-flash");
  await expect(
    routes.getByRole("listitem").filter({ hasText: "gpt-oss" }),
  ).toContainText("ollama · gpt-oss:20b");
  await expect(
    page.getByRole("list", { name: "Profiles on laptop" }),
  ).toContainText("Claude Code · models from qwen-coder");
  // A machine without routes says so; the picker moves between machines.
  await choose(page, page.getByLabel("Machine"), /^gpu-box\b/);
  await expect(page).toHaveURL(/\/settings\/models\?machine=gpu-box$/);
  await expect(
    page.getByText("No routes: its agents use their own providers."),
  ).toBeVisible();
});

test("a routed structured session lists its model calls", async ({ page }) => {
  await openUi(page);
  await routed(page, "Structured");
  await expect(page.locator(".session-head .meta")).toContainText(
    "claude-qwen · qwen-coder (qwen/qwen3-coder)",
  );
  await expect(page.getByText(/Ready · harness session/)).toBeVisible();
  const prompt = page.getByRole("textbox", { name: "Prompt" });
  await prompt.fill("model what is a vessel?");
  await prompt.press("Control+Enter");
  await expect(
    page.getByText("model fake reply to: what is a vessel?"),
  ).toBeVisible();
  const call = page.locator(".t-item.model-call");
  await expect(call).toHaveCount(1);
  // fake-provider reports 12 prompt and 5 completion tokens.
  await expect(call).toContainText(
    /Model call · qwen-coder · qwen\/qwen3-coder · 12 in · 5 out · end turn · \d+ ms/,
  );
  const inspector = page.getByRole("complementary", {
    name: "Session inspector",
  });
  await expect(inspector).toContainText("Model calls");
  await expect(inspector).toContainText("calls1");
  await expect(inspector).toContainText("routeqwen-coder");
  await expect(page.locator(".composer-bar .model")).toHaveText(
    "qwen/qwen3-coder",
  );
});

test("a routed terminal session counts its model calls", async ({ page }) => {
  await openUi(page);
  await routed(page, "Terminal");
  const terminal = page.getByLabel("Session terminal");
  await expect(terminal).toContainText("ready");
  await terminal.click();
  await page.keyboard.type("model hello\r");
  await expect(terminal).toContainText("model fake reply to: hello");
  await expect(page.locator(".term-chrome .calls")).toHaveText(
    "1 model call · 12 in · 5 out",
  );
});
