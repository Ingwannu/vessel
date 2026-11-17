import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { QuotaInstant } from "../../data/account-usage/clock";
import { LimitNoticeView } from "./limit-notice";

describe("the session's limit notice", () => {
  it("says the fact without a cause, the reset, waiting and Switch model", () => {
    const reset = QuotaInstant.fromIso("2026-11-17T14:00:00Z");
    const html = renderToStaticMarkup(
      <LimitNoticeView notice={{ reset }} locale="en-US" />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-label="Usage limit"');
    expect(html).toContain(
      "The saved account reports a usage limit. This does not prove why the turn stopped.",
    );
    expect(html).toMatch(/<time datetime="2026-11-17T14:00:00.000Z">/i);
    expect(html).toContain("Wait for reset");
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
    expect(html).toContain("Wait for reset");
  });
});
