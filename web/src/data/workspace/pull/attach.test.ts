import { describe, expect, it } from "vitest";

import type { ThreadPlace, TimelineItem } from "../../../wire/proto";
import {
  activityAttachment,
  ageEnglish,
  canAdd,
  threadAttachment,
  type About,
} from "./attach";
import { ageOf, entries, shown } from "./timeline";

const NOW = Date.parse("2026-11-19T12:00:00Z");
const HOUR = 3_600_000;

const about: About = {
  number: 42,
  title: "Fix login",
  url: "https://github.com/acme/widgets/pull/42",
  now: NOW,
};

function place(): ThreadPlace {
  return {
    thread: "PRRT_abc",
    path: "src/foo.ts",
    line: 14,
    start_line: 12,
    resolved: false,
    outdated: true,
  };
}

function comment(
  id: string,
  body: string,
  hours: number,
  thread: ThreadPlace | null,
): TimelineItem {
  return {
    kind: "comment",
    id,
    author: "octo",
    body,
    created_at: new Date(NOW - hours * HOUR).toISOString(),
    url: `https://github.com/acme/widgets/pull/42#${id}`,
    review: null,
    thread,
  };
}

describe("Add to chat's text (attach.rs)", () => {
  it("reads a comment as Paseo's", () => {
    const made = activityAttachment(about, comment("c1", " Why? ", 3, place()));
    expect(made?.kind).toBe("comment");
    expect(made?.id).toBe("42:c1");
    expect(made?.title).toBe("octo");
    expect(made?.url).toBe("https://github.com/acme/widgets/pull/42#c1");
    expect(made?.text).toBe(
      "GitHub pull request comment\n" +
        "Pull request: #42 Fix login\n" +
        "Pull request URL: https://github.com/acme/widgets/pull/42\n" +
        "URL: https://github.com/acme/widgets/pull/42#c1\n" +
        "Author: octo\n" +
        "Created: 3h ago\n" +
        "Location: src/foo.ts:12-14 · unresolved · outdated · thread PRRT_abc\n" +
        "\n" +
        "Why?",
    );
    expect(activityAttachment(about, comment("c2", "  ", 0, null))).toBeNull();
  });

  it("still sends a change request without words", () => {
    const review: TimelineItem = {
      kind: "review",
      id: "r1",
      author: null,
      body: "",
      created_at: new Date(NOW).toISOString(),
      url: null,
      state: "changes_requested",
    };
    const made = activityAttachment(about, review);
    expect(made?.kind).toBe("review");
    expect(made?.text).toContain("\nState: changes_requested\n");
    expect(
      made?.text.endsWith("Created: just now"),
      "no body, no blank line",
    ).toBe(true);
    expect(made?.title).toBe("unknown");
    expect(made?.url, "no page, no url").toBeNull();
    expect(made?.text).toContain("\nURL: \n");
  });

  it("makes a thread one conversation", () => {
    const items = [
      comment("c1", "Why?", 2, place()),
      comment("c2", "Because", 1, place()),
    ];
    const found = entries(shown(items))[0];
    if (found?.kind !== "thread") {
      throw new Error("one thread");
    }
    const attached = threadAttachment(about, found.thread);
    expect(attached?.id).toBe("42:thread:PRRT_abc");
    expect(attached?.title).toBe("src/foo.ts:12-14");
    expect(attached?.text).toBe(
      "GitHub pull request review thread\n" +
        "Pull request: #42 Fix login\n" +
        "Pull request URL: https://github.com/acme/widgets/pull/42\n" +
        "URL: https://github.com/acme/widgets/pull/42#c1\n" +
        "Location: src/foo.ts:12-14\n" +
        "Thread state: unresolved\n" +
        "Note: this thread is outdated (the code it refers to has changed)\n" +
        "\n" +
        "octo (2h ago):\nWhy?\n\n---\n\nocto (1h ago):\nBecause",
    );
  });

  it("names a discussion thread, and skips a long thread id or a silent one", () => {
    const long = "PRRT_" + "x".repeat(20);
    const where: ThreadPlace = {
      thread: long,
      resolved: true,
      outdated: false,
    };
    const made = activityAttachment(about, comment("c1", "Hm", 0, where));
    expect(made?.text).toContain("\nLocation: resolved · current\n");
    const found = entries([comment("c1", "Hm", 0, where)])[0];
    if (found?.kind !== "thread") {
      throw new Error("one thread");
    }
    const attached = threadAttachment(about, found.thread);
    expect(attached?.title).toBe("Discussion thread");
    expect(attached?.text).toContain("\nThread state: resolved\n\n");
    const silent = entries([comment("c2", " ", 0, where)])[0];
    if (silent?.kind !== "thread") {
      throw new Error("one thread");
    }
    expect(threadAttachment(about, silent.thread)).toBeNull();
  });

  it("counts a thread id's characters, not its UTF-16 units", () => {
    // 24 astral characters are 48 UTF-16 units: still short enough.
    const thread = "𝔱".repeat(24);
    const made = activityAttachment(
      about,
      comment("c1", "Hm", 0, { thread, resolved: false, outdated: false }),
    );
    expect(made?.text).toContain(`· thread ${thread}`);
  });

  it("writes ages in Paseo's English and adds only what says something", () => {
    expect(ageEnglish(ageOf(NOW - 400 * 24 * HOUR, NOW))).toBe("1y ago");
    expect(ageEnglish(ageOf(NOW - 65 * 24 * HOUR, NOW))).toBe("2mo ago");
    expect(ageEnglish(ageOf(NOW - 5 * 24 * HOUR, NOW))).toBe("5d ago");
    expect(canAdd(comment("c1", "\n\t", 0, null))).toBe(false);
    const approved: TimelineItem = {
      kind: "review",
      id: "r1",
      body: "",
      created_at: new Date(NOW).toISOString(),
      state: "approved",
    };
    expect(canAdd(approved), "an approval with no words").toBe(false);
  });
});
