// A machine's sessions as the fleet keeps them (fleet.ts): a list read
// again with the objects of the one before kept where nothing changed,
// and a session the node just answered with put in, or taken out, at
// once. Pure: the fleet's store applies them. Divided from fleet.ts when
// the page's join made it pass its budget (milestone 11).

import type { NodeKey } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";

/** `next` with the objects of `previous` kept where nothing changed, so
 * React re-renders only the rows that did. */
export function keep(
  previous: readonly SessionInfo[] | undefined,
  next: SessionInfo[],
): readonly SessionInfo[] {
  if (previous === undefined) {
    return next;
  }
  const byId = new Map(previous.map((session) => [session.id, session]));
  const kept = next.map((session) => {
    const old = byId.get(session.id);
    return old !== undefined && JSON.stringify(old) === JSON.stringify(session)
      ? old
      : session;
  });
  const same =
    kept.length === previous.length &&
    kept.every((session, index) => session === previous[index]);
  return same ? previous : kept;
}

/** `state` with `session` in `node`'s list: where it was, or first. A
 * machine whose list was never read is left as it is. */
export function withSession(
  state: FleetState,
  node: NodeKey,
  session: SessionInfo,
): FleetState {
  const before = state.sessions[node];
  if (before === undefined) {
    return state;
  }
  const found = before.some((listed) => listed.id === session.id);
  const after = found
    ? before.map((listed) => (listed.id === session.id ? session : listed))
    : [session, ...before];
  return { ...state, sessions: { ...state.sessions, [node]: after } };
}

/** `state` without the session `id` in `node`'s list. */
export function withoutSession(
  state: FleetState,
  node: NodeKey,
  id: SessionInfo["id"],
): FleetState {
  const before = state.sessions[node];
  if (before === undefined) {
    return state;
  }
  const after = before.filter((listed) => listed.id !== id);
  return { ...state, sessions: { ...state.sessions, [node]: after } };
}
