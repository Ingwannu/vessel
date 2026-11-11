// One control for the agent, its model and its effort (agent-orchestrator's
// `AgentModelPicker`): a quiet trigger that reads the model and, muted,
// the effort; a 256px menu opening upward with a row for each (the
// current value at its right, a submenu of the agent's values), then the
// agent's on/off settings as switch rows and any other choice it offers.
//
// Everything in it is the agent's own (data/options.ts): what it does not
// offer has no row. While the node reads the options the trigger says
// so, and the page stays usable; a failure says why in the menu.

import type { OptionChoice } from "../../wire/proto";
import type { AgentChoice } from "../../data/agent-choices";
import {
  type ControlsState,
  currentName,
  pickerLabel,
  pickerName,
  type SelectControl,
  selectChoice,
  slotLabel,
  toggleChoice,
} from "../../data/options";
import {
  Button,
  Menu,
  MenuNote,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
  MenuSwitchItem,
  Spinner,
} from "../../ui/kit";
import type { HarnessKind } from "../../wire/proto";

export interface AgentMenuProps {
  /** The agent, and the others it could be (`null`: it cannot change, as
   * on a running session). */
  agent: {
    value: HarnessKind;
    name: string;
    choices: readonly AgentChoice[] | null;
    onChange: (agent: HarnessKind) => void;
  };
  state: ControlsState;
  onChoose: (choice: OptionChoice) => void;
  disabled?: boolean;
}

/** A select's values as the kit's choices, in the agent's words. */
function valuesOf(control: SelectControl) {
  return control.values.map((value) => ({
    value: value.id,
    label: value.name,
    ...(value.description === undefined || value.description === null
      ? {}
      : { detail: value.description }),
    ...(value.group === undefined || value.group === null
      ? {}
      : { group: value.group }),
  }));
}

/** A row that opens the values of `control`. */
function SelectRow({
  control,
  label,
  onChoose,
}: {
  control: SelectControl;
  label: string;
  onChoose: (choice: OptionChoice) => void;
}) {
  return (
    <MenuSubmenu label={label} value={currentName(control)}>
      <MenuRadioGroup
        value={control.current}
        onChange={(value) => {
          onChoose(selectChoice(control, value));
        }}
        options={valuesOf(control)}
      />
    </MenuSubmenu>
  );
}

function TriggerLabel({ state, name }: { state: ControlsState; name: string }) {
  switch (state.kind) {
    case "loading":
      return (
        <>
          <Spinner size={12} />
          <span className="trigger-label trigger-muted">Loading models…</span>
        </>
      );
    case "failed":
    case "terminal":
      return <span className="trigger-label">{name}</span>;
    case "ready": {
      const label = pickerLabel(state.controls, name);
      return (
        <>
          <span className="trigger-label">{label.main}</span>
          {label.effort === null ? null : (
            <span className="trigger-muted">{label.effort}</span>
          )}
        </>
      );
    }
  }
}

export function AgentMenu({
  agent,
  state,
  onChoose,
  disabled,
}: AgentMenuProps) {
  const controls = state.kind === "ready" ? state.controls : null;
  return (
    <Menu
      side="top"
      align="start"
      width="narrow"
      trigger={
        <Button
          variant="ghost"
          size="sm"
          shape="soft"
          disabled={disabled}
          aria-label={pickerName(state, agent.name)}
        >
          <TriggerLabel state={state} name={agent.name} />
        </Button>
      }
    >
      <MenuSubmenu label="Agent" value={agent.name}>
        <MenuRadioGroup
          value={agent.value}
          onChange={agent.onChange}
          options={agent.choices ?? [{ value: agent.value, label: agent.name }]}
        />
      </MenuSubmenu>
      {controls === null ? null : (
        <>
          {controls.model === null ? null : (
            <SelectRow
              control={controls.model}
              label={slotLabel("model")}
              onChoose={onChoose}
            />
          )}
          {controls.effort === null ? null : (
            <SelectRow
              control={controls.effort}
              label={slotLabel("effort")}
              onChoose={onChoose}
            />
          )}
          {controls.toggles.length === 0 ? null : <MenuSeparator />}
          {controls.toggles.map((toggle) => (
            <MenuSwitchItem
              key={toggle.id}
              checked={toggle.on}
              onChange={(on) => {
                onChoose(toggleChoice(toggle, on));
              }}
            >
              {toggle.name}
            </MenuSwitchItem>
          ))}
          {controls.selects.length === 0 ? null : <MenuSeparator />}
          {controls.selects.map((select) => (
            <SelectRow
              key={select.id}
              control={select}
              label={select.name}
              onChoose={onChoose}
            />
          ))}
        </>
      )}
      <AgentNote state={state} />
    </Menu>
  );
}

/** What the menu says when it has no options to show. */
function AgentNote({ state }: { state: ControlsState }) {
  switch (state.kind) {
    case "loading":
      return <MenuNote>Asking the agent what it offers…</MenuNote>;
    case "failed":
      return (
        <MenuNote>
          The agent did not say what it offers: {state.message} It starts on its
          own defaults.
        </MenuNote>
      );
    case "terminal":
      return (
        <MenuNote>
          A terminal runs the agent as it is; choose its model in it.
        </MenuNote>
      );
    case "ready":
      return null;
  }
}
