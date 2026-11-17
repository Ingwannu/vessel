// The usage modal's machine choice: Paseo's HostFilter
// (components/hosts/host-filter.tsx, the pill its label manager uses too):
// the machine's status dot, its name and a chevron, opening a menu of the
// connected machines. No "all": the modal shows one machine's reports. The
// modal draws it only when more than one machine is connected.
import "./usage-host-filter.css";

import { Icon, Menu, MenuRadioGroup, StatusDot } from "../../ui/kit";
import type { NodeKey } from "../../wire/bridge";

export interface UsageHost {
  key: NodeKey;
  name: string;
}

export function UsageHostFilter({
  hosts,
  value,
  onChange,
}: {
  hosts: readonly UsageHost[];
  value: NodeKey;
  onChange: (key: NodeKey) => void;
}) {
  const name = hosts.find((host) => host.key === value)?.name ?? "";
  return (
    <Menu
      align="end"
      trigger={
        <button
          type="button"
          className="usage-host"
          aria-label={`Filter: ${name}`}
        >
          <StatusDot tone="ok" label="Connected" />
          <span className="usage-host-name">{name}</span>
          <Icon name="chevron-down" size={14} />
        </button>
      }
    >
      <MenuRadioGroup<NodeKey>
        value={value}
        onChange={onChange}
        options={hosts.map((host) => ({ value: host.key, label: host.name }))}
      />
    </Menu>
  );
}
