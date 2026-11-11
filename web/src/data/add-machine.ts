// Whether the add-machine dialog is open (track W2). It is opened from
// Settings › Machines, the welcome screen and the command palette, and
// drawn once by the app's root, so its state lives here rather than in
// any one of them (as the new-session dialog's lives in `context.ts`).

import { Store } from "./store";

/** Open, or not. A store of its own so opening it redraws nothing else. */
export const addMachine = new Store<boolean>(false);

export function openAddMachine(): void {
  addMachine.set(true);
}

export function closeAddMachine(): void {
  addMachine.set(false);
}
