// One capture for `npm run screens` (screens.ts, kit-screens.ts), into
// artifacts/screens/ (untracked) or VESSEL_SCREEN_DIR.

import { mkdirSync } from "node:fs";
import { hostname } from "node:os";
import { join } from "node:path";

import type { Page } from "@playwright/test";

const OUT =
  process.env["VESSEL_SCREEN_DIR"] ??
  join(import.meta.dirname, "..", "artifacts", "screens");

export async function shot(
  page: Page,
  name: string,
  { fullPage = false }: { fullPage?: boolean } = {},
): Promise<void> {
  mkdirSync(OUT, { recursive: true });
  // Let fonts and the last frames settle.
  await page.evaluate(async () => document.fonts.ready);
  await page.waitForTimeout(400);
  // Fleet join currently labels its device with the host name. Do not
  // publish the build machine's identity in otherwise synthetic fixtures.
  await page.screenshot({
    path: join(OUT, `${name}.png`),
    fullPage,
    mask: [page.getByText(hostname(), { exact: true })],
  });
}
