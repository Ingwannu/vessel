// This window's presence on each machine (`push.presence`, protocol 22;
// ADR 0041, ADR 0089), so a machine knows whether a person is here before
// it sends a Web Push, and which window is the one to tell.
//
// [Decision Log]
// - Purpose: a machine sent every Web Push even while the person had the
//   page open in front of them (ADR 0114's stated difference). The machine
//   already decides from presence: push only when no window reported real
//   input in the last 180 s, and nothing when a visible window shows that
//   very session. The page never reported presence.
// - Reference: Paseo 0.15.4 (~/refs/paseo-latest/packages/app/src/hooks):
//   - `client-activity-tracker.ts:1-2`: heartbeat 15 s, immediate
//     heartbeats at most every 5 s;
//   - `:63-73`: input records activity, then tries an immediate heartbeat;
//   - `:78-89`: a change of focused session records activity and sends at
//     once;
//   - `:90-105`: becoming visible records activity, and becoming hidden
//     sends nothing until the next heartbeat;
//   - `use-client-activity.ts:69-82`: visibility is
//     `document.visibilityState`; window focus, pointer, key, wheel and
//     touch are input;
//   - `use-client-activity.ts:125-148`: a heartbeat at once on connecting,
//     then every 15 s while connected, none while not.
//   The machine's rule is Paseo's `agent-attention-policy.ts:3,42-76`
//   (vessel-node `push/policy.rs`): present means input within 180 s, and
//   only a visible window focused on the target suppresses. An unfocused
//   but visible window still counts as visible, as in Paseo; a hidden one
//   reports `background`.
// - Difference, by the protocol: Paseo sends the input's own time. Vessel
//   sends `active` and the machine stamps its own time (ADR 0089: no client
//   wall clocks), so presence ends 180 s after the report that carried the
//   input. Input waits at most one heartbeat for that report, so presence
//   can last up to 15 s longer than Paseo's. A machine that connects gets
//   `active` only for input in the last 15 s, the same bound.
// - Choice: this module is the tracker, with no DOM and an injected clock;
//   `features/notifications/use-presence.ts` wires it to the page.
// - Effect: from now, a present window keeps a machine's Web Push from
//   being sent (Paseo's behaviour), and a visible window showing a session
//   keeps that session's push from being sent anywhere.

import { sessionIdOf } from "../bridge/ids";
import type { NodeKey } from "../wire/bridge";
import type {
  PushActivity,
  PushFocus,
  ReportPushPresence,
  SessionId,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome, type Problem } from "./outcome";

/** Paseo's `HEARTBEAT_INTERVAL_MS` (`client-activity-tracker.ts:1`). */
export const HEARTBEAT_INTERVAL_MS = 15_000;

/** Paseo's `ACTIVITY_HEARTBEAT_THROTTLE_MS` (`client-activity-tracker.ts:2`). */
export const ACTIVITY_HEARTBEAT_THROTTLE_MS = 5_000;

/** `push.presence`'s protocol. */
export const PRESENCE_PROTOCOL = 22;

/** The session this window shows, on its machine. */
export interface ShownSession {
  node: NodeKey;
  id: SessionId;
}

/** The shown session as the tracker takes it (`where.ts`'s `shownSession`). */
export function shownOf(
  shown: { node: NodeKey; id: string } | null,
): ShownSession | null {
  const id = shown === null ? null : sessionIdOf(shown.id);
  return shown === null || id === null ? null : { node: shown.node, id };
}

/** Report this window's presence to `node`. */
export function reportPresence(
  fleet: Fleet,
  node: NodeKey,
  report: ReportPushPresence,
): Promise<Outcome<unknown>> {
  return attempt(() => fleet.bridge.call(node, "push.presence", report));
}

/** Whether a machine refused presence for as long as it stays connected:
 * older than protocol 22, or a device it takes no presence from. A busy
 * machine ("too many windows") is asked again at the next heartbeat. */
