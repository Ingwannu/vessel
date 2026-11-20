import { describe, expect, it } from "vitest";

import type { CheckReport, CheckRun } from "../../../wire/proto";
import type { About } from "./attach";
import { canAddCheck, checkAttachment } from "./attach-check";

const about: About = {
  number: 42,
  title: "Fix login",
  url: "https://github.com/acme/widgets/pull/42",
  now: Date.parse("2026-11-19T12:00:00Z"),
};

const HEAD =
  "GitHub pull request check\n" +
  "Pull request: #42 Fix login\n" +
  "Pull request URL: https://github.com/acme/widgets/pull/42\n";

function report(over: Partial<CheckReport> = {}): CheckReport {
  return {
    run: 7,
    name: "build",
    state: "failed",
    details_url: "https://github.com/acme/widgets/actions/runs/1",
    output: { title: "1 error" },
    annotations: [
      {
        path: "src/lib.rs",
        start_line: 3,
        end_line: 4,
        level: "failure",
        message: "expected `;`",
      },
    ],
    failed_jobs: [
      {
        job: 9,
        name: "test",
        state: "failed",
        log: { text: "error: one\nerror: two", cut: true },
      },
    ],
    truncated: false,
    ...over,
  };
}

describe("a failed check's text (attach.rs)", () => {
  it("carries its report and logs", () => {
    const run: CheckRun = { name: "build", state: "failed", run: 7 };
    const made = checkAttachment(about, run, report());
    expect(made.kind).toBe("check");
    expect(made.id).toBe("42:check-run:7");
    expect(made.title).toBe("build");
    expect(made.url).toBe("https://github.com/acme/widgets/actions/runs/1");
    expect(made.text).toBe(
      HEAD +
        "Check: build\n" +
        "Status: failure\n" +
        "Conclusion: failure\n" +
        "Check URL: https://github.com/acme/widgets/pull/42\n" +
        "Details URL: https://github.com/acme/widgets/actions/runs/1\n" +
        "Output title: 1 error\n" +
        "\n" +
        "Annotations:\n" +
        "- src/lib.rs:3-4 failure: expected `;`\n" +
        "\n" +
        "Failed jobs:\n" +
        "- test: failure\n  ```\n  error: one\n  error: two\n  ```\n" +
        "  Log tail truncated to the latest capped lines.",
    );
    const bare = checkAttachment(about, run, null);
    expect(
      bare.text.endsWith("Check URL: https://github.com/acme/widgets/pull/42"),
    ).toBe(true);
    expect(bare.url, "the pull request's page stands in").toBe(about.url);
  });

  it("keys a check without a run by its name, and prefers its own page", () => {
    const run: CheckRun = {
      name: "lint",
      state: "failed",
      url: "https://github.com/acme/widgets/runs/3",
    };
    const made = checkAttachment(about, run, null);
    expect(made.id).toBe("42:check:lint");
    expect(made.url).toBe("https://github.com/acme/widgets/runs/3");
    expect(made.text).toContain(
      "\nCheck URL: https://github.com/acme/widgets/runs/3",
    );
  });

  it("writes every part of a report in Paseo's order", () => {
    const run: CheckRun = { name: "build", state: "failed", run: 7 };
    const full = report({
      state: "pending",
      details_url: null,
      url: "https://github.com/acme/widgets/runs/7",
      output: { title: "T", summary: "S", text: "line 1\nline 2" },
      annotations: [
        {
          path: "a.rs",
          start_line: 1,
          end_line: 1,
          level: "unknown",
          message: "",
        },
      ],
      failed_jobs: [
        {
          job: 9,
          name: "unit",
          state: "cancelled",
          url: "https://github.com/acme/widgets/jobs/9",
        },
      ],
      truncated: true,
    });
    expect(checkAttachment(about, run, full).text).toBe(
      HEAD +
        "Check: build\n" +
        "Status: failure\n" +
        "Check URL: https://github.com/acme/widgets/pull/42\n" +
        "Details URL: https://github.com/acme/widgets/runs/7\n" +
        "Output title: T\n" +
        "Output summary: S\n" +
        "Output text:\nline 1\nline 2\n" +
        "\n" +
        "Annotations:\n" +
        "- a.rs:1-1\n" +
        "\n" +
        "Failed jobs:\n" +
        "- unit: cancelled\n" +
        "  https://github.com/acme/widgets/jobs/9\n" +
        "\n" +
        "Note: Check details were truncated by GitHub/API or local caps.",
    );
  });

  it("offers a check's logs only when it failed", () => {
    expect(canAddCheck({ name: "a", state: "failed" })).toBe(true);
    expect(canAddCheck({ name: "a", state: "cancelled" })).toBe(false);
    expect(canAddCheck({ name: "a", state: "pending" })).toBe(false);
  });
});
