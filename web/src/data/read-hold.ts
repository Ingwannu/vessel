// "Mark as unread" against the page's own read marks (milestone 11,
// Track A-page): a session open in front would mark itself read again at
// once. A mark made anywhere on the page (the title bar's menu, the
// archive) asks the session's open page, if any, to send no `read` until
// a record arrives after it (`data/marks.ts`'s `heldAt`).

import { Store } from "./store";

/** Sessions marked unread here whose open page has not taken the hold. */
export const readHolds = new Store<ReadonlySet<string>>(new Set());

/** Ask `session`'s open page not to mark it read for now. */
export function holdRead(session: string): void {
  readHolds.update((held) => new Set([...held, session]));
}

/** Take the hold on `session`, if one was asked; true when there was. */
export function takeReadHold(session: string): boolean {
  if (!readHolds.get().has(session)) {
    return false;
  }
  readHolds.update((held) => {
    const next = new Set(held);
    next.delete(session);
    return next;
  });
  return true;
}
