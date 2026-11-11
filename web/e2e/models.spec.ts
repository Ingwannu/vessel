// Model routing in the UI (milestone 5): laptop's routes (e2e/fleet.ts)
// in the fleet panel and the New session picker, and a routed session's
// model calls, which go through the node's proxy to `fake-provider`.

import { expect, type Page, test } from "@playwright/test";

import { choose, newSessionPage, optionOf } from "./kit";
import { openUi } from "./open";

/** Start a session on laptop with Claude Code under profile
 * `claude-qwen`, which runs it on route `qwen-coder`. */
async function routed(
  page: Page,
  mode: "Structured" | "Terminal",
  prompt = "",
): Promise<void> {
  const box = await newSessionPage(page, "laptop", mode);
  await box.getByRole("button", { name: "Add" }).click();
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

test("a node's routes and its profiles' routes show in its panel", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/fleet/laptop`);
  const panel = page.getByRole("complementary", { name: "laptop" });
  await expect(panel).toContainText("Model routes");
  await expect(panel).toContainText("openrouter · qwen/qwen3-coder");
  await expect(panel).toContainText("small qwen/qwen3-coder-flash");
  await expect(panel).toContainText("ollama · gpt-oss:20b");
  await expect(panel).toContainText("claude-qwen → qwen-coder");
  // A node without routes has no such section.
  await page.goto(`${origin}/fleet/gpu-box`);
  await expect(
    page.getByRole("complementary", { name: "gpu-box" }),
  ).not.toContainText("Model routes");
});

test("a routed structured session lists its model calls", async ({ page }) => {
  await openUi(page);
  await routed(page, "Structured", "model what is a vessel?");
  await expect(
    page.getByText("model fake reply to: what is a vessel?"),
  ).toBeVisible();
  const call = page.locator(".t-item.model-call");
  await expect(call).toHaveCount(1);
  // fake-provider reports 12 prompt and 5 completion tokens.
  await expect(call).toContainText(
    /Model call · qwen-coder · qwen\/qwen3-coder · 12 in · 5 out · end turn · \d+ ms/,
  );
  const summary = page.getByRole("complementary", { name: "Session summary" });
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
