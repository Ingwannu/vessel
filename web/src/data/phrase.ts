// What a card's phrase says and means (DESIGN.md §6.12): its words, as
// agent-orchestrator's daemon writes them; whether it waits on its owner;
// whether work is turning; its colour. Divided from data/board.ts, which
// keeps where a session is (its lane) and how the board deals the cards;
// the same rule holds here: every `DisplayStatus` is matched
// exhaustively, so a phrase the node learns to send does not compile
// until it has words, a colour and an answer to each question.

import type { DisplayStatus } from "../wire/proto";
import type { Phrase, Placement } from "./board";
import { statusWord } from "./format";

/** The words of a node's phrase: agent-orchestrator's, as its daemon
 * writes them. */
export function displayStatusLabel(status: DisplayStatus): string {
  switch (status) {
    case "working":
      return "Working";
    case "blocked":
      return "Blocked";
    case "exited":
      return "Exited";
    case "no_signal":
      return "No signal";
    case "interrupted":
      return "Interrupted";
    case "awaiting_pr":
      return "Awaiting PR";
    case "idle":
      return "Idle";
    case "fixing_ci":
      return "Fixing CI failures";
    case "addressing_comments":
      return "Addressing comments";
    case "needs_review":
      return "Needs review";
    case "review_scheduled":
      return "Review scheduled";
    case "reviewing":
      return "Reviewing";
    case "review_failed":
      return "Review failed";
    case "review_pending":
      return "Review pending";
    case "draft":
      return "Draft";
    case "ci_failing":
      return "CI failing";
    case "commented":
      return "Commented";
    case "changes_requested":
      return "Changes requested";
    case "needs_human_review":
      return "Needs human review";
    case "mergeable":
      return "Mergeable";
    case "approved":
      return "Approved";
    case "merged":
      return "Merged";
    case "closed":
      return "Closed without merge";
    case "terminated":
      return "Terminated";
  }
}

/** What a card's status line reads. */
export function phraseLabel(phrase: Phrase): string {
  switch (phrase.from) {
    case "node":
      return displayStatusLabel(phrase.status);
    case "status":
      return statusWord(phrase.status);
  }
}

/** Whether a session waits on its owner: its card takes the needs-you
 * edge and leads its lane. The phrases are agent-orchestrator's
 * (`boardSessionNeedsAttention`): blocked, failing checks nobody fixes,
 * changes a person requested. Never true from an older node. */
export function needsYou(placed: Placement): boolean {
  if (placed.phrase.from !== "node") {
    return false;
  }
  switch (placed.phrase.status) {
    case "blocked":
    case "ci_failing":
    case "changes_requested":
      return true;
    case "working":
    case "exited":
    case "no_signal":
    case "interrupted":
    case "awaiting_pr":
    case "idle":
    case "fixing_ci":
    case "addressing_comments":
    case "needs_review":
    case "review_scheduled":
    case "reviewing":
    case "review_failed":
    case "review_pending":
    case "draft":
    case "commented":
    case "needs_human_review":
    case "mergeable":
    case "approved":
    case "merged":
    case "closed":
    case "terminated":
      return false;
  }
}

/** Whether work is turning right now, which earns the status line its
 * spinner: the agent in a turn, or a loop the fleet drives. A settled
 * phrase ("Mergeable", "Approved") gets none. From an older node, only a
 * session still starting. */
export function isWorking(placed: Placement): boolean {
  if (placed.phrase.from === "status") {
    return placed.phrase.status.state === "starting";
  }
  switch (placed.phrase.status) {
    case "working":
    case "fixing_ci":
    case "addressing_comments":
    case "reviewing":
    case "review_pending":
      return true;
    case "blocked":
    case "exited":
    case "no_signal":
    case "interrupted":
    case "awaiting_pr":
    case "idle":
    case "needs_review":
    case "review_scheduled":
    case "review_failed":
    case "draft":
    case "ci_failing":
    case "commented":
    case "changes_requested":
    case "needs_human_review":
    case "mergeable":
    case "approved":
    case "merged":
    case "closed":
    case "terminated":
      return false;
  }
}

/** The colour class of a card's status line (`.tone-*` in features/board/session-card.css): the
 * lane's own, except where the phrase says more than the lane does. */
export type Tone =
  "lane" | "needs-you" | "ready" | "merged" | "danger" | "quiet";

export function tone(placed: Placement): Tone {
  if (needsYou(placed)) {
    return "needs-you";
  }
  if (placed.column === "archive") {
    // An ended session's own status word: a failure stays visible in the
    // archive, the rest is quiet.
    return placed.phrase.from === "status" &&
      placed.phrase.status.state === "failed"
      ? "danger"
      : "quiet";
  }
  if (placed.phrase.from === "status") {
    return "lane";
  }
  switch (placed.phrase.status) {
    case "mergeable":
    case "approved":
      return "ready";
    case "merged":
      return "merged";
    case "closed":
    case "exited":
      return "danger";
    case "working":
    case "blocked":
    case "no_signal":
    case "interrupted":
    case "awaiting_pr":
    case "idle":
    case "fixing_ci":
    case "addressing_comments":
    case "needs_review":
    case "review_scheduled":
    case "reviewing":
    case "review_failed":
    case "review_pending":
    case "draft":
    case "ci_failing":
    case "commented":
    case "changes_requested":
    case "needs_human_review":
    case "terminated":
      return "lane";
  }
}
