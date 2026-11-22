// Exercise the typed cadence-only write through the real paired Bridge.
import { expect, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { choose } from "./kit";
import { openUi } from "./open";

test("edits an elapsed heartbeat at its opened revision without replacing the definition", async ({
  page,
}) => {
  const env = loadEnv();
  const onLaptop = (...args: string[]) =>
    cli(env, ["--node", "laptop", ...args]);
  const name = "Heartbeat cadence fixture";
  const session = onLaptop(
    "run",
    "claude",
    "--cwd",
    env.root,
    "--structured",
    "--detach",
  ).trim();
  let created = false;
  try {
    onLaptop(
      "schedule",
      "create",
      "--name",
      name,
      "--session",
      session,
      "--every",
      "90s",
      "--prompt",
      "Check the queue.",
      "--max-runs",
      "7",
      "--paused",
    );
    created = true;
    const show = () => onLaptop("schedule", "show", name);
    await openUi(page);
    await page.getByRole("link", { name: "Schedules", exact: true }).click();
    const open = page.getByRole("button", { name: `Edit heartbeat ${name}` });
    await expect(open).toBeVisible();
    await page.getByRole("button", { name: `Actions for ${name}` }).click();
    const menu = page.getByRole("menu");
    await expect(menu.getByRole("menuitem")).toHaveText([
      "Edit heartbeat",
      "Delete heartbeat",
    ]);
    await menu.getByRole("menuitem", { name: "Edit heartbeat" }).click();
    const edit = page.getByRole("dialog", { name: "Edit heartbeat" });
    const save = edit.getByRole("button", { name: "Save changes" });
    await expect(edit.getByRole("group", { name: "Target" })).toBeVisible();
    await expect(
      edit.getByText("Every 90 seconds", { exact: true }),
    ).toBeVisible();
    await expect(
      edit.getByRole("textbox", { name: "Cron expression" }),
    ).toHaveValue("");
    await expect(save).toBeDisabled();

    onLaptop(
      "schedule",
      "edit",
      name,
      "--prompt",
      "Check the newly named queue.",
    );
    const before = show();
    const pick = () =>
      choose(
        page,
        edit.getByRole("combobox", { name: "Cadence" }),
        /^Daily 9:00$/,
      );
    await pick();
    await expect(edit.getByText(/Next run/)).toBeVisible();
    await save.click();
    await expect(edit.getByRole("alert")).toContainText(
      "the schedule was changed meanwhile",
    );
    expect(show()).toBe(before);
    await edit.getByRole("button", { name: "Cancel" }).click();

    await page.reload();
    await open.click();
    await pick();
    await expect(save).toBeEnabled();
    await save.click();
    await expect(edit).toBeHidden();
    const after = show();
    expect(after).toContain("0 9 * * *");
    expect(after).toContain("paused");
    expect(after.slice(after.indexOf("\n"))).toBe(
      before.slice(before.indexOf("\n")),
    );
  } finally {
    if (created) onLaptop("schedule", "delete", name);
    onLaptop("kill", session);
  }
});
