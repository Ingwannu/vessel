// What a call to a node or an op of the bridge gives a screen: its value,
// or a typed reason it has none (the plan's "a typed result or a typed
// failure"). A component never catches an error itself.
//
// [Decision Log]
// - Purpose: Settings' sections call nodes (devices, sync, an agent's
//   options) and the bridge (the book's edits, pairing). Before, each
//   screen wrote its own `try`, turned the error into text with
//   `instanceof Error ? … : String(…)`, and lost why it failed: a busy
//   bridge and a refused link read the same.
// - Choice: one `attempt` here, at the edge between the bridge's promises
//   and the page, that keeps the bridge's reason (`OpFailed.reason`, a
//   call's `Failure`) as data, so what a screen says next is a switch on
//   it, tested, not a guess from the message.
// - Alternative: results instead of rejections in `bridge/client.ts`
//   itself; every other caller (the stores, the session screens) would
//   change with it, for no gain to them.

import { CallFailed } from "../bridge/client";
import { OpFailed, type OpReason } from "../bridge/frames";
import type { Failure } from "../wire/bridge";
import type { ErrorCode } from "../wire/proto";

/** Why there is no value. */
export type Problem =
  /** The bridge refused or could not do an op (`unreadable`, `busy`,
   * `failed`), or the page lost its socket before the answer. */
  | { from: "op"; reason: OpReason; message: string }
  /** A node did not answer a call (its error, an older protocol, gone). */
  | { from: "call"; failure: Failure; message: string };

/** A value, or why there is none. */
export type Outcome<T> =
  { ok: true; value: T } | { ok: false; problem: Problem };

/** `error`, thrown by the bridge, as a problem. Anything else thrown is a
 * fault of the page, reported as the op that failed without a reason. */
export function problemOf(error: unknown): Problem {
  if (error instanceof OpFailed) {
    return { from: "op", reason: error.reason, message: error.message };
  }
  if (error instanceof CallFailed) {
    return { from: "call", failure: error.failure, message: error.message };
  }
  return {
    from: "op",
    reason: "failed",
    message: error instanceof Error ? error.message : String(error),
  };
}

/** The node's own error code for `problem`, when the node answered with
 * one (`null`: the link, the bridge or an older protocol failed). */
export function errorCode(problem: Problem): ErrorCode | null {
  return problem.from === "call" && problem.failure.kind === "remote"
    ? problem.failure.error.code
    : null;
}

/** Run `work`, catching what the bridge throws into an outcome. */
export async function attempt<T>(work: () => Promise<T>): Promise<Outcome<T>> {
  try {
    return { ok: true, value: await work() };
  } catch (error) {
    return { ok: false, problem: problemOf(error) };
  }
}

/** What to do next, in the page's words, for a problem: the message says
 * what happened; this says what helps. */
export function nextStep(problem: Problem): string {
  if (problem.from === "call") {
    switch (problem.failure.kind) {
      case "remote":
        return "The machine refused it.";
      case "not_in_protocol":
        return "That machine runs an older Vessel; update it to use this.";
      case "disconnected":
        return "The connection dropped before the answer; it may or may not have happened.";
      case "unreachable":
      case "refused":
      case "failed":
        return "Check that the machine is on and Vessel is running there.";
    }
  }
  switch (problem.reason) {
    case "busy":
      return "This page has too many requests waiting; try again in a moment.";
    case "unreadable":
      return "This page sent something vessel ui could not read; reload the page and try again.";
    case "failed":
      return "Nothing was changed.";
    case "disconnected":
      return "This page lost its connection to vessel ui; it reconnects by itself.";
  }
}

/** The problem's message, then its next step, as two sentences.
 *
 * A node's message often ends without a stop ("… (it needs the `admin`
 * scope)"), so a bare join read as one run-on sentence (TRACKER 820).
 * One shared join, so every site that says both reads the same:
 * - the message is kept as the node wrote it, with a period added only
 *   when it does not already end in `.`, `!` or `?`;
 * - an empty message leaves only the next step. */
export function withNextStep(problem: Problem): string {
  const said = problem.message.trimEnd();
  const next = nextStep(problem);
  if (said === "") {
    return next;
  }
  return /[.!?]$/.test(said) ? `${said} ${next}` : `${said}. ${next}`;
}
