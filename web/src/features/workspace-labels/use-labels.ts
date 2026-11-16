// The labels hub for a component that shows labels: held while it is
// mounted, so the long polls run only while something draws them.

import { useEffect } from "react";

import { useFleet } from "../../context";
import { useStore } from "../../data/store";
import {
  labelsOf,
  type LabelsState,
  type WorkspaceLabels,
} from "../../data/workspace-labels/hub";

export function useLabels(): { hub: WorkspaceLabels; state: LabelsState } {
  const hub = labelsOf(useFleet());
  useEffect(() => hub.hold(), [hub]);
  const state = useStore(hub.store, (value) => value);
  return { hub, state };
}
