// The first chat (2026-11-13, the owner: "type and send, and it should
// just work, as in GOAT or the orchestrator"): Send on the new-session
// page opens the chat at once with the prompt in it, while the agent is
// still starting, and the agent's answer follows in the same chat.
// Timed against an agent that takes four seconds to start ("Slow
// starter", the fake ACP agent with `--slow-start`, added on laptop by
// e2e/fleet.ts): before, the page stayed on the new-session page until
// the agent had started and taken the prompt.

import { expect, test } from "@playwright/test";

import { choose, chooseAgent, newSessionPage, openNew, timeline } from "./kit";
import { openUi } from "./open";
import { shot } from "./shot";

/** How long the slow agent takes to start (e2e/fleet.ts). */
const START_MS = 4_000;

test("type, send, and the chat is open with the prompt in it before the agent has started", async ({
  page,
}) => {
  await openUi(page);
  await openNew(page);
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    /^laptop\b/,
  );
  await chooseAgent(page, "Slow starter");
  // What the agent offers is read first (it starts once to say): the
  // time below is Send's alone.
  await expect(page.getByText("Loading models…")).toHaveCount(0, {
    timeout: 30_000,
  });
  const box = page.getByRole("form", { name: "Prompt" });
  await box.getByRole("textbox", { name: "Prompt" }).fill("plain");
  const sent = Date.now();
  await box.getByRole("button", { name: "Send" }).click();
  if (process.env["VESSEL_SCREEN_DIR"] !== undefined) {
    // What the person sees half a second after Send.
    await page.waitForTimeout(500 - (Date.now() - sent));
    await shot(page, "n0-new-chat-half-a-second");
  }
  await expect(page).toHaveURL(/\/sessions\/laptop\/[^/]+$/);
  const chat = timeline(page);
  await expect(chat.getByText("plain", { exact: true })).toBeVisible();
  const shown = Date.now() - sent;
  if (process.env["VESSEL_SCREEN_DIR"] !== undefined) {
    await shot(page, "n1-new-chat-sending");
  }
  await expect(chat.getByText("echo: plain")).toBeVisible({ timeout: 30_000 });
  const answered = Date.now() - sent;
  // One prompt in the chat: the one shown at once became the log's.
  await expect(chat.getByText("plain", { exact: true })).toHaveCount(1);
  if (process.env["VESSEL_SCREEN_DIR"] !== undefined) {
    await shot(page, "n2-new-chat-answered");
  }
  test.info().annotations.push({
    type: "timing",
    description: `prompt shown in the chat ${String(shown)} ms after Send; answer ${String(answered)} ms after`,
  });
  console.log(
    `new chat: prompt shown ${String(shown)} ms after Send; answer ${String(answered)} ms`,
  );
  expect(shown).toBeLessThan(START_MS);
  // The page went in place of the new-session page.
  await page.goBack();
  await expect(
    page.getByRole("heading", { name: "What do you want to work on?" }),
  ).toHaveCount(0);
});

test("the first screen is a new chat in the project worked in last, and New from a session starts in its project", async ({
  page,
}) => {
  const origin = await openUi(page);
  const heading = page.getByRole("heading", {
    name: "What do you want to work on?",
  });
  const project = page.getByRole("combobox", { name: "Project" });
  // The index opens a new chat (GOAT's first screen), the cursor in it.
  await expect(heading).toBeVisible();
  await expect(page).toHaveURL(/\/new$/);
  await expect(page.getByRole("textbox", { name: "Prompt" })).toBeFocused();
  const box = await newSessionPage(page, "laptop", "Structured");
  await choose(page, project, /^notes\b/);
  await box.getByRole("textbox", { name: "Prompt" }).fill("plain");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  // New from that session starts in its project.
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(heading).toBeVisible();
  await expect(project).toHaveText(/notes/);
  // A new visit: the first screen starts in it too, laptop's latest
  // session having run there.
  await page.goto(`${origin}/`);
  await expect(heading).toBeVisible();
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    /^laptop\b/,
  );
  await expect(project).toHaveText(/notes/);
});
