import { describe, expect, it } from "vitest";

import { withPreviewCsp } from "./html-preview";

// Paseo's `html-preview-csp.test.ts` cases.
describe("an HTML preview's document", () => {
  it("places the policy before the complete source document", () => {
    const source =
      "<!DOCTYPE html><html><head><script>1</script></head></html>";
    const output = withPreviewCsp(source);
    expect(output).toMatch(
      /^<!doctype html><meta http-equiv="Content-Security-Policy"/u,
    );
    expect(output.endsWith(source)).toBe(true);
    expect(output.indexOf("Content-Security-Policy")).toBeLessThan(
      output.indexOf("<script>"),
    );
  });

  it("drops a leading BOM before appending the source", () => {
    const output = withPreviewCsp("\uFEFF<h1>Plan</h1>");
    expect(output.includes("\uFEFF")).toBe(false);
    expect(output.endsWith("<h1>Plan</h1>")).toBe(true);
  });

  it("refuses remote resources", () => {
    const document = withPreviewCsp("<p>x</p>");
    for (const directive of [
      "default-src 'none'",
      "connect-src 'none'",
      "form-action 'none'",
      "base-uri 'none'",
      "frame-src 'none'",
      "object-src 'none'",
      "style-src 'unsafe-inline'",
    ]) {
      expect(document).toContain(directive);
    }
    expect(document).not.toMatch(/script-src[^;]*https:/u);
  });
});
