import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { QuotaPercent } from "../../quota-percent";
import { QuotaMeter, type QuotaMeterTone } from "./quota-meter";

function value(percent: number): QuotaPercent {
  const result = QuotaPercent.from(percent);
  if (result === null) throw new Error("Fixture percentage must be finite");
  return result;
}

describe("read-only quota meter", () => {
  it("names its share without adding an input or keyboard controller", () => {
    const html = renderToStaticMarkup(
      <QuotaMeter
        value={value(50)}
        label="Weekly quota"
        valueText="50% left"
      />,
    );
    expect(html).toContain('role="meter"');
    expect(html).toContain('aria-label="Weekly quota"');
    expect(html).toContain('aria-valuemin="0"');
    expect(html).toContain('aria-valuemax="100"');
    expect(html).toContain('aria-valuenow="50"');
    expect(html).toContain('aria-valuetext="50% left"');
    expect(html).not.toMatch(/<(input|button)|tabindex|role="slider"/);
  });

  it("paints zero, half, full and clamped overflow without a minimum arc", () => {
    for (const [input, expected] of [
      [0, 0],
      [50, 50],
      [100, 100],
      [125, 100],
      [-20, 0],
    ] as const) {
      const html = renderToStaticMarkup(
        <QuotaMeter value={value(input)} label="Quota" />,
      );
      expect(html).toContain(`style="width:${String(expected)}%"`);
      expect(html).toContain(`aria-valuenow="${String(expected)}"`);
    }
    expect(QuotaPercent.from(Number.NaN)).toBeNull();
  });

  it("preserves typed normal tone and lets quiet override every tone", () => {
    const tones: readonly QuotaMeterTone[] = [
      "default",
      "ok",
      "warning",
      "danger",
    ];
    for (const tone of tones) {
      const normal = renderToStaticMarkup(
        <QuotaMeter value={value(50)} label="Quota" tone={tone} />,
      );
      expect(normal).toContain(
        tone === "default" ? "quota-meter-muted" : `quota-meter-${tone}`,
      );
      const quiet = renderToStaticMarkup(
        <QuotaMeter
          value={value(50)}
          label="Quota"
          tone={tone}
          emphasis="quiet"
        />,
      );
      expect(quiet).toContain("quota-meter-quiet");
      expect(quiet).not.toContain(
        `quota-meter-${tone === "default" ? "muted" : tone}`,
      );
    }
  });

  it("escapes a source label as text and keeps the caller's sizing class", () => {
    const html = renderToStaticMarkup(
      <QuotaMeter
        value={value(0)}
        label={'<quota "source">'}
        className="summary-share"
      />,
    );
    expect(html).toContain('class="quota-meter summary-share"');
    expect(html).toContain("&lt;quota &quot;source&quot;&gt;");
    expect(html).not.toContain("<quota");
  });
});
