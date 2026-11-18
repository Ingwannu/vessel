// TRACKER 803 (ADR 0110): "Reload agent" in a session's actions, as
// Paseo's tab menu row: its tooltip, then "Reloaded agent" once the node
// has started the agent again on its conversation, which goes on.
import { expect, test } from "@playwright/test";
import { startSession, timeline } from "./kit";
import { openUi } from "./open";

test("Reload agent restarts the session's agent and says so", async ({
  page,
}) => {
  await openUi(page);
  await startSession(page, "laptop", "Structured", "plain");
  await expect(timeline(page).getByText("echo: plain")).toBeVisible();
  await page.getByRole("button", { name: "Session actions" }).click();
  const row = page.getByRole("menuitem", { name: "Reload agent" });
  await row.hover();
  await expect(page.locator(".tooltip")).toHaveText(
    "Reload agent to update skills, MCPs or login status.",
  );
  await row.click();
  await expect(
    page.locator(".toast").filter({ hasText: "Reloaded agent" }),
  ).toBeVisible();
});
