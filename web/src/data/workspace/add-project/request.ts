// The Add project sheet's open request, Paseo's
// `stores/add-project-flow-store.ts`: a new id each time it is opened (so
// a new request starts the sheet fresh), and the machine it starts on when
// the place it was opened from names one.

import { Store } from "../../store";
import type { NodeKey } from "../../../wire/bridge";

/** One opening of the sheet. */
export interface AddProjectRequest {
  id: number;
  preferred?: NodeKey;
}

/** The open sheet's request; `null` while it is closed. */
export const addProjectRequest = new Store<AddProjectRequest | null>(null);

let next = 1;

/** Open the sheet, on `preferred` when it is one of the machines. */
export function openAddProject(preferred?: NodeKey): void {
  const id = next;
  next += 1;
  addProjectRequest.set(preferred === undefined ? { id } : { id, preferred });
}

export function closeAddProject(): void {
  addProjectRequest.set(null);
}
