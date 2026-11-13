// A new session's first prompt, on its way from the new-session page to
// the session (the new chat, 2026-11-13).
//
// [Decision Log]
// - Purpose: the owner asked that the first chat work as in GOAT and
//   agent-orchestrator: type, send, and the chat is open with the prompt
//   in it. The page waited on the new-session page until the node had
//   taken the prompt, which it does only once the agent has finished
//   starting (its handshake, up to a minute).
// - Constraint: the wire has no "create with a first prompt"; the prompt
//   is a second call after `session.create`, answered once the agent is
//   ready. The session's log has the prompt only once the agent took it.
// - Alternatives: hand the prompt to the session's composer to send when
//   the agent is ready, as files already are (the text would wait in the
//   box, not in the conversation, and any other tab of the session would
//   see it as its own draft); keep waiting on the new-session page.
// - Choice: agent-orchestrator's local echo of the brief. The new-session
//   page opens the session as soon as it exists, sends the prompt without
//   waiting on it, and keeps it here by session; the session's timeline
//   shows it at its end, marked as sending, until its log has a prompt
//   or the node refused it.
// - Cost: kept in this tab only. A reload while the prompt is on its way
//   loses the echo (the call is already with the bridge and the node
//   still delivers it; the log then shows it); a refusal after that
//   reload is not seen.

import type { NodeKey } from "../wire/bridge";
import type { SessionId } from "../wire/proto";
import { sendPrompt } from "./calls";
import type { Fleet } from "./fleet";
import { Store } from "./store";
import type { Entry, Timeline } from "./timeline-model";

/** Where a first prompt is. */
export type OpeningState =
  { kind: "sending" } | { kind: "sent" } | { kind: "refused"; message: string };

export interface Opening {
  session: SessionId;
  text: string;
  /** When Send was pressed (ISO 8601). */
  at: string;
  state: OpeningState;
}

/** The most first prompts kept: one per session started from this tab
 * lately; the oldest go. */
const KEPT = 16;

/** This tab's first prompts, by session. */
export const openings = new Store<ReadonlyMap<string, Opening>>(new Map());

/** Keep `text` as `session`'s first prompt, on its way. */
export function startOpening(
  session: SessionId,
  text: string,
  at: string = new Date().toISOString(),
): void {
  openings.update((current) => {
    const next = new Map(current);
    next.delete(session);
    next.set(session, { session, text, at, state: { kind: "sending" } });
    for (const key of next.keys()) {
      if (next.size <= KEPT) {
        break;
      }
      next.delete(key);
    }
    return next;
  });
}

/** Say what became of `session`'s first prompt. */
export function settleOpening(session: SessionId, state: OpeningState): void {
  openings.update((current) => {
    const found = current.get(session);
    if (found === undefined) {
      return current;
    }
    const next = new Map(current);
    next.set(session, { ...found, state });
    return next;
  });
}

/** Forget `session`'s first prompt (its log has it, or it was dealt
 * with). */
export function forgetOpening(session: SessionId): void {
  openings.update((current) => {
    if (!current.has(session)) {
      return current;
    }
    const next = new Map(current);
    next.delete(session);
    return next;
  });
}

/** Whether `timeline`'s log has a prompt of its own: the echo has done
 * its work. */
export function promptLogged(timeline: Timeline): boolean {
  return timeline.entries.some(
    (entry) => entry.kind === "prompt" && !entry.replayed,
  );
}

/** The key of the echo's row: no record's key looks like it. */
export const OPENING_KEY = "opening";

/** `timeline` as the session's page draws it: with `opening`'s prompt at
 * its end while it is on its way (or taken, not yet in the log), and
 * nothing added once the log has a prompt or the node refused it. */
export function withOpening(
  timeline: Timeline,
  opening: Opening | undefined,
): Timeline {
  if (
    opening === undefined ||
    opening.state.kind === "refused" ||
    promptLogged(timeline)
  ) {
    return timeline;
  }
  const echo: Entry = {
    key: OPENING_KEY,
    seq: timeline.next,
    at: opening.at,
    kind: "prompt",
    text: opening.text,
    replayed: false,
    turn: timeline.turns + 1,
    attachments: [],
    sending: true,
  };
  return { ...timeline, entries: [...timeline.entries, echo] };
}

/** Send `session`'s first prompt, kept as on its way meanwhile, and say
 * what became of it. Not awaited by the new-session page: the node
 * answers once the agent has started. */
export async function deliverOpening(
  fleet: Fleet,
  node: NodeKey,
  session: SessionId,
  text: string,
): Promise<OpeningState> {
  startOpening(session, text);
  const sent = await sendPrompt(fleet, node, session, text);
  const state: OpeningState = sent.ok
    ? { kind: "sent" }
    : { kind: "refused", message: sent.problem.message };
  settleOpening(session, state);
  return state;
}
