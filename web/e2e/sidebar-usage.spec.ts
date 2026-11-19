// The sidebar's usage summary at the sidebar's width (Paseo's
// usage/sidebar-item.tsx; the coordinator's P2 r3 verdict): the line packs
// its items from the left with Paseo's gaps (12 between sources, 6 after a
// mark, 8 between windows), and three sources fit whole at 240. The kit
// gallery's specimen holds the row, since the e2e fleet has no saved login.
import { expect, test } from "@playwright/test";

import { launchUi } from "./open";

test("three sources fit the sidebar's usage line at 240, packed from the left", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  const { origin } = await launchUi(page);
  await page.goto(`${origin}/kit`);
  const box = page.getByTestId("sidebar-usage-240");
  const line = box.locator(".side-usage-line");
  await expect(line.locator(".side-usage-percent")).toHaveCount(4);
  const fits = await line.evaluate((element) => {
    const row = element.getBoundingClientRect();
    const items = Array.from(element.children, (child) =>
      child.getBoundingClientRect(),
    );
    return {
      whole: element.scrollWidth <= element.clientWidth,
      inside: items.every((item) => item.right <= row.right + 0.5),
      starts: items.map((item) => item.left - row.left),
      ends: items.map((item) => item.right - row.left),
    };
  });
  expect(fits.whole).toBe(true);
  expect(fits.inside).toBe(true);
  // mark, 12%, mark, 12%, mark, 91%, 70%: each item starts at the last
  // one's end plus Paseo's gap, so nothing is spread across the row.
  const gaps = fits.starts
    .slice(1)
    .map((start, index) => start - (fits.ends[index] ?? 0));
  expect(gaps).toHaveLength(6);
  [6, 12, 6, 12, 6, 8].forEach((gap, index) => {
    expect(gaps[index]).toBeCloseTo(gap, 0);
  });
});
