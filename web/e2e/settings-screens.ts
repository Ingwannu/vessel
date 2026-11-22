// `npm run screens`, Settings and the first screen (track W2): every
// section with something in it, the add-machine dialog at each stage
// (and a refused link), a machine's menu and its rename and forget
// dialogs, in both themes; the busiest of them again at a phone's 390px;
// and the welcome from a UI with an empty book and no node of its own.
//
// Pairing links are credentials until used: no capture shows one (the
// link field is captured empty, the device's link stays hidden).

import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { expect, type Page, test } from "@playwright/test";

import { type FleetEnv, loadEnv } from "./fleet";
import { cli, fleetHome } from "./fleet-home";
import { launchUi, openUi } from "./open";
import { shot } from "./shot";

type Scheme = "light" | "dark";

/** A capture with the pointer off the page (so no row is hovered), at
 * `height` when a section runs past one screen: Settings scrolls inside
 * its column, so a full-page capture would stop at the fold. */
async function capture(page: Page, name: string, height?: number) {
  const size = page.viewportSize() ?? { width: 1440, height: 900 };
  if (height !== undefined) {
    await page.setViewportSize({ width: size.width, height });
  }
  await page.mouse.move(size.width - 2, 2);
  await shot(page, name);
  if (height !== undefined) {
    await page.setViewportSize(size);
  }
}

/** A fresh link from laptop, as `vessel node pair` prints it there;
 * `label` names the device that uses it. */
function pairingLink(env: FleetEnv, label?: string): string {
  const args = ["--node", "laptop", "node", "pair"];
  const link = /vessel:\/\/pair#\S+/.exec(
    cli(env, label === undefined ? args : [...args, "--label", label]),
  )?.[0];
  if (link === undefined) {
    throw new Error("vessel node pair printed no link");
  }
  return link;
}

/** A phone paired with laptop, so Devices lists one. */
function pairPhone(env: FleetEnv): void {
  const link = pairingLink(env, "phone");
  const phone = join(env.root, "phone-settings");
  try {
    execFileSync(env.vessel, ["nodes", "add", "laptop", "--pair", "--yes"], {
      input: `${link}\n`,
      env: {
        PATH: process.env["PATH"],
        HOME: phone,
        USERPROFILE: phone,
        VESSEL_CONFIG_DIR: join(phone, "config"),
      },
    });
  } catch {
    throw new Error("Device pairing failed");
  }
}

/** Paste `link` into the add dialog's field without echoing it on a
 * failure. */
async function paste(page: Page, link: string): Promise<void> {
  try {
    await page.getByLabel("Paste the link it printed").fill(link);
  } catch {
    throw new Error("Could not paste the pairing link");
  }
}

/** The add-machine dialog through to its end, as `name`, with captures
 * of each stage. */
async function addMachine(
  page: Page,
  env: FleetEnv,
  scheme: Scheme,
  name: string,
): Promise<string> {
  await page.getByRole("button", { name: "Add machine" }).click();
  const add = page.getByRole("dialog", { name: "Add a machine" });
  await expect(add).toBeVisible();
  await capture(page, `70-add-link-${scheme}`);
  await add.getByLabel("Paste the link it printed").fill("not a link");
  await add.getByRole("button", { name: "Continue" }).click();
  await expect(add).toContainText("This is not a pairing link");
  await capture(page, `70b-add-link-unreadable-${scheme}`);
  const link = pairingLink(env);
  await paste(page, link);
  await add.getByRole("button", { name: "Continue" }).click();
  const confirm = page.getByRole("dialog", {
    name: "Connect to this machine?",
  });
  await confirm.getByLabel("Name it").fill(name);
  await capture(page, `71-add-confirm-${scheme}`);
  await confirm.getByRole("button", { name: "Connect" }).click();
  const done = page.getByRole("dialog", { name: `${name} is ready` });
  await expect(done).toBeVisible({ timeout: 30_000 });
  await capture(page, `72-add-ready-${scheme}`);
  await done.getByRole("button", { name: "Close" }).last().click();
  return link;
}

/** The same link again: refused, as a used invitation is. */
async function addRefused(
  page: Page,
  scheme: Scheme,
  link: string,
): Promise<void> {
  await page.getByRole("button", { name: "Add machine" }).click();
  await paste(page, link);
  await page.getByRole("button", { name: "Continue" }).click();
  const confirm = page.getByRole("dialog", {
    name: "Connect to this machine?",
  });
  await confirm.getByLabel("Name it").fill("studio-b");
  await confirm.getByRole("button", { name: "Connect" }).click();
  const refused = page.getByRole("dialog", { name: "studio-b is not ready" });
  await expect(refused).toBeVisible({ timeout: 30_000 });
  await capture(page, `73-add-refused-${scheme}`);
  await refused.getByRole("button", { name: "Close" }).last().click();
}

