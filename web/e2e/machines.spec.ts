// Settings' machines and agents end to end (track W2): a machine added
// through the dialog with a real pairing link from the test fleet, then
// renamed, made the default and forgotten, each checked against the book
// as the CLI reads it; and an agent's card asking the fake agent what it
// offers.
//
// The pairing link is a credential until used: it is never put in an
// assertion's message, and a step that handles it rethrows its failure
// without it.

import { execFileSync } from "node:child_process";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { type FleetEnv, loadEnv } from "./fleet";
import { openUi } from "./open";

/** `vessel` as the UI's own user runs it: the same book and data. */
function cli(env: FleetEnv, args: string[]): string {
  return execFileSync(env.vessel, args, {
    encoding: "utf8",
    env: {
      PATH: process.env["PATH"],
      HOME: env.root,
      USERPROFILE: env.root,
      VESSEL_CONFIG_DIR: join(env.root, "config"),
      VESSEL_DATA_DIR: join(env.root, "ui-data"),
    },
  });
}

/** The book as `vessel nodes list` prints it: a line per machine, the
 * default marked with `*` before its name. */
function book(env: FleetEnv): string {
  return cli(env, ["nodes", "list"]);
}

/** Whether `listing` has a line for `name` (as the default, or any). */
function lists(listing: string, name: string, asDefault = false): boolean {
  const marker = asDefault ? "\\*" : "[ *]";
  return new RegExp(`^${marker} ${name}\\s`, "m").test(listing);
}

/** A fresh single-use link from laptop, as `vessel node pair` prints it
 * on the machine to add. */
function pairingLink(env: FleetEnv): string {
  const printed = cli(env, ["--node", "laptop", "node", "pair"]);
  const link = /vessel:\/\/pair#\S+/.exec(printed)?.[0];
  if (link === undefined) {
    throw new Error("vessel node pair printed no link");
  }
  return link;
}

/** Choose `item` in the "…" menu of the machine called `name`. */
async function machineMenu(page: Page, name: string, item: string) {
  await page.getByRole("button", { name: `Actions for ${name}` }).click();
  await page.getByRole("menuitem", { name: item }).click();
}

