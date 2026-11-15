// Model routing in the UI (milestone 5): laptop's routes (e2e/fleet.ts)
// in the fleet panel and the New session picker, and a routed session's
// model calls, which go through the node's proxy to `fake-provider`.

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionPage, optionOf, timeline } from "./kit";
import { openUi } from "./open";

/** Start a session on laptop with Claude Code under profile
 * `claude-qwen`, which runs it on route `qwen-coder`. */
async function routed(
  page: Page,
  mode: "Structured" | "Terminal",
  prompt = "",
): Promise<void> {
  const box = await newSessionPage(page, "laptop", mode);
  await box.getByRole("button", { name: "Add", exact: true }).click();
  const start = page.getByRole("dialog", { name: "Start with" });
  const profile = start.getByRole("combobox", { name: "Profile" });
  // A profile on a model route says which.
  await expect(
    await optionOf(page, profile, /^Claude Code on Qwen/),
  ).toContainText("qwen-coder");
  await page.keyboard.press("Escape");
  await choose(page, profile, /^Claude Code on Qwen/);
  await expect(profile).toContainText("Claude Code on Qwen");
  await page.keyboard.press("Escape");
  if (prompt !== "") {
    await box.getByRole("textbox", { name: "Prompt" }).fill(prompt);
  }
  await box
    .getByRole("button", {
      name: mode === "Terminal" ? "Open the terminal" : "Send",
    })
    .click();
  await expect(page).toHaveURL(/\/sessions\/laptop\/[^/]+$/);
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
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    /^gpu-box\b/,
  );
  await expect(page).toHaveURL(/\/settings\/models\?machine=gpu-box$/);
  await expect(
    page.getByText("No routes: its agents use their own providers."),
  ).toBeVisible();
});

test("a routed structured session lists its model calls", async ({ page }) => {
  await openUi(page);
  await routed(page, "Structured", "model what is a vessel?");
  await expect(
    timeline(page).getByText("model fake reply to: what is a vessel?"),
  ).toBeVisible();
  const call = page.locator(".t-item.model-call");
  await expect(call).toHaveCount(1);
  // fake-provider reports 12 prompt and 5 completion tokens. The call's
  // time is whatever the machine took, so every form `duration()` writes
  // is accepted: `40 ms`, `1.2 s` (from a second), `2 m 5 s` (from a
  // minute). A loaded runner took over a second and the old `\d+ ms`
  // failed on the correct "1.0 s".
  await expect(call).toContainText(
    /Model call · qwen-coder · qwen\/qwen3-coder · 12 in · 5 out · end turn · (?:\d+ ms|\d+\.\d s|\d+ m \d+ s)/,
  );
  const summary = page.getByRole("complementary", { name: "Session panel" });
  await expect(
    summary.getByRole("button", { name: /^Model calls/ }),
  ).toContainText("1");
  await expect(summary).toContainText("qwen/qwen3-coder");
  // The identifiers are behind Details.
  await summary.getByRole("button", { name: "Details" }).click();
  await expect(summary).toContainText("Model route");
  await expect(summary).toContainText("qwen-coder");
  await expect(summary).toContainText("claude-qwen");
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
