// Exercise the node's real approval and terminal lifecycle on a separate
// checkout so the other workspace tests keep their project file.
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { openUi } from "./open";

test("runs an approved workspace service and stops the owning terminal", async ({
  page,
}) => {
  const env = loadEnv();
  const project = "workspace-scripts-fixture";
  const folder = join(env.root, "repos", project);
  mkdirSync(folder, { recursive: true });
  writeFileSync(
    join(folder, "vessel.toml"),
    [
      "[scripts.check]",
      `command = "echo check-complete"`,
      "[services.preview]",
      `command = "echo preview-ready; sleep 60"`,
      "",
    ].join("\n"),
  );
  const laptop = (args: string[]) => cli(env, ["--node", "laptop", ...args]);
  laptop(["project", "add", folder]);
  const session = laptop([
    "run",
    "claude",
    "--structured",
    "--detach",
    "--project",
    project,
    "--title",
    "Workspace scripts fixture",
  ]).trim();
  try {
    const origin = await openUi(page);
    await page.goto(`${origin}/sessions/laptop/${session}`);
    const scripts = page.getByRole("button", { name: "Scripts", exact: true });
    await scripts.click();
    const rows = page.getByRole("list", { name: "Workspace scripts" });
    await expect(rows.getByText("check", { exact: true })).toBeVisible();
    await rows.getByRole("button", { name: "Run preview" }).click();
    const approval = page.getByRole("dialog", {
      name: "Run this project's commands?",
    });
    await expect(approval.getByText(/echo check-complete/)).toBeVisible();
    await expect(approval.getByText(/echo preview-ready/)).toBeVisible();
    await approval.getByRole("button", { name: "Approve and run" }).click();
    const terminal = page.locator(".workspace-terminal .xterm-host");
    await expect(terminal).toContainText("preview-ready");

    await scripts.click();
    const row = rows.getByRole("listitem").filter({ hasText: "preview" });
    await expect(
      row.getByText("Service · Running", { exact: true }),
    ).toBeVisible();
    await row.getByRole("button", { name: "Restart preview" }).click();
    await expect(terminal).toContainText(/preview-ready.*preview-ready/s);

    await scripts.click();
    await row.getByRole("button", { name: "Stop preview" }).click();
    await expect(
      row.getByText("Service · Stopped", { exact: true }),
    ).toBeVisible();
    await expect(
      row.getByRole("button", { name: "Run preview" }),
    ).toBeVisible();
    await expect(terminal).toContainText("preview-ready");
  } finally {
    laptop(["session", "archive", session]);
    laptop(["project", "remove", project]);
  }
});