/** Every section of Settings in `scheme`. */
async function sections(
  page: Page,
  env: FleetEnv,
  origin: string,
  scheme: Scheme,
): Promise<void> {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(`${origin}/settings/machines`);
  const machines = page.getByRole("list", { name: "Machines" });
  await expect(
    machines.getByRole("listitem").filter({ hasText: "laptop" }),
  ).toContainText("Connected");
  await capture(page, `61-machines-${scheme}`);

  await machines.getByRole("link", { name: "laptop" }).click();
  await expect(
    page.getByRole("list", { name: "Providers on laptop" }),
  ).toBeVisible();
  await capture(page, `62-machine-open-${scheme}`, 1500);

  // The dialogs over the plain list, as they are first met.
  await page.goto(`${origin}/settings/machines`);
  await expect(machines).toBeVisible();
  const link = await addMachine(page, env, scheme, "studio");
  await page.getByRole("button", { name: "Actions for studio" }).click();
  await expect(page.getByRole("menuitem", { name: "Forget…" })).toBeVisible();
  await capture(page, `74-machine-menu-${scheme}`);
  await page.getByRole("menuitem", { name: "Rename…" }).click();
  const rename = page.getByRole("dialog", { name: "Rename studio" });
  await rename.getByLabel("Name").fill("studio two");
  await capture(page, `75-rename-${scheme}`);
  await rename.getByRole("button", { name: "Cancel" }).click();
  await page.getByRole("button", { name: "Actions for studio" }).click();
  await page.getByRole("menuitem", { name: "Forget…" }).click();
  const forget = page.getByRole("dialog", { name: "Forget studio?" });
  await capture(page, `76-forget-${scheme}`);
  await forget.getByRole("button", { name: "Forget" }).click();
  await expect(forget).toBeHidden();
  await addRefused(page, scheme, link);

  await page.goto(
    `${origin}/settings/agents?machine=gpu-box&agent=claude-code`,
  );
  await expect(
    page.getByRole("list", { name: "What Claude Code offers" }),
  ).toBeVisible({ timeout: 30_000 });
  await capture(page, `63-agents-${scheme}`, 1150);

  await page.goto(`${origin}/settings/models?machine=laptop`);
  await expect(
    page.getByRole("list", { name: "Model routes on laptop" }),
  ).toBeVisible();
  await capture(page, `64-models-${scheme}`);

  await page.goto(`${origin}/settings/devices?machine=laptop`);
  await expect(
    page.getByRole("list", { name: "Paired devices" }),
  ).toBeVisible();
  await page.getByLabel("Device name").fill("tablet");
  await page.getByRole("button", { name: "New link" }).click();
  await expect(
    page.getByRole("button", { name: "Show pairing link" }),
  ).toBeVisible();
  await capture(page, `65-devices-${scheme}`);

  await page.goto(`${origin}/settings/sync?machine=laptop`);
  // Members report what they applied once they have pulled it.
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if ((await page.getByText("Up to date").count()) >= 2) {
      break;
    }
    await page.waitForTimeout(1000);
    await page.reload();
  }
  await capture(page, `66-sync-${scheme}`);

  await page.goto(`${origin}/settings/appearance`);
  await expect(page.getByRole("heading", { name: "Appearance" })).toBeVisible();
  await capture(page, `67-appearance-${scheme}`);

  await page.goto(`${origin}/settings/about`);
  await expect(page.getByRole("list", { name: "Licences" })).toBeVisible();
  await capture(page, `68-about-${scheme}`, 1300);
}

test("Settings and the first screen, light and dark, wide and narrow", async ({
  page,
}) => {
  test.setTimeout(480_000);
  const env = loadEnv();
  try {
    await fleetHome(env);
  } catch {
    throw new Error("Fleet sync setup failed");
  }
  pairPhone(env);
  const origin = await openUi(page);
  for (const scheme of ["light", "dark"] as const) {
    await sections(page, env, origin, scheme);
  }

  // A phone's width: the open machine, an agent's card, the dialog.
  await page.setViewportSize({ width: 390, height: 844 });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.goto(`${origin}/settings/machines?machine=laptop`);
    await expect(
      page.getByRole("list", { name: "About laptop" }),
    ).toBeVisible();
    await capture(page, `80-machines-390-${scheme}`, 1700);
    await page.goto(
      `${origin}/settings/agents?machine=gpu-box&agent=claude-code`,
    );
    await expect(
      page.getByRole("list", { name: "What Claude Code offers" }),
    ).toBeVisible({ timeout: 30_000 });
    await capture(page, `81-agents-390-${scheme}`, 1400);
    await page.goto(`${origin}/settings/machines`);
    await page.getByRole("button", { name: "Add machine" }).click();
    await expect(
      page.getByRole("dialog", { name: "Add a machine" }),
    ).toBeVisible();
    await capture(page, `82-add-390-${scheme}`);
    await page.keyboard.press("Escape");
  }

  // The first screen: a UI whose book is empty and whose own machine runs
  // no Vessel yet.
  const empty = join(env.root, "first-run");
  mkdirSync(join(empty, "config"), { recursive: true });
  const first = await launchUi(page, 0, empty);
  try {
    await page.goto(first.url);
  } catch {
    throw new Error("Could not open the first-run UI");
  }
  for (const [width, height] of [
    [1440, 900],
    [390, 844],
  ] as const) {
    await page.setViewportSize({ width, height });
    for (const scheme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: scheme });
      await expect(
        page.getByRole("heading", { name: "Welcome to Vessel" }),
      ).toBeVisible();
      await capture(page, `60-welcome-${String(width)}-${scheme}`);
    }
  }
});
