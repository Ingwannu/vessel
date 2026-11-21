import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { SharedPull } from "../../data/workspace/diff-stat";
import { WorkspaceMeta } from "./workspace-meta";

const PULL: SharedPull = {
  kind: "github",
  host: "github.com",
  path: "acme-labs/fleet-agent",
  number: 128,
  state: "open",
  draft: false,
  checks: "pending",
  counts: { passed: 4, failed: 0, pending: 2 },
  url: "https://github.com/acme-labs/fleet-agent/pull/128",
};

describe("the workspace line's markup (TRACKER 426)", () => {
  it("links the pull request and names the checks by their outcome", () => {
    const html = renderToStaticMarkup(
      <WorkspaceMeta machine="workstation" pull={PULL} chips={[]} />,
    );
    expect(html).toContain(
      'href="https://github.com/acme-labs/fleet-agent/pull/128"',
    );
    expect(html).toContain('aria-label="Pull request #128"');
    expect(html).toContain('role="img" aria-label="Checks running"');
    // Half the checks are done: the half wedge, from twelve to six.
    expect(html).toContain('d="M12 12V5A7 7 0 0 1 12 19Z"');
    expect(html.indexOf("workstation")).toBeLessThan(html.indexOf("128"));
  });

  it("draws a check for passed, a cross for failed, and the merged word", () => {
    const passed = renderToStaticMarkup(
      <WorkspaceMeta
        machine={null}
        pull={{ ...PULL, checks: "passing", state: "merged" }}
        chips={[]}
      />,
    );
    expect(passed).toContain('d="M8 12.5 L11 15.5 L16.5 9"');
    expect(passed).toContain("128 merged");
    const failed = renderToStaticMarkup(
      <WorkspaceMeta
        machine={null}
        pull={{ ...PULL, checks: "failing" }}
        chips={[]}
      />,
    );
    expect(failed).toContain('aria-label="Checks failed"');
    expect(failed).toContain('d="M8.5 8.5 L15.5 15.5 M15.5 8.5 L8.5 15.5"');
  });

  it("draws nothing without anything to say", () => {
    expect(
      renderToStaticMarkup(
        <WorkspaceMeta machine={null} pull={null} chips={[]} />,
      ),
    ).toBe("");
  });
});
