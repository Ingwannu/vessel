// Every call the session screens make to a node, as typed functions that
// answer a typed result or a typed failure (milestone 10, "A component
// does not call a node").
//
// [Decision Log]
// - Purpose: the new-session page and the session view start sessions,
//   send prompts, answer permissions, read and set an agent's options and
//   rename a session. Milestone 9 wrote each call inside the component
//   that drew its button, with its own `instanceof Error` dance.
// - Constraint: a failure has to say why in the operator's words (the
//   node's message, or what the link did) and stay typed, so a screen can
//   tell "this node is too old" from "the agent refused" without reading
//   text. Components may not import the bridge (`npm run lint`).
// - Alternatives: let calls throw and catch in components (every screen
//   repeats the narrowing, and a forgotten catch is an unhandled
//   rejection); a query library (ADR 0017 decided against one).
// - Choice: each function returns `Outcome<T>` (`outcome.ts`, shared with
//   the bridge's own ops): `{ ok: true, value }` or `{ ok: false, problem }`,
//   where the problem keeps the bridge's closed set. Nothing here throws.
// - Cost: one more indirection per call; the screens get shorter for it.

import type { NodeKey, StreamState } from "../wire/bridge";
import type {
  EventRecord,
  EventSeq,
  HarnessRef,
  HarnessOptions,
  OptionChoice,
  PermissionId,
  PermissionOptionId,
  SessionId,
  SessionInfo,
  SessionOption,
  SessionSpec,
  TerminalInput,
} from "../wire/proto";
import type { Fleet } from "./fleet";
import { attempt, type Outcome } from "./outcome";

/** What `harness` offers under `profile` (`null`: its default), asked of
 * the agent itself by `node` (protocol 9, ADR 0022). Takes a few seconds
 * the first time: the node starts the agent to ask it. */
export function probeOptions(
  fleet: Fleet,
  node: NodeKey,
  harness: HarnessRef,
  profile: string | null,
): Promise<Outcome<HarnessOptions>> {
  return attempt(() =>
    fleet.bridge.call(node, "harness.options", { harness, profile }),
  );
}

/** Start a session. */
export function createSession(
  fleet: Fleet,
  node: NodeKey,
  spec: SessionSpec,
): Promise<Outcome<SessionInfo>> {
  return attempt(() => fleet.bridge.call(node, "session.create", { spec }));
}

/** One session as the node reports it now. */
export function readSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<SessionInfo>> {
  return attempt(() => fleet.bridge.call(node, "session.get", { session }));
}

/** Send `text` as the next prompt of a structured session. */
export function sendPrompt(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  text: string,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.send", {
      session,
      input: { type: "prompt", content: [{ type: "text", text }] },
    }),
  );
}

/** Answer permission request `permission` with the agent's option
 * `option`. The first answer from any follower wins. */
export function answerPermission(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  permission: PermissionId,
  option: PermissionOptionId,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.send", {
      session,
      input: {
        type: "answer_permission",
        id: permission,
        outcome: { outcome: "selected", option },
      },
    }),
  );
}

/** Stop the turn the agent is running. `turn` (protocol 10) names the
 * turn meant, by its `turn_started` record: a stop that lands after that
 * turn ended (the queue started the next) holds the queue and stops
 * nothing. `null` stops whatever runs, as before protocol 10. */
export function cancelTurn(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  turn: EventSeq | null = null,
): Promise<Outcome<null>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.send", {
      session,
      input:
        turn === null ? { type: "cancel_turn" } : { type: "cancel_turn", turn },
    }),
  );
}

/** Keys (or a size) for a terminal session. This one rejects rather than
 * answering an outcome: its one caller, `TerminalSender`, already keeps
 * order and failures for a stream of sends (data/terminal-sender.ts). */
export function sendTerminal(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  input: TerminalInput,
): Promise<null> {
  return fleet.bridge.call(node, "session.send", {
    session,
    input: { type: "terminal", input },
  });
}

/** Stop a session and everything its agent started. */
export function killSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<null>> {
  return attempt(() => fleet.bridge.call(node, "session.kill", { session }));
}

/** Start a session's agent again on its conversation (TRACKER 803,
 * `session.reload`): answered once it is ready; an ended one is started
 * again, an archived one unarchived. */
export function reloadSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
): Promise<Outcome<SessionInfo>> {
  return attempt(() => fleet.bridge.call(node, "session.reload", { session }));
}

/** Continue a structured session under another profile of its agent
 * (`null`: the default one), between turns. */
export function moveProfile(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  profile: string | null,
): Promise<Outcome<SessionInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.move_profile", { session, profile }),
  );
}

/** Change one option of a running session. The answer is every option as
 * the agent reports them after the change; the agent's refusal is the
 * failure. The session's log reports the change too, which is what the
 * page shows (ADR 0022). */
export function setOption(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  choice: OptionChoice,
): Promise<Outcome<SessionOption[]>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.set_option", { session, choice }),
  );
}

/** Give a session another title (already checked: `data/titles.ts`). */
export function renameSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  title: string,
): Promise<Outcome<SessionInfo>> {
  return attempt(() =>
    fleet.bridge.call(node, "session.rename", { session, title }),
  );
}

/** What a followed session's stream reports. */
export interface Follower {
  record: (record: EventRecord) => void;
  state: (state: StreamState) => void;
}

/** Follow `session`'s log from record `from`; the returned function
 * stops. Records arrive in order, each once, across reconnects. */
export function followSession(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  from: number,
  follower: Follower,
): () => void {
  return fleet.bridge.follow(node, session, from, follower);
}
