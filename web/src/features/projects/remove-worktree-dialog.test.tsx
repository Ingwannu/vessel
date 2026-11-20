// The kit's Dialog draws through a portal; a layout-only fake lets the
// server render show its content and footer (as approval-dialog.test).
import type { ComponentProps } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import { fixtureId } from "../../testing/ids";
import type { Dialog as KitDialog } from "../../ui/kit/dialog";
import type { KeptReason, WorktreeInfo } from "../../wire/proto";
import { RemoveWorktreeView } from "./remove-worktree-dialog";

vi.mock("../../ui/kit/dialog", () => ({
  Dialog: ({ children, footer }: ComponentProps<typeof KitDialog>) => (
    <section>
      {children}
      <footer>{footer}</footer>
    </section>
  ),
}));

const WORKTREE: WorktreeInfo = {
  id: fixtureId("WorktreeId", "w-fix"),
  project: fixtureId("ProjectId", "p-shop"),
  path: "/work/shop-fix",
  branch: "fix-cart",
  created_branch: "fix-cart",
  created_at: "2026-11-20T00:00:00Z",
  created_by: { kind: "owner" },
  sessions: [],
  setup: { state: "none" },
};

const IN_USE: KeptReason = { kind: "in_use" };

function view(props: {
  kept: readonly KeptReason[];
  blocked: boolean;
  checking?: boolean;
  snapshotable: boolean;
  closing: number | null;
}): string {
  return renderToStaticMarkup(
    <RemoveWorktreeView
      worktree={WORKTREE}
      busy={null}
      error={null}
      onClose={vi.fn()}
      onRemove={vi.fn()}
      checking={false}
      {...props}
    />,
  );
}

function enabled(html: string, label: string): boolean {
  const button = new RegExp(`<button[^>]*>(?:(?!</button>).)*${label}`);
  const found = button.exec(html)?.[0] ?? "";
  return found !== "" && !/\sdisabled=""/.test(found);
}

describe("removing a worktree its terminals keep (ADR 0121, TRACKER 829)", () => {
  it("says Paseo's terminal warning with the count above the usual buttons", () => {
    const html = view({
      kept: [IN_USE],
      blocked: false,
      snapshotable: true,
      closing: 2,
    });
    expect(html).toContain(
      "This will close 2 terminal(s). Any running process in a closed terminal will be stopped immediately.",
    );
    // The kit's Alert, sm warning (a status, not an alert role).
    expect(html).toContain("alert-sm");
    expect(html).toContain("alert-warning");
    expect(html).toContain('role="status"');
    expect(enabled(html, "Discard and remove")).toBe(true);
    expect(enabled(html, "Keep a snapshot and remove")).toBe(true);
    expect(html).not.toContain("End the session in it");
    // The warning comes before the footer's buttons.
    expect(html.indexOf("terminal(s)")).toBeLessThan(
      html.indexOf("Discard and remove"),
    );
  });

  it("keeps today's words and no warning while a live session is in it", () => {
    const html = view({
      kept: [IN_USE],
      blocked: true,
      snapshotable: false,
      closing: null,
    });
    expect(html).toContain("End the session in it");
    expect(html).not.toContain("terminal(s)");
    expect(enabled(html, "Discard and remove")).toBe(false);
    expect(enabled(html, "Keep a snapshot and remove")).toBe(false);
  });

  it("says nothing about terminals or a session while it counts them", () => {
    const html = view({
      kept: [IN_USE],
      blocked: true,
      checking: true,
      snapshotable: false,
      closing: null,
    });
    expect(html).not.toContain("terminal(s)");
    expect(html).not.toContain("End the session in it");
    expect(enabled(html, "Discard and remove")).toBe(false);
    expect(enabled(html, "Keep a snapshot and remove")).toBe(false);
  });
});
