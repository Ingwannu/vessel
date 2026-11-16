// Isolated kit specimens only: no account report is fabricated or fetched.
// The shared bar is reviewed here before an application surface may use it.
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { expect, test } from "@playwright/test";

import { launchUi } from "./open";

const OUT =
  process.env["VESSEL_SCREEN_DIR"] ??
  join(import.meta.dirname, "../artifacts/screens");
const SHARES = [
  [0, 0],
  [50, 50],
  [100, 100],
  [125, 100],
] as const;

for (const locale of ["en-US", "ko-KR"] as const) {
  test.describe(locale, () => {
    test.use({ locale });
    for (const width of [1280, 390] as const) {
      for (const scheme of ["light", "dark"] as const) {
        test(`quota meter ${width} ${scheme}`, async ({ page }) => {
          test.setTimeout(120_000);
          await page.setViewportSize({ width, height: 900 });
          await page.emulateMedia({ colorScheme: scheme });
          const { origin } = await launchUi(page);
          await page.goto(`${origin}/kit`);
          const section = page.getByRole("region", {
            name: "Quota meters",
            exact: true,
          });
          await expect(section).toBeVisible();
          await page.evaluate(async () => {
            await document.fonts.ready;
          });
          await section.scrollIntoViewIfNeeded();
          const measurements = [];
          for (const [input, percent] of SHARES) {
            for (const emphasis of ["Normal", "Quiet"] as const) {
              const bar = section.getByRole("meter", {
                name: `${emphasis} ${String(input)} input`,
                exact: true,
              });
              await expect(bar).toHaveAttribute(
                "aria-valuenow",
                String(percent),
              );
              const metric = await bar.evaluate((element) => {
                const fill = element.querySelector(".quota-meter-fill");
                if (fill === null) throw new Error("Missing meter fill");
                const trackBox = element.getBoundingClientRect();
                const fillBox = fill.getBoundingClientRect();
                const trackStyle = getComputedStyle(element);
                const fillStyle = getComputedStyle(fill);
                return {
                  width: trackBox.width,
                  height: trackBox.height,
                  fillWidth: fillBox.width,
                  fillHeight: fillBox.height,
                  radius: trackStyle.borderRadius,
                  fillRadius: fillStyle.borderRadius,
                  overflow: trackStyle.overflow,
                  trackColor: trackStyle.backgroundColor,
                  fillColor: fillStyle.backgroundColor,
                };
              });
              expect(metric.height).toBe(6);
              expect(metric.fillHeight).toBe(6);
              expect(metric.radius).toBe("3px");
              expect(metric.fillRadius).toBe("999px");
              expect(metric.overflow).toBe("hidden");
              expect(
                Math.abs(metric.fillWidth - (metric.width * percent) / 100),
              ).toBeLessThan(0.02);
              expect(metric.trackColor).toBe(
                scheme === "light" ? "rgb(228, 228, 231)" : "rgb(67, 70, 69)",
              );
              if (emphasis === "Quiet") {
                expect(metric.fillColor).toBe(
                  scheme === "light" ? "rgb(212, 212, 216)" : "rgb(89, 91, 91)",
                );
              }
              measurements.push({ input, percent, emphasis, ...metric });
            }
          }
          await expect(section.getByRole("meter")).toHaveCount(12);
          const bounds = await section.boundingBox();
          if (bounds === null) throw new Error("Missing specimen bounds");
          expect(bounds.x).toBeGreaterThanOrEqual(0);
          expect(bounds.x + bounds.width).toBeLessThanOrEqual(width);
          const name = `quota-meter-${locale}-${scheme}-${String(width)}`;
          mkdirSync(OUT, { recursive: true, mode: 0o700 });
          writeFileSync(
            join(OUT, `${name}.json`),
            JSON.stringify({ locale, scheme, width, measurements }, null, 2),
            { mode: 0o600 },
          );
          await section.screenshot({ path: join(OUT, `${name}.png`) });
        });
      }
    }
  });
}
