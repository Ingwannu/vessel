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
import { agentKey } from "../../data/agents";
import {
  AGENT_LABEL,
  type ControlsState,
  currentName,
  type SelectControl,
  selectChoice,
  slotLabel,
  toggleChoice,
} from "../../data/options";
import { pickerLabel, pickerName } from "../../data/picker";
import {
  Button,
  Icon,
  Menu,
  MenuGroup,
  MenuNote,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
  MenuSwitchItem,
  Spinner,
} from "../../ui/kit";
import type { HarnessRef } from "../../wire/proto";

export interface AgentMenuProps {
  /** The agent, and the others it could be (`null`: it cannot change, as
   * on a running session). */
  agent: {
    /** One of the four, or (protocol 11) an agent added on the machine,
     * which a running session may have. */
    value: HarnessRef;
    name: string;
    choices: readonly AgentChoice[] | null;
    onChange: (agent: HarnessRef) => void;
  };
  state: ControlsState;
  onChoose: (choice: OptionChoice) => void;
  disabled?: boolean;
  /** A form uses the same picker with a full-width Model trigger. Its
   * other fields own effort and permissions. */
  modelField?: { storedModel: string | null };
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
  currentLabel,
  onChoose,
}: {
  control: SelectControl;
  label: string;
  currentLabel?: string | null;
  onChoose: (choice: OptionChoice) => void;
}) {
  return (
    <MenuSubmenu label={label} value={currentLabel ?? currentName(control)}>
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
    case "refused":
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
  modelField,
}: AgentMenuProps) {
  const controls = state.kind === "ready" ? state.controls : null;
  const model = controls?.model ?? null;
  const storedModel = modelField?.storedModel ?? null;
  const storedChoice =
    storedModel === null
      ? undefined
      : model?.values.find((choice) => choice.id === storedModel);
  const shownModel =
    modelField === undefined
      ? null
      : (storedChoice?.name ??
        storedModel ??
        (model === null ? null : currentName(model)));
  const fieldText =
    shownModel === null ? agent.name : `${agent.name} · ${shownModel}`;
  return (
    <Menu
      side={modelField === undefined ? "top" : "bottom"}
      align="start"
      width="narrow"
      trigger={
        <Button
          variant={modelField === undefined ? "ghost" : "secondary"}
          size={modelField === undefined ? "sm" : "md"}
          shape={modelField === undefined ? "soft" : "rect"}
          disabled={disabled}
          aria-label={
            modelField === undefined
              ? pickerName(state, agent.name)
              : `Model: ${fieldText}`
          }
          end={
            modelField === undefined ? undefined : (
              <Icon name="chevron-down" size={13} />
            )
          }
          data-model-picker=""
        >
          {modelField === undefined ? (
            <TriggerLabel state={state} name={agent.name} />
          ) : (
            <span className="trigger-label">{fieldText}</span>
          )}
        </Button>
      }
    >
      <MenuSubmenu label={AGENT_LABEL} value={agent.name}>
        <MenuRadioGroup<string>
          value={agentKey(agent.value)}
          onChange={(key) => {
            // Compared by key: an added agent is an object (data/agents.ts).
            const chosen = agent.choices?.find(
              (choice) => choice.value === key,
            );
            if (chosen !== undefined) {
              agent.onChange(chosen.agent);
            }
          }}
          options={
            agent.choices ?? [
              { value: agentKey(agent.value), label: agent.name },
            ]
          }
        />
      </MenuSubmenu>
      {controls === null ? null : (
        <>
          {model === null ? null : (
            <SelectRow
              control={
                storedChoice === undefined
                  ? model
                  : { ...model, current: storedChoice.id }
              }
              label={slotLabel("model")}
              currentLabel={modelField === undefined ? null : shownModel}
              onChoose={onChoose}
            />
          )}
          {modelField !== undefined || controls.effort === null ? null : (
            <SelectRow
              control={controls.effort}
              label={slotLabel("effort")}
              onChoose={onChoose}
            />
          )}
          {modelField !== undefined ||
          controls.toggles.length + controls.selects.length === 0 ? null : (
            <>
              <MenuSeparator />
              {/* The agent's other settings, apart from the three rows the
                  page names itself: its switches, then its other choices,
                  each by the agent's own name (data/options.ts). The
                  heading says whose they are: "More" did not, and what is
                  listed here is the agent's, not Vessel's. */}
              <MenuGroup label={`${agent.name} settings`}>
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
                {controls.selects.map((select) => (
                  <SelectRow
                    key={select.id}
                    control={select}
                    label={select.name}
                    onChoose={onChoose}
                  />
                ))}
              </MenuGroup>
            </>
          )}
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
    case "refused":
      return <MenuNote>{state.message}</MenuNote>;
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
