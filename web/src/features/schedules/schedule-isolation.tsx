import { FieldGroup, Select } from "../../ui/kit";
import type { IsolationChoice } from "../../data/schedules/isolation";

export function ScheduleIsolation({
  choice,
  change,
  legacy,
  git,
  eligible,
  disabled,
}: {
  choice: IsolationChoice;
  change: (choice: IsolationChoice) => void;
  legacy: boolean;
  git: boolean;
  eligible: boolean;
  disabled: boolean;
}) {
  return (
    <FieldGroup label="Isolation">
      <Select<IsolationChoice>
        aria-label="Isolation"
        value={choice}
        onChange={change}
        disabled={disabled}
        options={[
          ...(legacy
            ? [{ value: "legacy" as const, label: "Current behavior" }]
            : []),
          {
            value: "local",
            label: "Local",
            icon: "folder",
            disabled: !eligible,
          },
          {
            value: "worktree",
            label: "Worktree",
            icon: "git-branch",
            disabled: !git,
          },
        ]}
      />
      {choice === "worktree" && !git ? (
        <p className="schedule-form-note" role="status">
          Choose a Git project or switch to Local.
        </p>
      ) : null}
    </FieldGroup>
  );
}
