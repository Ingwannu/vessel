// Whether the command palette is open. It is opened from the sidebar's
// search row and by its shortcut, which works with the sidebar hidden
// (milestone 11), so its state is the shell's, not the sidebar's: it
// lives here, as the add-machine dialog's does (add-machine.ts).

import { Store } from "./store";

/** Open, or not. A store of its own so opening it redraws nothing else. */
export const palette = new Store<boolean>(false);

export function openPalette(): void {
  palette.set(true);
}

export function closePalette(): void {
  palette.set(false);
}
