// What an agent offers (protocol 9, ADR 0022), as the controls the page
// shows: the model, the effort, the permissions, on/off switches and any
// other choice, each in the agent's own words.
//
// [Decision Log]
// - Purpose: the new-session page and the session's composer both show
//   one control for agent, model and effort and one for permissions
//   (agent-orchestrator's composer), filled from what the agent reports.
// - Constraint: the options are the agent's, and change with every
//   release; Vessel writes no list of models or modes (the plan's "truth
//   over neatness"). An agent that does not offer an effort gets no
//   effort control, one that offers nothing gets only the agent's name.
// - Alternatives: per-agent tables mapping known ids to Vessel's own
//   labels (agent-orchestrator does this for the agents it knows; it goes
//   stale, and is the knowledge ADR 0022 keeps out of Vessel); showing
//   every option as one flat list (no place for the reference's two
//   controls).
// - Choice: the category decides where an option goes, and nothing else
//   is read from it. The first select of category `model` is the model,
//   of `thought_level` the effort, of `mode` the permissions. Every
//   toggle is a switch row; every other select (a second of a category,
//   `model_config`, `other`) is a submenu under them. An option of a kind
//   this page does not know (`unknown`) is skipped. Names, descriptions
//   and groups are passed through as the agent gave them.
// - Choices made before a session exists ("picks") are kept apart from
//   what the agent reported, and only those that differ from the agent's
//   own values are sent, in the agent's order (the node applies them in
//   order, and a choice the agent does not offer fails the session).

import type { OptionChoice, OptionValue, SessionOption } from "../wire/proto";

/** A select, as the page shows it. */
export interface SelectControl {
  id: string;
  name: string;
  description: string | null;
  /** The value in effect: an id among `values`, or one the agent set
   * that it does not list. */
  current: string;
  values: readonly OptionValue[];
}

/** An on/off option. */
export interface ToggleControl {
  id: string;
  name: string;
  description: string | null;
  on: boolean;
}

/** Everything an agent offers, by where the page shows it. */
export interface Controls {
  model: SelectControl | null;
  effort: SelectControl | null;
  permissions: SelectControl | null;
  /** Switch rows, under the model and effort. */
  toggles: readonly ToggleControl[];
  /** Other choices, each a submenu after the switches. */
  selects: readonly SelectControl[];
}

/** No options at all: an agent that offers none, or a terminal. */
export const NO_CONTROLS: Controls = {
  model: null,
  effort: null,
  permissions: null,
  toggles: [],
  selects: [],
};

/** Where one select goes. */
type Slot = "model" | "effort" | "permissions" | "other";

function slotOf(option: SessionOption): Slot {
  switch (option.category) {
    case "model":
      return "model";
    case "thought_level":
      return "effort";
    case "mode":
      return "permissions";
    case "model_config":
    case "other":
      return "other";
  }
}

/** `options` laid out as the page's controls. */
export function controlsOf(options: readonly SessionOption[]): Controls {
  const out: {
    model: SelectControl | null;
    effort: SelectControl | null;
    permissions: SelectControl | null;
    toggles: ToggleControl[];
    selects: SelectControl[];
  } = {
    model: null,
    effort: null,
    permissions: null,
    toggles: [],
    selects: [],
  };
  for (const option of options) {
    const description = option.description ?? null;
    switch (option.kind) {
      case "unknown":
        break;
      case "toggle":
        out.toggles.push({
          id: option.id,
          name: option.name,
          description,
          on: option.on,
        });
        break;
      case "select": {
        const control: SelectControl = {
          id: option.id,
          name: option.name,
          description,
          current: option.current,
          values: option.values,
        };
        const slot = slotOf(option);
        if (slot !== "other" && out[slot] === null) {
          out[slot] = control;
        } else {
          out.selects.push(control);
        }
        break;
      }
    }
  }
  return out;
}

/** The value of `control` in effect, or `null` when the agent set one it
 * does not list (the page then shows the id). */
