// Intentional content-only captures from the isolated fake fleet. No URL,
// trace, video or WebSocket recording; locale changes number/date formats.
import { mkdirSync } from "node:fs";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import { startSession, timeline } from "./kit";
import { openUi } from "./open";

const SIZES = [
  { width: 1280, height: 900 },
  { width: 390, height: 844 },
  { width: 800, height: 600 },
  { width: 1024, height: 700 },
] as const;
const SCHEMES = ["light", "dark"] as const;
const LOCALES = ["en-US", "ko-KR"] as const;

for (const locale of LOCALES) {
  for (const scheme of SCHEMES) {
    for (const size of SIZES) {
      test(`usage ${locale} ${scheme} ${size.width}x${size.height}`, async ({
        browser,
      }) => {
        const context = await browser.newContext({
          locale,
          colorScheme: scheme,
          viewport: size,
        });
        const page = await context.newPage();
        try {
          const origin = await openUi(page);
          await startSession(page, "gpu-box", "Structured", "usage-reasoning");
          await expect(
            timeline(page).getByText(/Turn ended · end turn · 30 tokens/),
          ).toBeVisible();
          await page.goto(`${origin}/settings/usage?machine=gpu-box`);
          const report = page.getByRole("list", {
            name: "Reported usage",
            exact: true,
          });
          await expect(report).toBeVisible();
          await expect(
            page.getByRole("button", { name: "Refresh", exact: true }),
          ).toBeEnabled();
          await page.evaluate(async () => document.fonts.ready);
          const overflow = await page.evaluate(
            () => document.documentElement.scrollWidth > window.innerWidth,
          );
          expect(overflow).toBe(false);
          const out = process.env["VESSEL_SCREEN_DIR"];
          if (out === undefined)
            throw new Error(
              "VESSEL_SCREEN_DIR must name the capture directory",
            );
          mkdirSync(out, { recursive: true });
          await page.screenshot({
            path: join(
              out,
              `usage-${locale}-${scheme}-${size.width}x${size.height}.png`,
            ),
            fullPage: true,
          });
        } finally {
          await context.close();
        }
      });
    }
  }
}
