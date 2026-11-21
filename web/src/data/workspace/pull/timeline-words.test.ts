import { describe, expect, it } from "vitest";

import type { ReviewState, TimelineItem } from "../../../wire/proto";
import { ageOf } from "./timeline";
import { ageWords, verbWords } from "./timeline-words";
import { pullWords } from "./words";

const en = pullWords("en");
const ko = pullWords("ko");

describe("the activity's words (timeline_words.rs)", () => {
  it("reads ages as Paseo's, and whole in Korean", () => {
    const now = Date.parse("2026-11-19T10:00:00Z");
    const ago = (minutes: number) => ageOf(now - minutes * 60_000, now);
    expect(ageWords({ step: "now" }, en)).toBe("just now");
    expect(ageWords(ago(3), en)).toBe("3m ago");
    expect(ageWords(ago(120), en)).toBe("2h ago");
    expect(ageWords(ago(65 * 24 * 60), en)).toBe("2mo ago");
    expect(ageWords(ago(120), ko)).toBe("2시간 전");
    expect(ageWords({ step: "now" }, ko)).toBe("방금");
  });

  it("names what each item's author did", () => {
    const item = (state: ReviewState): TimelineItem => ({
      kind: "review",
      id: "r1",
      body: "",
      created_at: "2026-11-19T10:00:00Z",
      state,
    });
    expect(verbWords(item("approved"), en)).toBe("approved");
    expect(verbWords(item("changes_requested"), en)).toBe("requested changes");
    expect(verbWords(item("commented"), en)).toBe("reviewed");
    expect(verbWords(item("unknown"), ko)).toBe("리뷰함");
    const comment: TimelineItem = {
      kind: "comment",
      id: "c1",
      body: "Hi",
      created_at: "2026-11-19T10:00:00Z",
    };
    expect(verbWords(comment, en)).toBe("commented");
  });
});
