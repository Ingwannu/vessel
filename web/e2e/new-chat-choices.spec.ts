// Choices a machine kept for an agent (its last start's model, effort
// and mode) on a Send made as soon as the new-session page opens, while
// the agent's options may still be read: applied, and kept again,
// including one equal to the agent's own default (the new chat,
// 2026-11-13). build-server's Claude Code is used, and left as it was.

import { expect, test } from "@playwright/test";

import { newSessionPage, timeline } from "./kit";
import { openUi } from "./open";

test("the choices a machine kept are applied by a Send made at once, and kept again", async ({
  page,
}) => {
  await openUi(page);
  // A start with the effort lowered on the new-session page: kept for
  // build-server's Claude Code.
  const box = await newSessionPage(page, "build-server", "Structured");
  const agent = box.getByRole("button", { name: /^Agent, model and effort/ });
  await expect(agent).toContainText("High");
  await agent.click();
  await page.getByRole("menuitem", { name: /^Effort/ }).click();
  await page.getByRole("menuitemradio", { name: "Low" }).click();
  await box.getByRole("textbox", { name: "Prompt" }).fill("options");
  await box.getByRole("button", { name: "Send" }).click();
  await expect(
    timeline(page).getByText(/^options mode=default .* effort=low/),
  ).toBeVisible();
  // The next one, sent as soon as the page opens (while the agent's
  // options may still be read): it starts with the kept effort.
  const again = await newSessionPage(page, "build-server", "Structured");
  await again.getByRole("textbox", { name: "Prompt" }).fill("options");
  await again.getByRole("button", { name: "Send" }).click();
  await expect(page).toHaveURL(/\/sessions\/build-server\/[^/]+$/);
  await expect(
    timeline(page).getByText(/^options mode=default .* effort=low/),
  ).toBeVisible();
  // The effort put back, and kept though it is the agent's own default.
  const last = await newSessionPage(page, "build-server", "Structured");
  const control = last.getByRole("button", {
    name: /^Agent, model and effort/,
  });
  await expect(control).toContainText("Low");
  await control.click();
  await page.getByRole("menuitem", { name: /^Effort/ }).click();
  await page.getByRole("menuitemradio", { name: "High" }).click();
  await last.getByRole("textbox", { name: "Prompt" }).fill("options");
  await last.getByRole("button", { name: "Send" }).click();
  await expect(
    timeline(page).getByText(/^options mode=default .* effort=high/),
  ).toBeVisible();
  const next = await newSessionPage(page, "build-server", "Structured");
  await expect(
    next.getByRole("button", { name: /^Agent, model and effort/ }),
  ).toContainText("High");
});