export function refusesPresence(problem: Problem): boolean {
  if (problem.from !== "call") {
    return false;
  }
  const { failure } = problem;
  return (
    failure.kind === "not_in_protocol" ||
    (failure.kind === "remote" &&
      (failure.error.code === "forbidden" ||
        failure.error.code === "unsupported"))
  );
}

/** Where reports go: the machines that take them now, and the call. */
export interface PresenceLink {
  /** Connected machines serving `push.presence` that have not refused it. */
  nodes: () => readonly NodeKey[];
  /** Send, without waiting; a failure is the link's to note. */
  send: (node: NodeKey, report: ReportPushPresence) => void;
}

export interface PresenceTracker {
  /** Real input: a key, a pointer, a wheel, a touch, the window focused. */
  recordUserActivity: () => void;
  /** A heartbeat now, unless one went in the last 5 s. */
  maybeSendImmediateHeartbeat: () => void;
  /** The shown session changed: activity, and a heartbeat at once. */
  setShown: (shown: ShownSession | null) => void;
  /** The page's visibility changed; says whether it did. */
  notifyAppVisibility: (visible: boolean) => { changed: boolean };
  /** A heartbeat to every machine. */
  sendHeartbeat: () => void;
  /** A heartbeat to one machine that just connected. */
  sendTo: (node: NodeKey) => void;
  /** A machine went away: what it was told is forgotten. */
  forget: (node: NodeKey) => void;
}

export interface PresenceInput {
  link: PresenceLink;
  now: () => number;
  initialVisible: boolean;
  initialShown: ShownSession | null;
}

/** Paseo's `createClientActivityTracker`, one report per machine. */
export function createPresenceTracker(input: PresenceInput): PresenceTracker {
  const { link, now } = input;
  // Paseo starts with now as the last activity (`:43`): opening the page
  // is input.
  let lastActivityAtMs = now();
  let appVisible = input.initialVisible;
  let shown = input.initialShown;
  let lastImmediateHeartbeatAtMs = Number.NEGATIVE_INFINITY;
  /** The last activity time each machine was told of. */
  const told = new Map<NodeKey, number>();

  function focusFor(node: NodeKey): PushFocus {
    if (!appVisible) {
      return { focus: "background" };
    }
    if (shown !== null && shown.node === node) {
      return { focus: "session", session: shown.id };
    }
    return { focus: "elsewhere" };
  }

  function activityFor(node: NodeKey): PushActivity {
    const before = told.get(node);
    told.set(node, lastActivityAtMs);
    if (before === undefined) {
      return now() - lastActivityAtMs <= HEARTBEAT_INTERVAL_MS
        ? "active"
        : "unchanged";
    }
    return lastActivityAtMs > before ? "active" : "unchanged";
  }

  function sendTo(node: NodeKey): void {
    link.send(node, { focus: focusFor(node), activity: activityFor(node) });
  }

  function sendHeartbeat(): void {
    for (const node of link.nodes()) {
      sendTo(node);
    }
  }

  function recordUserActivity(): void {
    lastActivityAtMs = now();
  }

  return {
    recordUserActivity,
    maybeSendImmediateHeartbeat() {
      const at = now();
      if (at - lastImmediateHeartbeatAtMs < ACTIVITY_HEARTBEAT_THROTTLE_MS) {
        return;
      }
      lastImmediateHeartbeatAtMs = at;
      sendHeartbeat();
    },
    setShown(next) {
      if (next?.node === shown?.node && next?.id === shown?.id) {
        return;
      }
      shown = next;
      recordUserActivity();
      sendHeartbeat();
    },
    notifyAppVisibility(visible) {
      if (appVisible === visible) {
        return { changed: false };
      }
      appVisible = visible;
      if (visible) {
        recordUserActivity();
      }
      return { changed: true };
    },
    sendHeartbeat,
    sendTo(node) {
      if (link.nodes().includes(node)) {
        sendTo(node);
      }
    },
    forget(node) {
      told.delete(node);
    },
  };
}
