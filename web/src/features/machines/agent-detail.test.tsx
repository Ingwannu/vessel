import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { OffersFailure } from "./agent-detail";

describe("an agent's offers that could not be read (TRACKER 820)", () => {
  it("is an error Alert: what did not happen over the node's reason", () => {
    const html = renderToStaticMarkup(
      <OffersFailure
        name="Claude Code"
        refused={null}
        reason="The agent exited. Check that it is installed."
      />,
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain("alert-sm");
    expect(html).toContain("Claude Code did not say what it offers");
    expect(html).toContain("The agent exited. Check that it is installed.");
  });

  it("says a refused start in one line, with no title", () => {
    const html = renderToStaticMarkup(
      <OffersFailure
        name="Claude Code"
        refused="Claude Code is starting a session; ask again when it is ready."
        reason="unused"
      />,
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain(
      "Claude Code is starting a session; ask again when it is ready.",
    );
    expect(html).not.toContain("alert-title");
    expect(html).not.toContain("unused");
  });
});
