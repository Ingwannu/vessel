// Machines as Settings shows them (track W2): what a machine's row says,
// what its "…" menu offers, and the names the book takes. Pure functions
// of the book and of what the bridge pushed; the screens only draw them.
//
// The words are the page's ("Words on the page" in the plan): a node is a
// machine, a harness an agent, and a transport is said as how the machine
// is reached. The protocol's words stay in the CLI and in detail rows that
// show a literal value.

import type { BookNode, NodeKey, NodeStatus } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import type { StateTone } from "./format";
import { isLive } from "./format";

/** The key the bridge gives this machine's own node (`NodeKey::HERE`). */
export const HERE: NodeKey = "_here";

/** Whether `key` is the machine this page was opened from. */
export function isHere(key: NodeKey): boolean {
  return key === HERE;
}

/** A machine's connection, in GOAT's words for the same states. */
export function machineState(status: NodeStatus | undefined): {
  word: string;
  tone: StateTone;
} {
  switch (status?.state) {
    case undefined:
    case "idle":
    case "connecting":
      return { word: "Connecting", tone: "warn" };
    case "connected":
      return status.node.state === "draining"
        ? { word: "Draining", tone: "warn" }
        : { word: "Connected", tone: "ok" };
    case "reconnecting":
      return { word: "Reconnecting", tone: "warn" };
    case "failed":
      return { word: "Offline", tone: "neutral" };
  }
}

/** How the page reaches `node`, as a person would say it. */
export function reachWord(node: BookNode): string {
  switch (node.transport) {
    case "ws":
      return "Direct";
    case "ssh":
      return "Over SSH";
    case "paired":
      return "Paired";
  }
}

/** An operating system as it is usually written. */
export function systemName(os: string): string {
  switch (os) {
    case "linux":
      return "Linux";
    case "macos":
      return "macOS";
    case "windows":
      return "Windows";
    case "freebsd":
      return "FreeBSD";
    default:
      return os;
  }
}

/** What a machine runs on, once it has said ("Linux x86_64"). */
export function platform(status: NodeStatus | undefined): string | null {
  return status?.state === "connected"
    ? `${systemName(status.node.os)} ${status.node.arch}`
    : null;
}

/** "1 live session", "3 live sessions". */
export function liveCount(
  sessions: readonly SessionInfo[] | undefined,
): string {
  const live = (sessions ?? []).filter(isLive).length;
  return `${String(live)} live ${live === 1 ? "session" : "sessions"}`;
}

/** A machine's row's one muted line (GOAT's machine row): what it runs
 * on, how it is reached, its state, and its live sessions while
 * connected. */
export function machineLine(
  node: BookNode,
  status: NodeStatus | undefined,
  sessions: readonly SessionInfo[] | undefined,
): string[] {
  const runsOn = platform(status);
  return [
    ...(runsOn === null ? [] : [runsOn]),
    reachWord(node),
    machineState(status).word,
    ...(status?.state === "connected" ? [liveCount(sessions)] : []),
  ];
}

/** What a machine's "…" menu offers. This machine's node is not a book
 * entry (the bridge refuses to forget or rename it); any machine can be
 * the default, this one by the book naming none. */
export function machineActions(node: BookNode): {
  rename: boolean;
  makeDefault: boolean;
  forget: boolean;
} {
  const here = isHere(node.key);
  return { rename: !here, makeDefault: !node.default, forget: !here };
}

/** The longest name the book takes (`NodeName`). */
export const MAX_NAME = 64;

/** Why `text` cannot be a machine's name in `book`, or `null` when it can.
 * The book's own rules (`vessel-client`'s `NodeName`), checked as typed so
 * a taken or malformed name never spends a pairing link. `keep` is the
 * machine being renamed: its own name is not "taken". */
export function nameProblem(
  text: string,
  book: readonly BookNode[],
  keep: NodeKey | null = null,
): string | null {
  if (text === "") {
    return "Give it a name.";
  }
  if (!/^[A-Za-z0-9]/.test(text)) {
    return "Start with a letter or a digit.";
  }
  if (!/^[A-Za-z0-9._-]*$/.test(text)) {
    return "Use letters, digits, “.”, “_” and “-” only (no spaces).";
  }
  if (text.length > MAX_NAME) {
    return `At most ${String(MAX_NAME)} characters.`;
  }
  const taken = book.some(
    (node) => node.key === text && node.key !== keep && !isHere(node.key),
  );
  return taken ? `There is already a machine called ${text}.` : null;
}

/** The machine a section is about: the one in its address, else the
 * book's default, else the first connected, else the first. */
export function machineFor(
  book: readonly BookNode[],
  statuses: Readonly<Record<NodeKey, NodeStatus>>,
  asked: string | undefined,
): BookNode | null {
  const named = book.find((node) => node.key === asked);
  if (named !== undefined) {
    return named;
  }
  return (
    book.find((node) => node.default) ??
    book.find((node) => statuses[node.key]?.state === "connected") ??
    book[0] ??
    null
  );
}
