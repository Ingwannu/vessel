import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { QuotaInstant } from "../../data/account-usage/clock";
import { LimitNoticeView } from "./limit-notice";

describe("the session's limit notice", () => {
  it("says the fact without a cause, the reset on one line, and Switch model", () => {
    const reset = QuotaInstant.fromIso("2026-11-17T14:00:00Z");
    const html = renderToStaticMarkup(
      <LimitNoticeView notice={{ reset }} locale="en-US" />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-label="Usage limit"');
    expect(html).toContain("This account has reached its usage limit.");
    expect(html).not.toMatch(/turn stopped|because/);
    expect(html).toMatch(
      /Resets <time datetime="2026-11-17T14:00:00.000Z">\w{3}, Nov 1[78], \d{1,2}:00\s[AP]M<\/time>/i,
    );
    expect(html).not.toContain("Wait for reset");
    expect(html).toContain(
      "No prompt will be sent automatically. Continue when the account is ready.",
    );
    expect(html).toContain("Switch model");
  });

  it("leaves the reset out when none is ahead", () => {
    const html = renderToStaticMarkup(
      <LimitNoticeView notice={{ reset: null }} locale="en-US" />,
    );
    expect(html).not.toContain("<time");
    expect(html).not.toContain("Resets");
    expect(html).toContain("No prompt will be sent automatically.");
  });
});
