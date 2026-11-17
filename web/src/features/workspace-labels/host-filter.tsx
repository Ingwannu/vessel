// The machine choice in the label manager: Paseo's HostFilter, a pill
// with the machine's status dot, its name and a chevron, opening a menu
// of the machines. The manager draws it only when there is more than one
// machine, and it has no "all": the manager edits one machine's labels.

import type { ManagedMachine } from "../../data/workspace-labels/manager";
import type { NodeKey } from "../../wire/bridge";
import { Icon, Menu, MenuRadioGroup, StatusDot } from "../../ui/kit";

/** The word for a machine the manager cannot edit, in Paseo's terms. */
function stateWord(state: ManagedMachine["state"]): string {
  switch (state) {
    case "online":
      return "Online";
    case "offline":
      return "Offline";
    case "unsupported":
      return "Needs update";
  }
}

export function HostFilter({
  machines,
  value,
  onChange,
  disabled,
}: {
  machines: readonly ManagedMachine[];
  value: NodeKey;
  onChange: (node: NodeKey) => void;
  disabled: boolean;
}) {
  const chosen = machines.find((machine) => machine.node === value);
  const name = chosen?.name ?? "";
  const state = chosen?.state ?? "offline";
  return (
    <Menu
      trigger={
        <button
          type="button"
          className="label-host"
          aria-label={`Filter: ${name}`}
          disabled={disabled}
        >
          <StatusDot
            tone={state === "online" ? "ok" : "neutral"}
            label={stateWord(state)}
          />
          <span className="label-host-name">{name}</span>
          <Icon name="chevron-down" size={14} />
        </button>
      }
    >
      <MenuRadioGroup<NodeKey>
        value={value}
        onChange={onChange}
        options={machines.map((machine) => ({
          value: machine.node,
          label: machine.name,
          ...(machine.state === "online"
            ? {}
            : { detail: stateWord(machine.state) }),
        }))}
      />
    </Menu>
  );
}
