// One toast at a time, as Paseo's ToastHost (toast-host.tsx 81-130 and its
// viewport's timer, 146-195): a new toast replaces the one up and starts
// its own life; dismissing acts only on the toast it was asked for; a
// pointer resting on the card holds the life it has left.
//
// [Decision Log]
// - Purpose: TRACKER 802. Paseo shows one toast; the page kept a stack (a
//   quarter of the window, scrolling) in which each toast ran its own 4 s.
// - Existing: `ui/toasts.tsx` held a list and one timer per toast; the
//   host's placement and its dock-anchored bottom edge (TRACKER 796,
//   styles/feedback.css) stay as they are.
// - Alternatives: (1) keep the list and cap it at one, leaving the old
//   timers to tell a gone toast from the shown one; (2) Paseo's shape: one
//   slot, numbered toasts, one timer restarted for each new toast.
// - Choice: 2, as plain module state outside React so that it is tested
//   under node (the page's vitest runs without a DOM).
// - Costs: a toast that arrives while another is being read replaces it,
//   an error that waits for its dismissal too (Paseo's rule). The lives
//   stay the page's (DESIGN.md §6.10), not Paseo's 2.2 s and 3.2 s.
import { Store } from "../data/store";

export type ToastTone = "ok" | "error";

export interface Toast {
  readonly id: number;
  readonly tone: ToastTone;
  readonly text: string;
}

/** How long each tone stays up by itself; `null` waits for a dismissal. */
const LIFE_MS: Readonly<Record<ToastTone, number | null>> = {
  ok: 4000,
  error: null,
};

/** The toast up, if any. */
export const shownToast = new Store<Toast | null>(null);

let next = 1;
let timer: ReturnType<typeof setTimeout> | null = null;
let deadline: number | null = null;
let remaining = 0;

function stop(): void {
  if (timer !== null) clearTimeout(timer);
  timer = null;
  deadline = null;
}

/** Take toast `id` down; a toast that already replaced it stays. */
export function dismissToast(id: number): void {
  if (shownToast.get()?.id !== id) return;
  stop();
  remaining = 0;
  shownToast.set(null);
}

function run(id: number, ms: number): void {
  stop();
  remaining = ms;
  deadline = Date.now() + ms;
  timer = setTimeout(() => {
    dismissToast(id);
  }, ms);
}

/** Say `text`, in place of the toast up. */
export function toast(text: string, tone: ToastTone = "ok"): void {
  const id = next;
  next += 1;
  stop();
  remaining = 0;
  shownToast.set({ id, tone, text });
  const life = LIFE_MS[tone];
  if (life !== null) run(id, life);
}

/** A pointer came to rest on toast `id`: its life waits. */
export function holdToast(id: number): void {
  if (shownToast.get()?.id !== id || deadline === null) return;
  remaining = Math.max(0, deadline - Date.now());
  stop();
}

/** The pointer left toast `id`: the life it had left runs on. */
export function releaseToast(id: number): void {
  const shown = shownToast.get();
  if (shown?.id !== id || timer !== null) return;
  const life = LIFE_MS[shown.tone];
  if (life !== null) run(id, remaining > 0 ? remaining : life);
}
