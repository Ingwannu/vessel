// Project icons (ADR 0112): a project with Paseo's favicon under `public`
// is drawn with it, as an image from a `data:` address of the node's type,
// in the sidebar and in Settings > Projects; a project without one gets
// Paseo's lettered identity square (TRACKER 809).

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import { loadEnv } from "./fleet";
import { openUi } from "./open";

const SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#e5484d"/></svg>`;

function cli(args: string[]): string {
  const env = loadEnv();
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

test("a project's own icon is its mark, else its lettered square", async ({
  page,
}) => {
  const env = loadEnv();
  const folder = join(env.root, "repos", "iconed");
  mkdirSync(join(folder, "public"), { recursive: true });
  writeFileSync(join(folder, "public", "favicon.svg"), SVG);
  // Once per fleet: a repeated run finds it added.
  if (!cli(["project", "list"]).includes("iconed")) {
    cli(["project", "add", folder]);
  }
  const origin = await openUi(page);
  const side = page.locator(".side-project", {
    has: page.getByRole("button", { name: /^iconed\b/ }),
  });
  const icon = side.locator("img.project-icon");
  await expect(icon).toHaveAttribute(
    "src",
    `data:image/svg+xml;base64,${Buffer.from(SVG).toString("base64")}`,
  );
  await expect
    .poll(() => icon.evaluate((image: HTMLImageElement) => image.naturalWidth))
    .toBeGreaterThan(0);
  // The fixture's own repositories have none: Paseo's lettered square.
  const shop = page.locator(".side-project", {
    has: page.getByRole("button", { name: /^shop\b/ }),
  });
  await expect(shop.locator(".project-identity")).toHaveText("S");
  await expect(shop.locator("img")).toHaveCount(0);

  // Settings › Projects (the sidebar's "+" opens Add project's sheet).
  await page.goto(`${origin}/settings/projects?machine=laptop`);
  const rows = page.getByRole("list", { name: "Projects" });
  const row = rows.getByRole("listitem").filter({
    has: page.getByRole("button", { name: "iconed", exact: true }),
  });
  await expect(row.locator("img.project-icon")).toBeVisible();
  // The fleet is shared by every spec: leave it as it was.
  cli(["project", "remove", "iconed"]);
});
