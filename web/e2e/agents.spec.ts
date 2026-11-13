// Agents added on a machine by their command (protocol 11, ADR 0032), as
// the page shows them: chosen on the new-session page by their label, run
// as a conversation with no terminal offered, named on the session's bar,
// its card and its details; one whose secret the machine lacks refused in
// words before Send; and every one listed in Settings › Agents, found or
// not. The fleet adds them to laptop (e2e/fleet.ts).

import { expect, type Page, test } from "@playwright/test";

import { choose, chooseAgent, timeline } from "./kit";
import { openUi } from "./open";

/** The new-session page on laptop with `agent` chosen. */
async function newWith(page: Page, agent: string): Promise<void> {
  await page.getByRole("button", { name: "New", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "What do you want to work on?" }),
  ).toBeVisible();
  await choose(
    page,
    page.getByRole("combobox", { name: "Machine" }),
    /^laptop\b/,
  );
  await chooseAgent(page, agent);
}

test("an added agent is chosen by its label and runs as a conversation", async ({
  page,
}) => {
  const origin = await openUi(page);
  await newWith(page, "Helper");
  const box = page.getByRole("form", { name: "Prompt" });
  await expect(
    box.getByRole("button", { name: /^Agent, model and effort: Helper/ }),
  ).toBeVisible();
  // No terminal is offered for it: the "+" holds its profiles only.
  await box.getByRole("button", { name: "Add", exact: true }).click();
  const add = page.getByRole("dialog", { name: "Start with" });
  await expect(add.getByRole("combobox", { name: "Profile" })).toBeVisible();
  await expect(
    add.getByRole("switch", { name: "Open as terminal" }),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await box.getByRole("textbox", { name: "Prompt" }).fill("hello helper");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/[^/]+\/[^/]+$/);
  await expect(timeline(page).getByText("echo: hello helper")).toBeVisible();
  // Named by its label on the session's bar and in its details.
  await expect(page.getByRole("img", { name: "Helper" }).first()).toBeVisible();
  await page.getByRole("button", { name: "Details" }).click();
  await expect(page.getByText("Helper", { exact: true }).first()).toBeVisible();
  // And on its card on the board.
  await page.goto(`${origin}/sessions?node=laptop`);
  await expect(
    page
      .locator(".card")
      .filter({ hasText: "hello helper" })
      .getByRole("img", { name: "Helper" }),
  ).toBeVisible();
});

test("an added agent whose secret the machine lacks is refused in words before Send", async ({
  page,
}) => {
  await openUi(page);
  await newWith(page, "Keyed agent");
  const box = page.getByRole("form", { name: "Prompt" });
  await box.getByRole("textbox", { name: "Prompt" }).fill("hello");
  await expect(
    page.getByText(
      "Keyed agent cannot start on this machine now: it needs the secret KEYED_TOKEN, which is not set there.",
      { exact: false },
    ),
  ).toBeVisible();
  await expect(box.getByRole("button", { name: "Send" })).toBeDisabled();
});

test("Settings lists the agents added on a machine, found or not", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/agents?machine=laptop`);
  const agents = page.getByRole("list", { name: "Agents on laptop" });
  await expect(
    agents.getByRole("listitem").filter({ hasText: "Helper" }),
  ).toContainText("Runs as a conversation; it has no terminal");
  await expect(
    agents.getByRole("listitem").filter({ hasText: "ghost" }),
  ).toContainText("Not found");
  await agents.getByRole("link", { name: "Helper" }).click();
  await expect(
    page.getByRole("heading", { name: "Helper on laptop" }),
  ).toBeVisible();
});
