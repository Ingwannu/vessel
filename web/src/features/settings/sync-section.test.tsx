import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { KeptOwnEdit } from "./sync-section";

describe("a machine that kept its own edit (TRACKER 820)", () => {
  it("is a warning Alert in the flow naming the files and the incoming copy", () => {
    const html = renderToStaticMarkup(
      <KeptOwnEdit label="laptop" files={["CLAUDE.md", "skills/review.md"]} />,
    );
    expect(html).toContain('role="status"');
    expect(html).toContain("alert-sm");
    expect(html).toContain("alert-warning");
    expect(html).toContain(
      "laptop kept its own edit of CLAUDE.md, skills/review.md.",
    );
    expect(html).toContain("<code>.vessel-incoming</code>");
  });
});