test("a machine is added from a link, renamed, made default and forgotten", async ({
  page,
}) => {
  const env = loadEnv();
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/machines`);
  const machines = page.getByRole("list", { name: "Machines" });
  await expect(machines.getByRole("listitem")).toHaveCount(4);

  await page.getByRole("button", { name: "Add machine" }).click();
  const add = page.getByRole("dialog", { name: "Add a machine" });
  // The command to run on the other machine, as the CLI names it.
  await expect(add.locator(".command-line-text")).toHaveText(
    "vessel node pair",
  );
  const field = add.getByLabel("Paste the link it printed");

  // Text that is not a link is said to be so, and nothing is paired.
  await field.fill("not a link");
  await add.getByRole("button", { name: "Continue" }).click();
  await expect(add).toContainText("This is not a pairing link");

  const link = pairingLink(env);
  try {
    await field.fill(link);
  } catch {
    throw new Error("Could not paste the pairing link");
  }
  await add.getByRole("button", { name: "Continue" }).click();
  const confirm = page.getByRole("dialog", {
    name: "Connect to this machine?",
  });
  // The key shown is the one laptop prints for comparing.
  const status = cli(env, ["--node", "laptop", "node", "status"]);
  const key = /^key\s+(.+)$/m.exec(status)?.[1]?.trim() ?? "";
  expect(key).not.toBe("");
  await expect(confirm.locator(".add-fingerprint")).toHaveText(key);
  await expect(confirm).toContainText("Directly at 127.0.0.1:");

  // The book's name rules, checked as typed.
  const name = confirm.getByLabel("Name it");
  await name.fill("gpu-box");
  await expect(confirm).toContainText("There is already a machine called");
  await name.fill("two words");
  await expect(confirm).toContainText("no spaces");
  await expect(confirm.getByRole("button", { name: "Connect" })).toBeDisabled();
  await name.fill("studio");
  await confirm.getByRole("button", { name: "Connect" }).click();

  const done = page.getByRole("dialog", { name: "studio is ready" });
  await expect(done).toBeVisible({ timeout: 30_000 });
  const checks = done.getByRole("list", { name: "Adding the machine" });
  await expect(checks.getByRole("listitem")).toHaveText([
    /Passed:\s*Paired\s*Added to your machines as studio/,
    /Passed:\s*Connected\s*laptop/,
    /Passed:\s*Agents found\s*Claude Code, Codex/,
  ]);
  await expect(
    done.getByRole("button", { name: "Start a session there" }),
  ).toBeVisible();
  expect(lists(book(env), "studio")).toBe(true);
  await done.getByRole("button", { name: "Close" }).last().click();
  await expect(done).toBeHidden();
  await expect(
    machines.getByRole("listitem").filter({ hasText: "studio" }),
  ).toContainText("Paired");

  // Rename: the book takes the new name.
  await machineMenu(page, "studio", "Rename…");
  const rename = page.getByRole("dialog", { name: "Rename studio" });
  await rename.getByLabel("Name").fill("studio-2");
  await rename.getByRole("button", { name: "Rename" }).click();
  await expect(rename).toBeHidden();
  await expect(machines.getByRole("link", { name: "studio-2" })).toBeVisible();
  expect(lists(book(env), "studio-2")).toBe(true);
  expect(lists(book(env), "studio")).toBe(false);

  // Make default: the CLI marks it.
  await machineMenu(page, "studio-2", "Make default");
  await expect(
    machines.getByRole("listitem").filter({ hasText: "studio-2" }),
  ).toContainText("Default");
  expect(lists(book(env), "studio-2", true)).toBe(true);

  // Forget: asked first, then gone from the page and the book.
  await machineMenu(page, "studio-2", "Forget…");
  const forget = page.getByRole("dialog", { name: "Forget studio-2?" });
  await expect(forget).toContainText("Nothing on studio-2 changes");
  await forget.getByRole("button", { name: "Forget" }).click();
  await expect(forget).toBeHidden();
  await expect(machines.getByRole("link", { name: "studio-2" })).toHaveCount(0);
  expect(lists(book(env), "studio-2")).toBe(false);

  // The fleet's default as the other tests expect it.
  cli(env, ["nodes", "default", "laptop"]);
});

test("an agent's card says what the fake agent offers", async ({ page }) => {
  const origin = await openUi(page);
  await page.goto(`${origin}/settings/agents?machine=gpu-box`);
  const agents = page.getByRole("list", { name: "Agents on gpu-box" });
  await expect(
    agents.getByRole("listitem").filter({ hasText: "Claude Code" }),
  ).toContainText("Available");
  await expect(
    agents.getByRole("listitem").filter({ hasText: "Gemini" }),
  ).toContainText("Not found");

  await agents.getByRole("link", { name: "Claude Code" }).click();
  await expect(page).toHaveURL(/agent=claude-code/);
  const offers = page.getByRole("list", { name: "What Claude Code offers" });
  const row = (label: string) =>
    offers.getByRole("listitem").filter({ hasText: label });
  await expect(row("Model")).toContainText("Fake-small, Fake-large", {
    timeout: 30_000,
  });
  await expect(row("Model")).toContainText("Now Fake-small");
  await expect(row("Effort")).toContainText("Low, High");
  // The agent's "Mode" is the page's permissions.
  await expect(row("Permissions")).toContainText("Default, Plan, Locked");
  await expect(row("Fast")).toContainText("On, Off");
});

test("every way in: the sidebar, the palette, Settings and the old addresses", async ({
  page,
}) => {
  const origin = await openUi(page);
  const sidebar = page.getByRole("navigation", { name: "Sidebar" });
  const add = page.getByRole("dialog", { name: "Add a machine" });

  // The "+" on the sidebar's Machines heading.
  await sidebar.getByRole("button", { name: "Add machine" }).click();
  await expect(add).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(add).toBeHidden();

  // The palette's action.
  await page.keyboard.press("Control+k");
  await page.getByRole("option", { name: /Add a machine/ }).click();
  await expect(add).toBeVisible();
  await page.keyboard.press("Escape");

  // A machine in the sidebar opens its detail in Settings, marked there.
  await sidebar
    .getByRole("link", { name: /gpu-box/ })
    .first()
    .click();
  await expect(page).toHaveURL(/\/settings\/machines\?machine=gpu-box$/);
  await expect(page.getByRole("list", { name: "About gpu-box" })).toBeVisible();
  // Inside Settings the sidebar lists its sections, the current one marked.
  await expect(
    sidebar.getByRole("link", { name: "Machines", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await sidebar.getByRole("link", { name: "Agents", exact: true }).click();
  await expect(page).toHaveURL(/\/settings\/agents/);
  await expect(
    sidebar.getByRole("link", { name: "Agents", exact: true }),
  ).toHaveAttribute("aria-current", "page");

  // New from a machine's detail starts on that machine.
  await page.goto(`${origin}/settings/machines?machine=build-server`);
  await expect(
    page.getByRole("list", { name: "About build-server" }),
  ).toBeVisible();
  await page.keyboard.press("n");
  await expect(page).toHaveURL(/\/new\?machine=build-server$/);

  // The old addresses lead to Settings.
  await page.goto(`${origin}/fleet/laptop`);
  await expect(page).toHaveURL(/\/settings\/machines\?machine=laptop$/);
  await page.goto(`${origin}/sync?node=laptop`);
  await expect(page).toHaveURL(/\/settings\/sync\?machine=laptop$/);
});
