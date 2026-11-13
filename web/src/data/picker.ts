// The agent/model/effort control's words (agent-orchestrator's
// `AgentModelPicker`): what its closed trigger reads, and its accessible
// name, which says what its visible text alone does not. Divided from
// `options.ts`, which reads the agent's options; this says them.

import { type Controls, type ControlsState, currentName } from "./options";

/** What the agent/model/effort control reads when closed: the model's
 * name (or the agent's, when it offers no model), then the effort's. */
export function pickerLabel(
  controls: Controls,
  agent: string,
): { main: string; effort: string | null } {
  return {
    main: controls.model === null ? agent : currentName(controls.model),
    effort: controls.effort === null ? null : currentName(controls.effort),
  };
}

/** The agent/model/effort control's accessible name: what it is for,
 * then what it shows, which its visible text alone does not say. */
export function pickerName(state: ControlsState, agent: string): string {
  switch (state.kind) {
    case "loading":
      return `Agent, model and effort: ${agent}, loading models`;
    case "failed":
    case "refused":
    case "terminal":
      return `Agent, model and effort: ${agent}`;
    case "ready": {
      const label = pickerLabel(state.controls, agent);
      const parts = [
        agent,
        ...(label.main === agent ? [] : [label.main]),
        ...(label.effort === null ? [] : [label.effort]),
      ];
      return `Agent, model and effort: ${parts.join(", ")}`;
    }
  }
}
