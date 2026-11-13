// A project to start the next session in, asked from outside the
// new-session page (the sidebar's "+" on a project's row): the page's
// tray starts there instead of at no project. Kept in memory only; a
// reload starts from no project, as the tray does.

import type { NodeKey } from "../../wire/bridge";
import type { ProjectId } from "../../wire/proto";
import { Store } from "../store";

/** The machine and project the next new session should start in. */
export interface WantedPlace {
  node: NodeKey;
  project: ProjectId;
}

export const wantedPlace = new Store<WantedPlace | null>(null);
