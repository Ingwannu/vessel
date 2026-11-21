// A pull request's activity as Paseo's pane lists it
// (`pull-request-panel/timeline.ts`, `activity-state.ts`,
// `activity-location.ts`, and `data.ts`'s mapping and `formatAge`), from
// `pr.timeline`'s items (ADR 0088 part 3), which come oldest first.
// Mirrors the native `view/src/workspace/pull/timeline.rs`.
//
// [Decision Log]
// - Purpose: the pane's cards (comments, reviews with their threads,
//   threads), what is folded at first, the header's counts and each
//   card's age, decided once and tested outside the screen.
// - Choice: Paseo's steps in its order: drop what has nothing to say,
//   gather a thread's comments where its first one stands, move a thread
//   under the review its first comment belongs to; a resolved or outdated
//   thread starts folded, nothing else does. `now` is a parameter.
// - Differences: Paseo writes ages in English only ("3h ago"); here the
//   step is a value and the language words it (timeline-words.ts). A
//   thread's comments are typed non-empty, so its root always exists.
//   "Nothing to say" trims Unicode White_Space as Rust's `str::trim`
//   does, not `String.prototype.trim` (which also strips U+FEFF and keeps
//   U+0085). A creation time that does not parse reads as "just now".

import type {
  ReviewState,
  ThreadPlace,
  TimelineItem,
} from "../../../wire/proto";
import { instant } from "./time";

/** A review, among the items. */
export type ReviewItem = Extract<TimelineItem, { kind: "review" }>;
/** A comment, in the conversation or in a review thread. */
export type CommentItem = Extract<TimelineItem, { kind: "comment" }>;

/** Rust's `str::trim`: Unicode White_Space at both ends. */
const EDGE_SPACE = /^\p{White_Space}+|\p{White_Space}+$/gu;

/** `text` without the white space around it. */
export function trimmed(text: string): string {
  return text.replace(EDGE_SPACE, "");
}

/** A review thread: its place (its first comment's), and its comments
 * oldest first; the first is its root. */
export interface Thread {
  readonly place: ThreadPlace;
  readonly comments: readonly [CommentItem, ...CommentItem[]];
}

/** A thread's key among the entries (Paseo's `thread:<id>`). */
export function threadKey(thread: Thread): string {
  return `thread:${thread.place.thread}`;
}

/** Paseo's `shouldCollapseByDefault`: resolved or outdated. */
export function threadFoldedAtFirst(thread: Thread): boolean {
  return thread.place.resolved || thread.place.outdated;
}

/** One card of the activity: a conversation comment or a review with no
 * thread of its own; a thread whose review is not listed; a review and
 * the threads its comments started, in order. */
export type Entry =
  | { readonly kind: "single"; readonly item: TimelineItem }
  | { readonly kind: "thread"; readonly thread: Thread }
  | {
      readonly kind: "review";
      readonly review: ReviewItem;
      readonly threads: readonly Thread[];
    };

/** An entry's key, for what a person folded (Paseo's entry id). */
export function entryKey(entry: Entry): string {
  switch (entry.kind) {
    case "single":
      return entry.item.id;
    case "review":
      return entry.review.id;
    case "thread":
      return threadKey(entry.thread);
  }
}

/** Whether an entry starts folded: only a thread can. */
export function entryFoldedAtFirst(entry: Entry): boolean {
  return entry.kind === "thread" && threadFoldedAtFirst(entry.thread);
}

/** Whether an item has something to say: Paseo drops an empty comment,
 * and a review that only commented with no words. */
export function saysSomething(item: TimelineItem): boolean {
  if (trimmed(item.body) !== "") {
    return true;
  }
  return (
    item.kind === "review" &&
    (item.state === "approved" || item.state === "changes_requested")
  );
}

/** The items worth a card, in their order. */
export function shown(items: readonly TimelineItem[]): TimelineItem[] {
  return items.filter(saysSomething);
}

/** A thread while it is gathered. */
interface Gathering {
  readonly place: ThreadPlace;
  readonly comments: [CommentItem, ...CommentItem[]];
}

/** An item standing alone, or a thread gathered where it started. */
type Grouped =
  | { readonly kind: "single"; readonly item: TimelineItem }
  | { readonly kind: "thread"; readonly thread: Gathering };

