import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { About } from "../../../data/workspace/pull/attach";
import { pullWords } from "../../../data/workspace/pull/words";
import type { Checks, TimelineItem } from "../../../wire/proto";
import { PullActivity } from "./pull-activity";
import { PullChecks } from "./pull-checks";
import { PullTooOld } from "./pull-pane";

const NOW = Date.parse("2026-11-20T12:00:00Z");
const words = pullWords("en-US");
const about: About = {
  number: 128,
  title: "Retry failed uploads with backoff",
  url: "https://github.com/acme-labs/fleet-agent/pull/128",
  now: NOW,
};
const base = "https://github.com/acme-labs/fleet-agent";

const checks: Checks = {
  state: "failing",
  passed: 2,
  failed: 1,
  pending: 1,
  truncated: false,
  failing: [],
  runs: [
    {
      name: "build",
      state: "passed",
      started_at: "2026-11-20T11:50:00Z",
      completed_at: "2026-11-20T11:54:00Z",
    },
    { name: "docs", state: "skipped" },
    { name: "e2e", state: "pending", started_at: "2026-11-20T11:58:00Z" },
    {
      name: "test",
      state: "failed",
      run: 106,
      url: `${base}/runs/106`,
      workflow: "CI",
      started_at: "2026-11-20T11:50:00Z",
      completed_at: "2026-11-20T11:57:05Z",
    },
  ],
};

describe("the pull request pane's sections (TRACKER 430)", () => {
  it("leads the checks with the worst outcome and groups them in Paseo's order", () => {
    const html = renderToStaticMarkup(
      <PullChecks
        checks={checks}
        words={words}
        now={NOW}
        about={about}
        report={() => Promise.reject(new Error("not in this test"))}
        add={() => undefined}
      />,
    );
    const failing = html.indexOf("failing");
    expect(failing).toBeGreaterThan(-1);
    // One segment per presentation that has checks.
    expect(html.match(/pull-ring-part/g)?.length).toBe(4);
    expect(
      html.indexOf('href="https://github.com/acme-labs/fleet-agent/runs/106"'),
    ).toBeGreaterThan(-1);
    // Only the failed check offers its logs.
    expect(html.match(/Add to chat/g)?.length).toBe(1);
  });

  it("threads comments under their review and folds a resolved thread", () => {
    const items: TimelineItem[] = [
      {
        kind: "comment",
        id: "IC_1",
        author: "jdoe",
        body: "This retries uploads.",
        created_at: "2026-11-19T12:00:00Z",
        url: `${base}/pull/128#c1`,
      },
      {
        kind: "review",
        id: "PRR_1",
        author: "mkim",
        body: "The cap needs a test.",
        created_at: "2026-11-20T09:00:00Z",
        url: `${base}/pull/128#r1`,
        state: "changes_requested",
      },
      {
        kind: "comment",
        id: "RC_1",
        author: "mkim",
        body: "Should this cap at 30 seconds? `max_delay` is unbounded.",
        created_at: "2026-11-20T09:00:00Z",
        url: `${base}/pull/128#d1`,
        review: "PRR_1",
        thread: {
          thread: "PRRT_open1",
          path: "src/upload/retry.rs",
          line: 48,
          resolved: false,
          outdated: false,
        },
      },
      {
        kind: "comment",
        id: "RC_3",
        author: "mkim",
        body: "Nit: log the attempt.",
        created_at: "2026-11-19T12:00:00Z",
        url: `${base}/pull/128#d3`,
        thread: {
          thread: "PRRT_done1",
          path: "src/upload/client.rs",
          line: 12,
          resolved: true,
          outdated: false,
        },
      },
    ];
    const html = renderToStaticMarkup(
      <PullActivity
        items={items}
        words={words}
        now={NOW}
        forge="GitHub"
        about={about}
        add={() => undefined}
      />,
    );
    expect(html).toContain("Add all to chat");
    expect(html).toContain("This retries uploads.");
    expect(html).toContain("Should this cap at 30 seconds?");
    // Bodies are markdown: the inline code is code.
    expect(html).toContain("<code>max_delay</code>");
    // The resolved thread starts folded: its place shows, its words do not.
    expect(html).toContain("src/upload/client.rs");
    expect(html).not.toContain("Nit: log the attempt.");
  });

  it("says why a machine before protocol 39 shows no pull request", () => {
    const html = renderToStaticMarkup(<PullTooOld language="en-US" />);
    expect(html).toContain("Update Vessel on this machine");
    const ko = renderToStaticMarkup(<PullTooOld language="ko-KR" />);
    expect(ko).toContain("이 머신의 Vessel을 업데이트하세요");
  });
});
