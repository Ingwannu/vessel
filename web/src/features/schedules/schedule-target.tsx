import {
  type ControlsState,
  NO_CONTROLS,
  selectChoice,
  type SelectControl,
} from "../../data/options";
import type { NewSessionSpec } from "../../data/schedules/form";
import { Button, FieldGroup, Select } from "../../ui/kit";
import type { OptionChoice } from "../../wire/proto";
import { AgentMenu } from "../compose/agent-menu";
import { KEEP_PLACE, type useScheduleTarget } from "./use-schedule-target";

function ScheduleOption({
  label,
  control,
  choose,
  disabled,
}: {
  label: string;
  control: SelectControl;
  choose: (choice: OptionChoice) => void;
  disabled: boolean;
}) {
  const value = control.current;
  const known = control.values.map((choice) => ({
    value: choice.id,
    label: choice.name,
  }));
  const choices = known.some((choice) => choice.value === value)
    ? known
    : [{ value, label: value, disabled: true }, ...known];
  return (
    <FieldGroup label={label}>
      <Select<string>
        aria-label={label}
        value={value}
        options={choices}
        disabled={disabled}
        onChange={(next) => {
          const option = control.values.find((entry) => entry.id === next);
          if (option !== undefined) choose(selectChoice(control, option.id));
        }}
      />
    </FieldGroup>
  );
}

export function ScheduleTargetFields({
  target,
  original,
  busy,
}: {
  target: ReturnType<typeof useScheduleTarget>;
  original: NewSessionSpec | undefined;
  busy: boolean;
}) {
  const { controls } = target;
  const place = original?.place;
  const savedProject =
    place?.kind === "project"
      ? target.projectOptions.find((entry) => entry.value === place.project)
          ?.label
      : undefined;
  const projectOptions =
    original === undefined
      ? target.projectOptions
      : [
          {
            value: KEEP_PLACE,
            label: savedProject ?? original.cwd ?? "Current location",
          } as const,
          ...target.projectOptions,
        ];
  const pickerState: ControlsState =
    target.options.kind === "ready"
      ? { kind: "ready", controls: controls ?? NO_CONTROLS }
      : target.options;
  const profileOptions = target.profileOptions.some(
    (entry) => entry.value === target.profile,
  )
    ? target.profileOptions
    : [
        { value: target.profile, label: target.profile, disabled: true },
        ...target.profileOptions,
      ];

  return (
    <>
      <FieldGroup label="Project">
        <Select
          aria-label="Project"
          value={target.project ?? (original === undefined ? null : KEEP_PLACE)}
          options={projectOptions}
          placeholder={
            target.projectsLoading ? "Loading projects…" : "Select project"
          }
          onChange={target.chooseProject}
          disabled={busy || projectOptions.length === 0}
        />
      </FieldGroup>
      {target.spec === null ? null : (
        <>
          {target.harness === null ? null : (
            <FieldGroup label="Model">
              <div className="schedule-model-picker">
                <AgentMenu
                  agent={{
                    value: target.harness,
                    name: target.harnessLabel,
                    choices: target.agentOptions,
                    onChange: target.chooseAgent,
                  }}
                  state={pickerState}
                  modelField={{ storedModel: target.originalModel }}
                  onChoose={target.chooseOption}
                  disabled={busy}
                />
              </div>
            </FieldGroup>
          )}
          {target.options.kind === "loading" ? (
            <p className="schedule-form-note" role="status">
              Loading models…
            </p>
          ) : target.options.kind === "failed" ? (
            <div className="schedule-model-error">
              <p className="schedule-form-note" role="status">
                {target.options.message}
              </p>
              <Button
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={target.again}
              >
                Try again
              </Button>
            </div>
          ) : null}
          {profileOptions.length > 1 ? (
            <FieldGroup label="Profile">
              <Select
                aria-label="Profile"
                value={target.profile}
                options={profileOptions}
                onChange={target.chooseProfile}
                disabled={busy}
              />
            </FieldGroup>
          ) : null}
          {controls?.effort == null ? null : (
            <ScheduleOption
              label="Thinking"
              control={controls.effort}
              choose={target.chooseOption}
              disabled={busy}
            />
          )}
          {controls?.permissions == null ? null : (
            <ScheduleOption
              label="Mode"
              control={controls.permissions}
              choose={target.chooseOption}
              disabled={busy}
            />
          )}
        </>
      )}
    </>
  );
}
