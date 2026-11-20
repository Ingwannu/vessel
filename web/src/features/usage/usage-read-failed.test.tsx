import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { UsageReadFailed } from "./usage-read-failed";

describe("usage that could not be read (TRACKER 820)", () => {
  it("is Paseo's error Alert in the flow: what failed, the reason, Try again", () => {
    const html = renderToStaticMarkup(
      <UsageReadFailed
        message="The node refused: no route"
        onRetry={vi.fn()}
      />,
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain("alert-sm");
    expect(html).toContain("alert-error");
    expect(html).toContain("Unable to load usage");
    expect(html).toContain("The node refused: no route");
    expect(html).toMatch(/<button[^>]*>Try again<\/button>/);
    expect(html).not.toContain('class="banner');
  });
});