/** Paseo's `groupThreads`: a thread's comments where its first stands. */
function gather(items: readonly TimelineItem[]): Grouped[] {
  const grouped: Grouped[] = [];
  const threads = new Map<string, Gathering>();
  for (const item of items) {
    const place = item.kind === "comment" ? (item.thread ?? null) : null;
    if (item.kind === "review" || place === null) {
      grouped.push({ kind: "single", item });
      continue;
    }
    const existing = threads.get(place.thread);
    if (existing === undefined) {
      const thread: Gathering = { place, comments: [item] };
      threads.set(place.thread, thread);
      grouped.push({ kind: "thread", thread });
    } else {
      existing.comments.push(item);
    }
  }
  return grouped;
}

/** Paseo's `groupThreads` then `nestThreadsUnderReviews`: a thread whose
 * root belongs to a listed review moves under it; one whose review is not
 * listed stays where it started. */
export function entries(items: readonly TimelineItem[]): Entry[] {
  const grouped = gather(items);
  const reviews = new Set(
    grouped.flatMap((entry) =>
      entry.kind === "single" && entry.item.kind === "review"
        ? [entry.item.id]
        : [],
    ),
  );
  const nested = new Map<string, Thread[]>();
  const top = grouped.filter((entry) => {
    if (entry.kind === "single") {
      return true;
    }
    const review = entry.thread.comments[0].review ?? null;
    if (review === null || !reviews.has(review)) {
      return true;
    }
    const under = nested.get(review);
    if (under === undefined) {
      nested.set(review, [entry.thread]);
    } else {
      under.push(entry.thread);
    }
    return false;
  });
  return top.map((entry): Entry => {
    if (entry.kind === "thread") {
      return { kind: "thread", thread: entry.thread };
    }
    const item = entry.item;
    const threads = item.kind === "review" ? nested.get(item.id) : undefined;
    if (item.kind !== "review" || threads === undefined) {
      return { kind: "single", item };
    }
    // The first review with this id takes its threads, as the native
    // rule's drain does; a second one with the same id stays single.
    nested.delete(item.id);
    return { kind: "review", review: item, threads };
  });
}

/** The Activity header's pills (Paseo counts the flat list). */
export interface ActivityCounts {
  /** Approving reviews. */
  readonly approvals: number;
  /** Reviews that requested changes. */
  readonly changes: number;
  /** Comments, thread replies included, and reviews that commented. */
  readonly comments: number;
}

/** What a review's state adds to the counts. */
function countOf(state: ReviewState): keyof ActivityCounts {
  switch (state) {
    case "approved":
      return "approvals";
    case "changes_requested":
      return "changes";
    case "commented":
    case "unknown":
      return "comments";
  }
}

/** The counts of `items` (the shown ones). */
export function activityCounts(items: readonly TimelineItem[]): ActivityCounts {
  const counts = { approvals: 0, changes: 0, comments: 0 };
  for (const item of items) {
    counts[item.kind === "review" ? countOf(item.state) : "comments"] += 1;
  }
  return counts;
}

/** How long ago, in Paseo's steps (floored): under a minute; minutes under
 * an hour; hours under a day; days under thirty; months of thirty days
 * under a year; years of 365 days. */
export type Age =
  | { readonly step: "now" }
  | {
      readonly step: "minutes" | "hours" | "days" | "months" | "years";
      readonly count: number;
    };

const MINUTE = 60_000;

/** Paseo's `formatAge` of `at` seen from `now`, both in milliseconds
 * since the epoch; a time after `now`, or no time, is "just now". */
export function ageOf(at: number, now: number): Age {
  const elapsed = now - at;
  if (Number.isNaN(elapsed) || elapsed < MINUTE) {
    return { step: "now" };
  }
  const minutes = Math.floor(elapsed / MINUTE);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);
  if (minutes < 60) {
    return { step: "minutes", count: minutes };
  }
  if (hours < 24) {
    return { step: "hours", count: hours };
  }
  if (days < 30) {
    return { step: "days", count: days };
  }
  if (days < 365) {
    return { step: "months", count: Math.floor(days / 30) };
  }
  return { step: "years", count: Math.floor(days / 365) };
}

/** An item's age at `now`. */
export function itemAge(item: TimelineItem, now: number): Age {
  return ageOf(instant(item.created_at) ?? Number.NaN, now);
}

/** A thread's place as its header names it (Paseo's
 * `formatPullRequestThreadPath`): `path:start-line`, `path:line` or the
 * path; null without a path ("Discussion thread"). */
export function threadPath(place: ThreadPlace): string | null {
  const path = place.path ?? null;
  if (path === null) {
    return null;
  }
  const line = place.line ?? null;
  if (line === null) {
    return path;
  }
  const start = place.start_line ?? null;
  return start === null ? `${path}:${line}` : `${path}:${start}-${line}`;
}
