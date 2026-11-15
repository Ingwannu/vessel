// A permission request as the page answers it: GOAT's approval row (what
// is asked, then Decline and Approve, the rest under "More"), and the one
// quiet line the timeline keeps for it.
//
// [Decision Log]
// - Purpose: milestone 9 drew every request twice (a card in the
//   timeline and a bar above the composer), each with every option as a
//   button. The owner asked for one prompt.
// - Constraint: the options are the agent's (ACP's `allow_once`,
//   `allow_always`, `reject_once`, `reject_always`, with the agent's own
//   names), and an agent may offer any of them, in any order.
// - Choice: Approve is the first "once" allow, else the first allow;
//   Decline the first "once" reject, else the first reject. Every other
//   option, in the agent's order, is under "More". The buttons keep the
//   agent's names. The timeline's line says the outcome in a word and the
//   option's name.

import type {
  PermissionOption,
  PermissionOptionKind,
  PermissionOutcome,
} from "../wire/proto";

/** The answers of one request, as the approval row shows them. */
export interface Answers {
  approve: PermissionOption | null;
  decline: PermissionOption | null;
  more: readonly PermissionOption[];
}

function allows(kind: PermissionOptionKind): boolean {
  switch (kind) {
    case "allow_once":
    case "allow_always":
      return true;
    case "reject_once":
    case "reject_always":
      return false;
  }
}

function permits(option: PermissionOption): boolean {
  return option.action === undefined || option.action === null
    ? allows(option.kind)
    : option.action.behavior === "allow";
}

/** `options` split into Approve, Decline and the rest. */
export function answersOf(options: readonly PermissionOption[]): Answers {
  const approve =
    options.find((option) => option.kind === "allow_once" && permits(option)) ??
    options.find(permits) ??
    null;
  const decline =
    options.find(
      (option) => option.kind === "reject_once" && !permits(option),
    ) ??
    options.find((option) => !permits(option)) ??
    null;
  return {
    approve,
    decline,
    more: options.filter((option) => option !== approve && option !== decline),
  };
}

/** Where a request stands. */
export type Settled =
  | { state: "pending" }
  | { state: "allowed"; option: string }
  | { state: "declined"; option: string }
  | { state: "cancelled" }
  | { state: "unknown"; option: string };

/** `outcome` of a request offering `options`. */
export function settled(
  options: readonly PermissionOption[],
  outcome: PermissionOutcome | null,
): Settled {
  if (outcome === null) {
    return { state: "pending" };
  }
  switch (outcome.outcome) {
    case "cancelled":
      return { state: "cancelled" };
    case "selected": {
      const chosen = options.find((option) => option.id === outcome.option);
      if (chosen === undefined) {
        return { state: "unknown", option: outcome.option };
      }
      return permits(chosen)
        ? { state: "allowed", option: chosen.name }
        : { state: "declined", option: chosen.name };
    }
  }
}

/** The timeline's line for a request: what was asked about, then how it
 * ended. */
export function settledLine(what: string, state: Settled): string {
  switch (state.state) {
    case "pending":
      return `Waiting on permission · ${what}`;
    case "allowed":
      return `Allowed · ${what} · ${state.option}`;
    case "declined":
      return `Declined · ${what} · ${state.option}`;
    case "unknown":
      return `Permission outcome unavailable · ${what}`;
    case "cancelled":
      return `Permission cancelled · ${what}`;
  }
}
