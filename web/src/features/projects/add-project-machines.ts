// The machines the Add project sheet can add to (TRACKER 234): the
// connected ones, with what each offers. Paseo's `AddProjectFlow`
// `availableHosts`, host as machine (docs/plans/wp-add-project-flow.md,
// differences 2, 8 and 11).

import { useMemo } from "react";

import { useFleetState } from "../../context";
import { nodeLabel } from "../../data/nodes";
import { useStore } from "../../data/store";
import type { AddProjectMachine } from "../../data/workspace/add-project/model";
import {
  projectStore,
  type ProjectsView,
} from "../../data/workspace/project-store";
import {
  PROJECT_DIRECTORY_PROTOCOL,
  REPOSITORY_SEARCH_PROTOCOL,
} from "../../wire/proto";

/** Whether a machine may clone: its list says its Git is not missing or
 * too old (Paseo's "Update this host to clone GitHub repositories"); a
 * machine not read yet may. */
function canClone(view: ProjectsView | undefined): boolean {
  if (view?.kind !== "ready") return true;
  const git = view.list.git.state;
  return git !== "missing" && git !== "too_old";
}

export function useAddProjectMachines(): AddProjectMachine[] {
  const book = useFleetState((state) => state.book);
  const statuses = useFleetState((state) => state.statuses);
  const views = useStore(projectStore, (all) => all);
  return useMemo(
    () =>
      (book ?? []).flatMap((node) => {
        const status = statuses[node.key];
        if (status?.state !== "connected") return [];
        return [
          {
            node: node.key,
            label: nodeLabel(node, status),
            // Every machine adds projects with stable ids (difference 2);
            // Browse is the native app's only (difference 5); GitHub
            // search is `forge.repos` (TRACKER 440, ADR 0088).
            canAddProject: true,
            canBrowse: false,
            canCloneGithubRepositories: canClone(views[node.key]),
            canSearchGithubRepositories:
              status.protocol >= REPOSITORY_SEARCH_PROTOCOL,
            canCreateDirectory: status.protocol >= PROJECT_DIRECTORY_PROTOCOL,
          },
        ];
      }),
    [book, statuses, views],
  );
}
