// The permissions control (agent-orchestrator's permission menu): a quiet
// trigger led by a shield, the mode's name and a caret (the one control
// in the row that reads as a select); a 320px menu opening upward whose
// rows carry an icon, the mode's name, the agent's description of it and
// a check on the one in effect.
//
// The modes are the agent's own (its option of category `mode`), in its
// words; the reference's per-agent icons and wording are not reproduced,
// since Vessel holds no per-agent knowledge (ADR 0022).

import type { SelectControl } from "../../data/options";
import { currentName, selectChoice, slotLabel } from "../../data/options";
import { Button, Icon, Menu, MenuRadioGroup } from "../../ui/kit";
import type { OptionChoice } from "../../wire/proto";

export function PermissionsMenu({
  control,
  onChoose,
  disabled,
}: {
  control: SelectControl;
  onChoose: (choice: OptionChoice) => void;
  disabled?: boolean;
}) {
  const name = currentName(control);
  return (
    <Menu
      side="top"
      align="end"
      width="wide"
      trigger={
        <Button
          variant="ghost"
          size="sm"
          shape="soft"
          icon="shield"
          disabled={disabled}
          aria-label={`${slotLabel("permissions")}: ${name}`}
          end={<Icon name="chevron-down" size={13} />}
        >
          <span className="trigger-label">{name}</span>
        </Button>
      }
    >
      <MenuRadioGroup
        value={control.current}
        onChange={(value) => {
          onChoose(selectChoice(control, value));
        }}
        options={control.values.map((value) => ({
          value: value.id,
          label: value.name,
          icon: "shield",
          ...(value.description === undefined || value.description === null
            ? {}
            : { detail: value.description }),
        }))}
      />
    </Menu>
  );
}
