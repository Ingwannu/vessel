// The Add project sheet's calls (TRACKER 234): one at a time, the page's
// progress while it runs, its words when it fails; on success the sheet
// closes on the project (plan differences 6 and 7).

import { useNavigate } from "@tanstack/react-router";
import { useRef } from "react";

import type { Fleet } from "../../data/fleet";
import type { Submitted } from "../../data/workspace/add-project/calls";
import { setAddProjectPageStatus } from "../../data/workspace/add-project/editing";
import type {
  AddProjectFlowState,
  AddProjectPage,
} from "../../data/workspace/add-project/model";
import { tookProject } from "../../data/workspace/project-store";
import { wantedPlace } from "../../data/workspace/wanted-place";
import type { NodeKey } from "../../wire/bridge";
import { useNewSession } from "../shell/use-new-session";

export type Submit = (
  kind: AddProjectPage["kind"],
  call: () => Promise<Submitted>,
) => void;

export function useAddProjectSubmit(
  fleet: Fleet,
  node: NodeKey | null,
  setState: (
    update: (current: AddProjectFlowState) => AddProjectFlowState,
  ) => void,
  onClose: () => void,
): Submit {
  const navigate = useNavigate();
  const newSession = useNewSession();
  const busy = useRef(false);
  return (kind, call) => {
    if (node === null || busy.current) return;
    busy.current = true;
    setState((current) =>
      setAddProjectPageStatus(current, kind, {
        isSubmitting: true,
        error: null,
      }),
    );
    void call().then((result) => {
      busy.current = false;
      if (!result.ok) {
        setState((current) =>
          setAddProjectPageStatus(current, kind, {
            isSubmitting: false,
            error: result.message,
          }),
        );
        return;
      }
      const project = result.value;
      tookProject(fleet, node, project);
      onClose();
      if (project.state.state === "ready") {
        // As the sidebar's "New session here".
        wantedPlace.set({ node, project: project.id });
        newSession(node);
      } else {
        // A clone: its progress is its row in Settings › Projects.
        void navigate({
          to: "/settings/$section",
          params: { section: "projects" },
          search: { machine: node },
        });
      }
    });
  };
}
