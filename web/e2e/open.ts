// Each independent browser context gets a fresh one-use UI launch. Nodes
// stay shared, but a consumed bootstrap must never be reset for a test.
import { spawn } from "node:child_process";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { expect, type Page } from "@playwright/test";
import { loadEnv } from "./fleet";

/** `root` is where the UI's user keeps its book and data: the fleet's
 * own by default; another directory gives a UI with an empty book and no
 * node of its own (the first screen, settings-screens.ts). */
export async function launchUi(
  page: Page,
  port = 0,
  root: string = loadEnv().root,
): Promise<{ url: string; origin: string }> {
  const env = loadEnv();
  const child = spawn(env.vessel, ["ui", "--no-open", "--port", String(port)], {
    env: {
      PATH: process.env["PATH"],
      SYSTEMROOT: process.env["SYSTEMROOT"],
      HOME: root,
      USERPROFILE: root,
      VESSEL_CONFIG_DIR: join(root, "config"),
      VESSEL_DATA_DIR: join(root, "ui-data"),
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  page.on("close", () => {
    child.kill();
  });
  const lines = createInterface({ input: child.stdout });
  const url = await new Promise<string>((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error("UI launch timed out"));
    }, 15_000);
    lines.on("line", (line) => {
      const match = /^Vessel UI: (http:\/\/\S+)$/.exec(line);
      if (match?.[1] !== undefined) {
        clearTimeout(timer);
        resolve(match[1]);
      }
    });
    child.on("exit", () => {
      clearTimeout(timer);
      reject(new Error("UI ended before startup"));
    });
    child.on("error", () => {
      clearTimeout(timer);
      reject(new Error("UI could not start"));
    });
  });
  return { url, origin: new URL(url).origin };
}

/** Launch address `url` opening `path` instead of the index (which opens
 * a new chat since 2026-11-13): the one-use code is in the fragment, and
 * the page reads it on any path. */
export function launchAt(url: string, path: string): string {
  const at = new URL(url);
  at.pathname = path;
  return at.toString();
}

export async function openUi(page: Page): Promise<string> {
  const { url, origin } = await launchUi(page);
  try {
    // Do not let Playwright's navigation error echo a capability URL.
    await page.goto(url);
  } catch {
    throw new Error("Could not open the UI");
  }
  await expect(page.getByRole("navigation", { name: "Sidebar" })).toBeVisible();
  // The address's code is used once: the page keeps the session it
  // proved for reloads only after `vessel ui` welcomed it
  // (bridge/token.ts). The sidebar is drawn before that; a test that
  // navigates as soon as it shows would reload with neither, and see
  // "Open Vessel from its address" (raced under load: app.spec.ts's
  // "every New", a slow bridge reproduces it every time).
  await page.waitForFunction(
    () => window.sessionStorage.getItem("vessel.session") !== null,
  );
  return origin;
}
