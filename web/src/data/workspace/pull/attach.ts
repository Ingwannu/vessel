// What "Add to chat" hands the composer from the pull request pane:
// Paseo's `pull-request-panel/context-attachment.ts` (pinned 0.15.4), the
// same headings and lines, for a comment, a review and a thread (ADR 0088
// part 3); a failed check with its logs is attach-check.ts (part 4).
// Mirrors the native `view/src/workspace/pull/attach.rs`.
//
// [Decision Log]
// - Purpose: the agent reads what a reviewer said, or why a check failed,
//   without the person copying it.
// - Constraint: the text goes to the agent, not to the person; Paseo
//   writes it in English with its own ages ("3h ago").
// - Choice: Paseo's text line for line, GitHub only; ages in Paseo's
//   English whatever the page's language. The native tests pin every
//   line; attach.test.ts holds the same strings.
// - Differences: none in the text. `now` is a parameter of `About`.

import type { ThreadPlace, TimelineItem } from "../../../wire/proto";
import {
  itemAge,
  threadKey,
  threadPath,
  trimmed,
  type Age,
  type Thread,
} from "./timeline";

/** Which tray item an attachment is (Paseo's `forge.change_request_*`):
 * a comment or a whole thread, a review, a check and its logs. */
export type AttachKind = "comment" | "review" | "check";

/** One attachment. */
export interface Attachment {
  readonly kind: AttachKind;
  /** Its identity in the tray (a second add replaces the first). */
  readonly id: string;
  /** Its title: the author, the thread's place, or the check. */
  readonly title: string;
  /** The text the agent reads. */
  readonly text: string;
  /** Its page, when there is one. */
  readonly url: string | null;
}

/** The pull request an attachment is about. */
export interface About {
  readonly number: number;
  readonly title: string;
  /** Its page. */
  readonly url: string;
  /** When the attachment is made, in milliseconds since the epoch. */
  readonly now: number;
}

/** Paseo's heading and the two lines after it. */
export function aboutLines(about: About, heading: string): string[] {
  return [
    heading,
    `Pull request: #${about.number} ${about.title}`,
    `Pull request URL: ${about.url}`,
  ];
}

/** Paseo's `formatAge`, in its English. */
export function ageEnglish(age: Age): string {
  if (age.step === "now") {
    return "just now";
  }
  switch (age.step) {
    case "minutes":
      return `${age.count}m ago`;
    case "hours":
      return `${age.count}h ago`;
    case "days":
      return `${age.count}d ago`;
    case "months":
      return `${age.count}mo ago`;
    case "years":
      return `${age.count}y ago`;
  }
}

/** An item's author ("unknown" without one), its words trimmed, and its
 * page ("" without one, as Paseo's line prints it). */
function parts(item: TimelineItem): {
  author: string;
  body: string;
  url: string;
} {
  return {
    author: item.author ?? "unknown",
    body: trimmed(item.body),
    url: item.url ?? "",
  };
}

/** Paseo's `canAddPullRequestActivityToChat`: words, or a change
 * request. */
export function canAdd(item: TimelineItem): boolean {
  return (
    parts(item).body !== "" ||
    (item.kind === "review" && item.state === "changes_requested")
  );
}

/** A review state as Paseo's wire names it. */
function reviewState(item: Extract<TimelineItem, { kind: "review" }>): string {
  switch (item.state) {
    case "approved":
      return "approved";
    case "changes_requested":
      return "changes_requested";
    case "commented":
    case "unknown":
      return "commented";
  }
}

/** A thread id the location line names: at most 24 characters, counted
 * as code points (the `u` flag), as the native rule counts `chars()`, not
 * UTF-16 units. */
const SHORT_THREAD = /^.{0,24}$/su;

/** Paseo's `formatPullRequestActivityLocation`: the place, then resolved
 * or not, outdated or current, and a short thread id. */
function location(place: ThreadPlace): string {
  const said: string[] = [];
  const path = threadPath(place);
  if (path !== null) {
    said.push(path);
  }
  said.push(place.resolved ? "resolved" : "unresolved");
  said.push(place.outdated ? "outdated" : "current");
  if (SHORT_THREAD.test(place.thread)) {
    said.push(`thread ${place.thread}`);
  }
  return said.join(" · ");
}

/** A comment or a review (Paseo's activity text); null for one with
 * nothing to add. */
export function activityAttachment(
  about: About,
  item: TimelineItem,
): Attachment | null {
  if (!canAdd(item)) {
    return null;
  }
  const { author, body, url } = parts(item);
  const review = item.kind === "review";
  const lines = aboutLines(
    about,
    review ? "GitHub pull request review" : "GitHub pull request comment",
  );
  lines.push(`URL: ${url}`, `Author: ${author}`);
  if (item.kind === "review") {
    lines.push(`State: ${reviewState(item)}`);
  }
  lines.push(`Created: ${ageEnglish(itemAge(item, about.now))}`);
  const place = item.kind === "comment" ? (item.thread ?? null) : null;
  if (place !== null) {
    lines.push(`Location: ${location(place)}`);
  }
  if (body !== "") {
    lines.push("", body);
  }
  return {
    kind: review ? "review" : "comment",
    id: `${about.number}:${item.id}`,
    title: author,
    text: lines.join("\n"),
    url: url === "" ? null : url,
  };
}

/** A whole thread, its comments with words in order; null when none has
 * any. */
export function threadAttachment(
  about: About,
  thread: Thread,
): Attachment | null {
  const said = thread.comments.flatMap((item) => {
    const { author, body } = parts(item);
    if (body === "") {
      return [];
    }
    return [`${author} (${ageEnglish(itemAge(item, about.now))}):\n${body}`];
  });
  if (said.length === 0) {
    return null;
  }
  const { url } = parts(thread.comments[0]);
  const path = threadPath(thread.place);
  const lines = aboutLines(about, "GitHub pull request review thread");
  lines.push(`URL: ${url}`);
  if (path !== null) {
    lines.push(`Location: ${path}`);
  }
  lines.push(
    `Thread state: ${thread.place.resolved ? "resolved" : "unresolved"}`,
  );
  if (thread.place.outdated) {
    lines.push(
      "Note: this thread is outdated (the code it refers to has changed)",
    );
  }
  lines.push("", said.join("\n\n---\n\n"));
  return {
    kind: "comment",
    id: `${about.number}:${threadKey(thread)}`,
    title: path ?? "Discussion thread",
    text: lines.join("\n"),
    url: url === "" ? null : url,
  };
}
