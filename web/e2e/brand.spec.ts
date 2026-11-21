// The owner's mark (docs/design/brand/, 2026-11-13) on the page: the
// sidebar's logo in the text's colour, and the page's icon, the mark in
// white on a tile of the brand colour, both made by the build from the
// brand's file (vite-mark.ts).

import { expect, test } from "@playwright/test";

import { openUi } from "./open";

test("the owner's mark is the sidebar's logo and the page's icon", async ({
  page,
}) => {
  const origin = await openUi(page);
  const brand = page
    .getByRole("navigation", { name: "Sidebar" })
    .getByRole("link", { name: "Vessel" });
  await expect(brand.locator("svg.mark path")).toHaveCount(2);
  // The icon follows the sessions (ADR 0089, "The page's badge and
  // favicon"): plain, running or attention, by what the fixture machines'
  // sessions are doing.
  await expect(page.locator('link[rel="icon"]')).toHaveAttribute(
    "href",
    /^\/favicon(-running|-attention)?\.svg$/,
  );
  const icon = await page.request.get(`${origin}/favicon.svg`);
  expect(icon.status()).toBe(200);
  expect(icon.headers()["content-type"]).toContain("image/svg+xml");
  const body = await icon.text();
  expect(body).toContain('fill="#1a5868"');
  expect(body).toContain('fill="#fff"');
  // Paseo's running and attention favicons: the same tile and one dot.
  for (const [state, dot] of [
    ["running", "#3b82f6"],
    ["attention", "#22c55e"],
  ] as const) {
    const variant = await page.request.get(`${origin}/favicon-${state}.svg`);
    expect(variant.status()).toBe(200);
    const drawn = await variant.text();
    expect(drawn).toContain('fill="#1a5868"');
    expect(drawn).toContain(
      `<circle cx="52.11" cy="52.11" r="11.89" fill="${dot}"/>`,
    );
  }
});
