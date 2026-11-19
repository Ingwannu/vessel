// ADR 0111 (TRACKER 567, 568, 569): Settings › Diagnostics runs the app
// diagnostic over every machine, an agent's card has Paseo's
// "Diagnostic", and a machine says where its node's log is.
import { expect, test } from "@playwright/test";
import { openUi } from "./open";

test("the app diagnostic asks each machine and shows one text", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/diagnostics`);
  await page.getByRole("button", { name: "Run" }).click();
  const dialog = page.getByRole("dialog", { name: "App diagnostic" });
  const text = dialog.getByTestId("diagnostic-text");
  await expect(text).toContainText("Vessel page diagnostics", {
    timeout: 30_000,
  });
  // The steps or the text, never both (Paseo's sheet).
  await expect(dialog.getByTestId("app-diagnostic-steps")).toHaveCount(0);
  await expect(text).toContainText("Machine: laptop");
  await expect(text).toContainText("Vessel diagnostics");
  await expect(text).toContainText("Tools");
  await expect(
    dialog.getByRole("button", { name: "Copy diagnostic" }),
  ).toBeEnabled();
});

test("an agent's Diagnostic probes it and shows Paseo's text", async ({
  page,
}) => {
  const origin = await openUi(page);
  await page.goto(
    `${origin}/settings/agents?machine=gpu-box&agent=claude-code`,
  );
  await page.getByRole("button", { name: "Diagnostic" }).click();
  const dialog = page.getByRole("dialog", { name: "Diagnostic" });
  const text = dialog.getByTestId("diagnostic-text");
  await expect(text).toContainText("Claude Code", { timeout: 30_000 });
  await expect(text).toContainText("Status: Ready");
  await dialog.getByRole("button", { name: "Refresh diagnostic" }).click();
  await expect(text).toContainText("Status: Ready", { timeout: 30_000 });
});

test("a machine says where its node's log is", async ({ page }) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/machines?machine=laptop`);
  const row = page.getByRole("listitem").filter({ hasText: "Log file" });
  await expect(row).toBeVisible();
  await expect(row.getByRole("button", { name: "Open logs" })).toBeVisible();
});
