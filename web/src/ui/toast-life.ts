// One toast at a time, as Paseo's ToastHost (toast-host.tsx 81-130 and its
// viewport's timer, 146-195): a new toast replaces the one up and starts
// its own life; dismissing acts only on the toast it was asked for; a
// pointer resting on the card holds the life it has left.
//
// [Decision Log]
// - Purpose: TRACKER 802. Paseo shows one toast with its own duration
//   (2.2 s, an error 3.2 s, `null` until dismissed); the page kept a
//   scrolling stack in which each success ran 4 s and each error stayed
//   until its close button.
// - Existing: `ui/toasts.tsx` held a list and one timer per toast; the
//   host's placement and its dock-anchored bottom edge (TRACKER 796,
//   styles/feedback.css) stay as they are.
// - Alternatives: (1) keep the list and cap it at one, leaving the old
//   timers to tell a gone toast from the shown one; (2) Paseo's shape: one
//   slot, numbered toasts, one timer restarted for each new toast.
// - Choice: 2, as plain module state outside React so that it is tested
//   under node (the page's vitest runs without a DOM).
// - The default variant is `success`, not Paseo's `default`: nearly every
//   page toast reports a finished action, which Paseo shows as success; a
//   notice that is neither passes `default`.
// - Costs: a toast that arrives while another is read replaces it, an
//   error too, and an error leaves by itself after 3.2 s (Paseo's rule).
import { Store } from "../data/store";

/** Paseo's toast variants (`ToastVariant`). */
export type ToastVariant = "default" | "info" | "success" | "warning" | "error";

export interface Toast {
  readonly id: number;
  readonly variant: ToastVariant;
  readonly text: string;
  /** Paseo's `copied` toast: success, its mark in the text colour. */
  readonly copied: boolean;
}

export interface ToastOptions {
  /** How long it stays by itself; `null` until dismissed. */
  readonly durationMs?: number | null;
}

/** Paseo's `DEFAULT_DURATION_MS`, and its `toast.error` duration. */
export const TOAST_LIFE_MS = 2200;
export const ERROR_TOAST_LIFE_MS = 3200;

/** The toast up, if any. */
export const shownToast = new Store<Toast | null>(null);

let next = 1;
let life: number | null = null;
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
  life = null;
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

function show(
  text: string,
  variant: ToastVariant,
  copied: boolean,
  options: ToastOptions,
): () => void {
  const id = next;
  next += 1;
  stop();
  remaining = 0;
  const fallback = variant === "error" ? ERROR_TOAST_LIFE_MS : TOAST_LIFE_MS;
  life = options.durationMs === undefined ? fallback : options.durationMs;
  shownToast.set({ id, variant, text, copied });
  if (life !== null) run(id, Math.max(0, life));
  return () => {
    dismissToast(id);
  };
}

/** Say `text`, in place of the toast up; the returned call takes it down. */
export function toast(
  text: string,
  variant: ToastVariant = "success",
  options: ToastOptions = {},
): () => void {
  return show(text, variant, false, options);
}

/** Paseo's `toast.copied`: a success whose mark is in the text colour. */
export function toastCopied(text: string): () => void {
  return show(text, "success", true, {});
}

/** A pointer came to rest on toast `id`: its life waits. */
export function holdToast(id: number): void {
  if (shownToast.get()?.id !== id || deadline === null) return;
  remaining = Math.max(0, deadline - Date.now());
  stop();
}

/** The pointer left toast `id`: the life it had left runs on. */
export function releaseToast(id: number): void {
  if (shownToast.get()?.id !== id || timer !== null || life === null) return;
  run(id, remaining > 0 ? remaining : life);
}
