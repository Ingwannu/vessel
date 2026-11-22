import { expect, type Page, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { cli } from "./fleet-home";
import { openUi } from "./open";

const machine = "build-server";

async function openAgents(page: Page) {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/machines?machine=${machine}`);
  return page.getByRole("region", { name: "Agents", exact: true });
}

test("the fresh node confirms Vessel tools Off and retains a changed setting", async ({
  page,
}) => {
  const agents = await openAgents(page);
  const toggle = agents.getByRole("switch", { name: "Inject Vessel tools" });
  await expect(toggle).toHaveAttribute("aria-checked", "false");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-checked", "true");

  await page.reload();
  await expect(toggle).toHaveAttribute("aria-checked", "true");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-checked", "false");
});

test("the host system prompt retains exact text and can be cleared", async ({
  page,
}) => {
  const agents = await openAgents(page);
  const card = agents.getByRole("list", { name: "System prompt", exact: true });
  const edit = card.getByRole("button", { name: "Edit", exact: true });
  const dialog = page.getByRole("dialog", { name: "Append system prompt" });
  const entry = dialog.getByRole("textbox", { name: "Append system prompt" });
  const save = dialog.getByRole("button", { name: "Save", exact: true });
  const prompt =
    "  Keep replies concise.\nKeep track of the remaining tasks.  ";

  await edit.click();
  await expect(entry).toHaveValue("");
  await entry.fill(prompt);
  await save.click();
  await expect(dialog).toBeHidden();

  await page.reload();
  await edit.click();
  await expect(entry).toHaveValue(prompt);
  await entry.fill("");
  await save.click();
  await expect(dialog).toBeHidden();

  await edit.click();
  await expect(entry).toHaveValue("");
});

test("restart recovery retains its prompt when the policy is Off", async ({
  page,
}) => {
  const env = loadEnv();
  const settings = () => cli(env, ["--node", machine, "node", "settings"]);
  const agents = await openAgents(page);
  const title = "Resume agents after restart";
  const card = agents.getByRole("list", { name: title, exact: true });
  const toggle = card.getByRole("switch", { name: title });
  const edit = card.getByRole("button", { name: "Edit prompt" });
  const dialog = page.getByRole("dialog", { name: "Resume prompt" });
  const entry = dialog.getByRole("textbox", { name: "Resume prompt" });
  const save = dialog.getByRole("button", { name: "Save", exact: true });
  const prompt = "  Resume the current work.\nList what remains.  ";

  await expect(toggle).toHaveAttribute("aria-checked", "false");
  await expect(edit).toBeDisabled();
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-checked", "true");
  await edit.click();
  await expect(entry).toHaveValue("Resume working.");
  await entry.fill(prompt);
  await save.click();
  await expect(dialog).toBeHidden();

  await page.reload();
  await expect(toggle).toHaveAttribute("aria-checked", "true");
  await toggle.click();
  await expect(toggle).toHaveAttribute("aria-checked", "false");
  await expect(edit).toBeDisabled();
  expect(settings()).toContain(
    `startup recovery: off\nresume prompt: ${prompt}\n`,
  );
});
