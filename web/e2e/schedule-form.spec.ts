// Exercise the sheet with the machine's real project and harness catalogue.
import { expect, test } from "@playwright/test";

import { SCHEDULE_ISOLATION_PROTOCOL } from "../src/wire/proto";

import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { choose } from "./kit";
import { openUi } from "./open";

test("creates a cron schedule and edits a stored elapsed schedule without conversion", async ({
  page,
}) => {
  const env = loadEnv();
  const name = "Recurring review fixture";
  const elapsed = "Elapsed review fixture";
  cli(env, [
    "--node",
    "laptop",
    "schedule",
    "create",
    "--name",
    elapsed,
    "--harness",
    "claude-code",
    "--project",
    "shop",
    "--every",
    "45m",
    "--max-runs",
    "3",
    "--prompt",
    "Original elapsed prompt",
  ]);
  try {
    await openUi(page);
    await page.getByRole("link", { name: "Schedules", exact: true }).click();
    await page
      .getByRole("button", { name: "New schedule", exact: true })
      .click();
    const create = page.getByRole("dialog", { name: "New schedule" });
    const save = create.getByRole("button", { name: "Create schedule" });
    await expect(save).toBeDisabled();
    await create.getByLabel("Name", { exact: true }).fill(name);
    await create
      .getByLabel("Prompt", { exact: true })
      .fill("Review the open changes");
    const host = create.getByRole("combobox", { name: "Host" });
    await choose(page, host, /^laptop$/);
    await choose(
      page,
      create.getByRole("combobox", { name: "Project" }),
      /^shop$/,
    );
    const isolation = create.getByRole("combobox", {
      name: "Isolation",
      exact: true,
    });
    if (SCHEDULE_ISOLATION_PROTOCOL === null) {
      await expect(isolation).toHaveCount(0);
    } else {
      await expect(isolation).toHaveText("Local");
      await choose(page, isolation, /^Worktree$/);
      await expect(isolation).toHaveText("Worktree");
    }
    const model = create.getByRole("group", { name: "Model", exact: true });
    const picker = model.getByRole("button", {
      name: /Model: Claude Code · Fake-small/i,
    });
    await expect(picker).toBeVisible();
    await expect(
      create.getByRole("combobox", { name: "Agent", exact: true }),
    ).toHaveCount(0);
    await picker.click();
    await page.getByRole("menuitem", { name: /^Model/ }).click();
    await page
      .getByRole("menuitemradio", { name: "Fake-large", exact: true })
      .press("Enter");
    await expect(
      model.getByRole("button", { name: /Model: Claude Code · Fake-large/i }),
    ).toBeVisible();
    await create
      .getByRole("textbox", { name: "Cron expression" })
      .fill("bad cadence");
    await expect(
      create.getByText("Enter a five-field cron expression."),
    ).toBeVisible();
    await expect(save).toBeDisabled();
    await choose(
      page,
      create.getByRole("combobox", { name: "Cadence" }),
      /^Weekdays 9:00$/,
    );
    await expect(create.getByText(/Next run/)).toBeVisible();
    await save.click();
    await expect(create).toBeHidden();
    const list = page.getByRole("list", { name: "Schedules" });
    await expect(
      list.getByRole("button", { name: `Edit schedule ${name}` }),
    ).toBeVisible();

    if (SCHEDULE_ISOLATION_PROTOCOL !== null) {
      await list.getByRole("button", { name: `Edit schedule ${name}` }).click();
      const saved = page.getByRole("dialog", { name: "Edit schedule" });
      await expect(
        saved.getByRole("combobox", { name: "Isolation", exact: true }),
      ).toHaveText("Worktree");
      await saved.getByRole("button", { name: "Cancel" }).click();
      await expect(saved).toBeHidden();
    }

    await list
      .getByRole("button", { name: `Edit schedule ${elapsed}` })
      .click();
    const edit = page.getByRole("dialog", { name: "Edit schedule" });
    await expect(
      edit.getByText("Every 45 minutes", { exact: true }),
    ).toBeVisible();
    await expect(
      edit.getByRole("textbox", { name: "Cron expression" }),
    ).toHaveCount(0);
    await expect(edit.getByLabel("Max runs", { exact: true })).toHaveValue("3");
    if (SCHEDULE_ISOLATION_PROTOCOL !== null) {
      await expect(
        edit.getByRole("combobox", { name: "Isolation", exact: true }),
      ).toHaveText("Current behavior");
    }
    await edit
      .getByLabel("Prompt", { exact: true })
      .fill("Edited elapsed prompt");
    await edit.getByRole("button", { name: "Save changes" }).click();
    await expect(edit).toBeHidden();
    const definition = cli(env, [
      "--node",
      "laptop",
      "schedule",
      "show",
      elapsed,
    ]);
    expect(definition).toContain("every 2700s");
    expect(definition).toContain("Edited elapsed prompt");
  } finally {
    for (const schedule of [name, elapsed]) {
      if (
        cli(env, ["--node", "laptop", "schedule", "list"]).includes(schedule)
      ) {
        cli(env, ["--node", "laptop", "schedule", "delete", schedule]);
      }
    }
  }
});