export function currentValue(control: SelectControl): OptionValue | null {
  return control.values.find((value) => value.id === control.current) ?? null;
}

/** What `control`'s value in effect is called. */
export function currentName(control: SelectControl): string {
  return currentValue(control)?.name ?? control.current;
}

/** Whether `controls` has anything to choose. */
export function offersAnything(controls: Controls): boolean {
  return (
    controls.model !== null ||
    controls.effort !== null ||
    controls.permissions !== null ||
    controls.toggles.length > 0 ||
    controls.selects.length > 0
  );
}

/** Choices made before a session exists, by option id. */
export type Picks = Readonly<Record<string, OptionChoice>>;

/** Whether the agent would take `choice` for `option`. */
function offered(option: SessionOption, choice: OptionChoice): boolean {
  switch (option.kind) {
    case "select":
      return (
        choice.kind === "select" &&
        option.values.some((value) => value.id === choice.value)
      );
    case "toggle":
      return choice.kind === "toggle";
    case "unknown":
      return false;
  }
}

/** Whether `choice` is what `option` already is. */
function already(option: SessionOption, choice: OptionChoice): boolean {
  switch (option.kind) {
    case "select":
      return choice.kind === "select" && option.current === choice.value;
    case "toggle":
      return choice.kind === "toggle" && option.on === choice.on;
    case "unknown":
      return false;
  }
}

/** `options` as they will be once `picks` are applied: what the page
 * shows before the session exists. A pick the agent does not offer (made
 * for another agent or profile) is left out. */
export function withPicks(
  options: readonly SessionOption[],
  picks: Picks,
): SessionOption[] {
  return options.map((option) => {
    const choice = picks[option.id];
    if (choice === undefined || !offered(option, choice)) {
      return option;
    }
    if (option.kind === "select" && choice.kind === "select") {
      return { ...option, current: choice.value };
    }
    if (option.kind === "toggle" && choice.kind === "toggle") {
      return { ...option, on: choice.on };
    }
    return option;
  });
}

/** The choices to start a session with: the picks the agent offers and
 * that change something, in the order the agent lists its options. */
export function choicesFor(
  options: readonly SessionOption[],
  picks: Picks,
): OptionChoice[] {
  return options.flatMap((option) => {
    const choice = picks[option.id];
    return choice !== undefined &&
      offered(option, choice) &&
      !already(option, choice)
      ? [choice]
      : [];
  });
}

/** `picks` with `choice` added (replacing one for the same option). */
export function pick(picks: Picks, choice: OptionChoice): Picks {
  return { ...picks, [choice.id]: choice };
}

/** Choose `value` of select `control`. */
export function selectChoice(
  control: SelectControl,
  value: string,
): OptionChoice {
  return { id: control.id, kind: "select", value };
}

/** Turn `control` on or off. */
export function toggleChoice(
  control: ToggleControl,
  on: boolean,
): OptionChoice {
  return { id: control.id, kind: "toggle", on };
}

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

/** What the composer's controls can show: the agent's options, or why
 * they are not there. */
export type ControlsState =
  | { kind: "loading" }
  | { kind: "failed"; message: string }
  | { kind: "ready"; controls: Controls }
  /** A terminal session: the agent's own TUI, which takes no options. */
  | { kind: "terminal" };

/** The protocol that reports a session's options (ADR 0022). */
export const OPTIONS_PROTOCOL = 9;

/** A running session's controls, from what its log last reported
 * (`null`: not yet). A node older than protocol 9 never reports them, so
 * its sessions say so instead of loading for ever. */
export function reportedControls(
  options: readonly SessionOption[] | null,
  protocol: number | null,
): ControlsState {
  if (options !== null) {
    return { kind: "ready", controls: controlsOf(options) };
  }
  return protocol !== null && protocol < OPTIONS_PROTOCOL
    ? {
        kind: "failed",
        message:
          "This machine runs an older Vessel, which does not report them.",
      }
    : { kind: "loading" };
}
