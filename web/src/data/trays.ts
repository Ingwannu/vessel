// The trays of this tab, one per composer (a session, or the new-session
// page), and the hand-over of the new-session page's files to the session
// it created.

import { Tray } from "./tray";

/** The trays of this tab, by composer. */
const trays = new Map<string, Tray>();

/** The most trays kept: a tray for each of the last sessions written in
 * is enough; the oldest is dropped beyond it. */
const TRAYS = 16;

/** The tray of composer `key` (a session id, or the new-session page's
 * `new`), made when first asked. */
export function trayOf(key: string): Tray {
  const found = trays.get(key);
  if (found !== undefined) {
    // Most recent last, so the first is the one to drop.
    trays.delete(key);
    trays.set(key, found);
    return found;
  }
  const made = new Tray();
  trays.set(key, made);
  if (trays.size > TRAYS) {
    const [oldest] = trays.keys();
    if (oldest !== undefined) {
      trays.get(oldest)?.dispose();
      trays.delete(oldest);
    }
  }
  return made;
}

/** Give the new-session page's files to the session it created: the
 * session's composer uploads them once its agent is ready. */
export function handOver(from: string, to: string): void {
  const tray = trays.get(from);
  if (tray === undefined) {
    return;
  }
  trays.delete(from);
  trays.get(to)?.dispose();
  trays.set(to, tray);
}
