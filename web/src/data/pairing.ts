// Adding a machine (track W2): what each step of the dialog says, decided
// here so the dialog only draws it.
//
// [Decision Log]
// - Purpose: the owner asked for connecting a computer to be intuitive,
//   as Paseo and GOAT do it. Before, Settings had a field, a Check button
//   and a list of the link's raw fields, with no guidance and no way to
//   see whether the machine then worked.
// - Constraints: the link is a credential until redeemed, and `inspect`
//   reads it without sending anything, so the person can compare the
//   machine's key before anything leaves the page (ADR 0010). The ops are
//   track E's; nothing new crosses the wire.
// - Alternatives: one form (link, name, Pair) as before; a wizard with a
//   page per step. Paseo shows "Connect to this host?" with the key's
//   fingerprint between pasting and pairing; GOAT shows what follows as a
//   checklist whose rows turn from pending to passed or failed, a failed
//   one saying what to do with a command to copy.
// - Choice: Paseo's order and GOAT's checklist, in one dialog: the
//   command and the link; the machine and its key, then a name; then the
//   checklist: paired, connected, agents found. Each row is a pure
//   function of what the op answered and what the bridge pushed after.
// - Cost: the offer names the machine by its id only (the name it gives
//   itself is known once it is connected), so the confirmation shows the
//   id, the routes and the key, not a name.

import type { BookNode, NodeStatus } from "../wire/bridge";
import type { Harness, PairingOffer } from "../wire/proto";
import { agentName } from "./agents";
import { platform } from "./machines";
import { nextStep, type Problem } from "./outcome";

/** The command that makes a pairing link, run on the machine to add. */
export const PAIR_COMMAND = "vessel node pair";

/** The command that starts a machine's node and keeps it serving. */
export const START_COMMAND = "vessel node start";

/** The command that lists the agents a machine finds. */
export const AGENTS_COMMAND = "vessel harnesses";

/** Where the pairing is. */
export type Pairing =
  | { kind: "pairing" }
  | { kind: "paired"; node: BookNode }
  | { kind: "refused"; problem: Problem };

/** One row of the checklist. */
export type CheckId = "paired" | "connected" | "agents";

/** A row's state (GOAT's preflight: pending is its "Checking…"). */
export type CheckState = "pending" | "passed" | "failed" | "skipped";

/** What a failed row says to do, and a command for it if one helps. */
export interface Fix {
  text: string;
  command?: string;
}

export interface Check {
  id: CheckId;
  label: string;
  state: CheckState;
  /** One line: what happened, or what is awaited. */
  result: string;
  /** For a failed row: what to do. */
  fix?: Fix;
}

/** How the link says the machine is reached: its direct addresses, then
 * its relays, each as host and port. */
export function offerRoutes(offer: PairingOffer): string[] {
  const host = (route: string) => {
    try {
      return new URL(route).host;
    } catch {
      return route;
    }
  };
  return [
    ...offer.direct.map((route) => `Directly at ${host(route)}`),
    ...offer.relays.map((route) => `Through the relay at ${host(route)}`),
  ];
}

/** "in 9 minutes", "in 1 minute", or `null` once expired. */
export function expiresIn(
  iso: string,
  now: number = Date.now(),
): string | null {
  const left = Date.parse(iso) - now;
  if (Number.isNaN(left) || left <= 0) {
    return null;
  }
  const minutes = Math.ceil(left / 60_000);
  return minutes === 1 ? "in 1 minute" : `in ${String(minutes)} minutes`;
}

/** What a refused pairing says to do next. */
function pairingFix(problem: Problem): Fix {
  if (problem.from === "op" && problem.reason === "failed") {
    return {
      text: "A link works once and for ten minutes. Make a new one there and paste it again.",
      command: PAIR_COMMAND,
    };
  }
  return { text: nextStep(problem) };
}

function pairedCheck(pairing: Pairing): Check {
  const base = { id: "paired", label: "Paired" } as const;
  switch (pairing.kind) {
    case "pairing":
      return {
        ...base,
        state: "pending",
        result: "Asking the machine to accept this page…",
      };
    case "paired":
      return {
        ...base,
        state: "passed",
        result: `Added to your machines as ${pairing.node.label}`,
      };
    case "refused":
      return {
        ...base,
        state: "failed",
        result: pairing.problem.message,
        fix: pairingFix(pairing.problem),
      };
  }
}

function connectedCheck(
  pairing: Pairing,
  status: NodeStatus | undefined,
): Check {
  const base = { id: "connected", label: "Connected" } as const;
  if (pairing.kind !== "paired") {
    return {
      ...base,
      state: pairing.kind === "refused" ? "skipped" : "pending",
      result: pairing.kind === "refused" ? "Not tried" : "Waits for pairing",
    };
  }
  switch (status?.state) {
    case undefined:
    case "idle":
    case "connecting":
      return { ...base, state: "pending", result: "Connecting…" };
    case "reconnecting":
      return {
        ...base,
        state: "pending",
        result: `Trying again: ${status.message}`,
      };
    case "connected":
      return {
        ...base,
        state: "passed",
        result: [
          status.node.name,
          platform(status),
          `Vessel ${status.node.version}`,
        ]
          .filter((part) => part !== null)
          .join(" · "),
      };
    case "failed":
      return {
        ...base,
        state: "failed",
        result: status.message,
        fix: {
          text: "Check that Vessel is running on that machine:",
          command: START_COMMAND,
        },
      };
  }
}

function agentsCheck(
  connected: Check,
  harnesses: readonly Harness[] | undefined,
  readError: string | undefined,
): Check {
  const base = { id: "agents", label: "Agents found" } as const;
  if (connected.state !== "passed") {
    return {
      ...base,
      state: connected.state === "pending" ? "pending" : "skipped",
      result:
        connected.state === "pending"
          ? "Waits for the connection"
          : "Not tried",
    };
  }
  if (readError !== undefined) {
    return {
      ...base,
      state: "failed",
      result: readError,
      fix: { text: "Try again from Settings › Machines." },
    };
  }
  if (harnesses === undefined) {
    return { ...base, state: "pending", result: "Asking which agents it has…" };
  }
  if (harnesses.length === 0) {
    return {
      ...base,
      state: "failed",
      result: "No agent is installed there.",
      fix: {
        text: "Install Claude Code, Codex, Gemini or OpenCode on that machine; this lists what it finds:",
        command: AGENTS_COMMAND,
      },
    };
  }
  return {
    ...base,
    state: "passed",
    result: harnesses.map((harness) => agentName(harness.kind)).join(", "),
  };
}

/** The checklist after Connect, from what is known so far. */
export function pairingChecks(
  pairing: Pairing,
  status: NodeStatus | undefined,
  harnesses: readonly Harness[] | undefined,
  readError: string | undefined,
): Check[] {
  const connected = connectedCheck(pairing, status);
  return [
    pairedCheck(pairing),
    connected,
    agentsCheck(connected, harnesses, readError),
  ];
}

/** Where the checklist stands as a whole: running while a row is pending,
 * done when every row passed, else stopped at a failure. */
export function checklistState(
  checks: readonly Check[],
): "running" | "done" | "stopped" {
  if (checks.some((check) => check.state === "pending")) {
    return "running";
  }
  return checks.every((check) => check.state === "passed") ? "done" : "stopped";
}
