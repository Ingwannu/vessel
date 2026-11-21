import { describe, expect, it } from "vitest";

import type { ForgeKind, ReviewState, TimelineItem } from "../../../wire/proto";
import { brand, copied, itemActs, threadActs } from "./card-menu";

const CREATED = "2026-11-19T10:00:00Z";

function review(state: ReviewState, body: string, url: boolean): TimelineItem {
  return {
    kind: "review",
    id: "r1",
    author: "reviewer",
    body,
    created_at: CREATED,
    url: url ? "https://github.com/acme/app/pull/1#r1" : null,
    state,
  };
}

function comment(body: string, url: boolean): TimelineItem {
  return {
    kind: "comment",
    id: "c1",
    author: "author",
    body,
    created_at: CREATED,
    url: url ? "https://github.com/acme/app/pull/1#c1" : null,
    review: null,
    thread: null,
  };
}

describe("an activity card's menu (card_menu.rs)", () => {
  it("offers all three for a comment with words, in Paseo's order", () => {
    expect(itemActs(comment("Looks fine", true), true)).toEqual([
      "addToChat",
      "copy",
      "open",
    ]);
  });

  it("has nothing to add to without a composer", () => {
    expect(itemActs(comment("Looks fine", true), false)).toEqual([
      "copy",
      "open",
    ]);
  });

  it("adds and opens a change request without words, but copies nothing", () => {
    expect(itemActs(review("changes_requested", " ", true), true)).toEqual([
      "addToChat",
      "open",
    ]);
    expect(itemActs(review("approved", "", true), true)).toEqual(["open"]);
  });

  it("has no Open row for an item without a page", () => {
    expect(itemActs(comment("Looks fine", false), false)).toEqual(["copy"]);
    expect(itemActs(review("approved", "", false), true)).toEqual([]);
    const absent: TimelineItem = {
      kind: "comment",
      id: "c2",
      body: "Hi",
      created_at: CREATED,
    };
    expect(itemActs(absent, false), "an absent url is no page").toEqual([
      "copy",
    ]);
  });

  it("copies the markdown source as it came", () => {
    const source =
      "  Should this cap at **30 seconds**?\n\n```rust\nlet max_delay = 30;\n```\n";
    expect(copied(comment(source, true))).toBe(source);
    expect(copied(review("commented", "\n\t ", true))).toBeNull();
  });

  it("opens a thread's first comment from its menu", () => {
    expect(threadActs(comment("x", true))).toEqual(["open"]);
    expect(threadActs(comment("x", false))).toEqual([]);
    expect(threadActs(null)).toEqual([]);
    expect(threadActs(undefined)).toEqual([]);
  });

  it("names the forge GitHub, or by its host", () => {
    const repo = (kind: ForgeKind) => ({
      kind,
      host: "git.example.com",
      path: "acme/app",
    });
    expect(brand(repo("github"))).toBe("GitHub");
    expect(brand(repo("unknown"))).toBe("git.example.com");
  });
});
