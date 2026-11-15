// A layout-only portal fake permits SSR; the actual dialog decision and
// kit buttons remain under test, without introducing a DOM dependency.
import type { ComponentProps } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import type { ListedCommands } from "../../data/workspace/command-changes";
import { fixtureId } from "../../testing/ids";
import type { Dialog as KitDialog } from "../../ui/kit/dialog";
import { ApprovalDialog } from "./approval-dialog";

vi.mock("../../ui/kit/dialog", () => ({
  Dialog: ({ children, footer }: ComponentProps<typeof KitDialog>) => (
    <section>
      {children}
      <footer>{footer}</footer>
    </section>
  ),
}));

function listed(): ListedCommands {
  return {
    state: "listed",
    setup: [],
    teardown: [],
    approved: false,
    digest: fixtureId("CommandsDigest", "a".repeat(64)),
  };
}

function render(
  commands: ListedCommands,
  comparison: "none" | "unsupported_terminals" | "unsupported_ports" = "none",
): string {
  return renderToStaticMarkup(
    <ApprovalDialog
      commands={commands}
      compared={{ kind: comparison }}
      where="in this worktree"
      busy={false}
      onApprove={vi.fn()}
      onClose={vi.fn()}
    />,
  );
}

describe("terminal approval refusal", () => {
  it("renders a plain refusal and no approve button for current terminals", () => {
    const html = render({ ...listed(), terminals: [{ command: "echo next" }] });
    expect(html).toContain("Approval is unavailable here");
    expect(html).not.toContain("Approve and run");
    expect(html).toContain("Cancel");
  });

  it("refuses terminal-bearing history even with a stale comparison prop", () => {
    const html = render({
      ...listed(),
      last_approved: {
        commands: {
          setup: [],
          teardown: [],
          terminals: [{ command: "echo before" }],
        },
        digest: fixtureId("CommandsDigest", "b".repeat(64)),
        approved_at: "2026-11-12T09:30:00Z",
      },
    });
    expect(html).toContain("Approval is unavailable here");
    expect(html).not.toContain("Approve and run");
  });

  it("does not approve when the folder comparison cannot be displayed", () => {
    const html = render(listed(), "unsupported_terminals");
    expect(html).toContain("Approval is unavailable here");
    expect(html).not.toContain("Approve and run");
  });

  it("still renders approval for a legacy set", () => {
    expect(render(listed())).toContain("Approve and run");
  });
});

describe("declared port approval refusal", () => {
  const service = { name: "dev", kind: "service", command: "serve" } as const;
  it("shows a refusal and no approve button for a current declaration", () => {
    const html = render({ ...listed(), scripts: [{ ...service, port: 3210 }] });
    expect(html).toContain("service port declarations");
    expect(html).not.toContain("Approve and run");
  });
  it("refuses a declaration removed from the previous approval", () => {
    const html = render({
      ...listed(),
      scripts: [service],
      last_approved: {
        commands: {
          setup: [],
          teardown: [],
          scripts: [{ ...service, port: 3210 }],
        },
        digest: listed().digest,
        approved_at: "2026-11-12T09:30:00Z",
      },
    });
    expect(html).toContain("service port declarations");
    expect(html).not.toContain("Approve and run");
  });
  it("honours a folder comparison refusal", () => {
    const html = render(listed(), "unsupported_ports");
    expect(html).toContain("service port declarations");
    expect(html).not.toContain("Approve and run");
  });
});

describe("allocation approval refusal", () => {
  const service_ports = {
    range: "7000-7009",
    port_script: "tools/port-approval",
  };
  it("renders a plain refusal and no approve button for a current block", () => {
    const html = render({ ...listed(), service_ports });
    expect(html).toContain("allocation settings");
    expect(html).not.toContain("Approve and run");
  });
  it("refuses removal from the previous approval", () => {
    const html = render({
      ...listed(),
      last_approved: {
        commands: { setup: [], teardown: [], service_ports },
        digest: listed().digest,
        approved_at: "2026-11-12T09:30:00Z",
      },
    });
    expect(html).toContain("allocation settings");
    expect(html).not.toContain("Approve and run");
  });
  it("honors a folder comparison refusal", () => {
    const html = render(listed(), "unsupported_ports");
    expect(html).toContain("allocation settings");
    expect(html).not.toContain("Approve and run");
  });
});
