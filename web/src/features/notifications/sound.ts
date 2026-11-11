// The notification sound: two short tones drawn with Web Audio, a rising
// pair when a session needs its owner and a falling pair when it
// finishes (GOAT plays one sound for input, another for completion).
// Drawn, not a file: nothing to fetch, and no asset outside the page's
// integrity graph (ADR 0017). Browsers allow sound only after a click or
// a key on the page, so the context is created then (`unlock`).

import type { NoticeKind } from "../../data/notices";

let context: AudioContext | null = null;

/** Make sound possible: call from a click or a key. */
export function unlockSound(): void {
  if (context === null && typeof AudioContext !== "undefined") {
    context = new AudioContext();
  }
  void context?.resume();
}

function tone(audio: AudioContext, frequency: number, at: number): void {
  const oscillator = audio.createOscillator();
  const gain = audio.createGain();
  oscillator.type = "sine";
  oscillator.frequency.value = frequency;
  gain.gain.setValueAtTime(0.0001, at);
  gain.gain.exponentialRampToValueAtTime(0.18, at + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.22);
  oscillator.connect(gain).connect(audio.destination);
  oscillator.start(at);
  oscillator.stop(at + 0.25);
}

/** Play `kind`'s sound, if the page may. */
export function playNotice(kind: NoticeKind): void {
  if (context?.state !== "running") {
    return;
  }
  const now = context.currentTime;
  const [first, second] = kind === "needs-you" ? [660, 880] : [880, 660];
  tone(context, first, now);
  tone(context, second, now + 0.16);
}
