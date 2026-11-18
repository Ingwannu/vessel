// Host filter (Paseo's components/hosts/host-filter.tsx): a pill with the
// chosen machine's status dot, its name and a chevron, opening a menu of
// the machines to pick one. No "all" here: the page's two uses (the label
// manager, the usage modal) each act on one machine. The caller draws it
// only when there is more than one machine.
//
// [Decision Log]
// - Purpose: one pill for every screen that picks a machine, as Paseo has
//   one HostFilter for History, Schedules, Usage and the label manager.
// - Before: the label manager and the usage modal each drew their own
//   copy (workspace-labels/host-filter.tsx, usage/usage-host-filter.tsx),
//   alike but for padding, a width cap, the dot's tone and a disabled
//   state (node reviewer cleanup, row 324 round).
// - Choice: a generic kit control over a closed value type, with the
//   differences as `size` (md: the label manager's 6px by 12px, as wide as
//   its column allows; sm: the 4px by 10px pill in a dialog's title row,
//   at most 200px) and per-host tone, status word and detail, so neither
//   screen changes. Words about a machine's state stay the caller's.

import "./host-filter.css";

import type { Tone } from "./badge";
import { Icon } from "./icon";
import { Menu, MenuRadioGroup } from "./menu";
import { StatusDot } from "./status-dot";

/** One machine in the menu. */
export interface HostOption<V extends string> {
  value: V;
  /** Its name. */
  label: string;
  /** The status dot's colour and its word for a screen reader. */
  tone: Tone;
  status: string;
  /** A muted word after the name in the menu ("Offline"). */
  detail?: string;
}

export function HostFilter<V extends string>({
  hosts,
  value,
  onChange,
  disabled = false,
  size = "md",
  align = "start",
}: {
  hosts: readonly HostOption<V>[];
  value: V;
  onChange: (value: V) => void;
  disabled?: boolean;
  /** md: in a column (the label manager); sm: in a title row. */
  size?: "md" | "sm";
  align?: "start" | "end";
}) {
  const chosen = hosts.find((host) => host.value === value);
  const name = chosen?.label ?? "";
  return (
    <Menu
      align={align}
      trigger={
        <button
          type="button"
          className={size === "sm" ? "host-filter sm" : "host-filter"}
          aria-label={`Filter: ${name}`}
          disabled={disabled}
        >
          <StatusDot
            tone={chosen?.tone ?? "neutral"}
            label={chosen?.status ?? ""}
          />
          <span className="host-filter-name">{name}</span>
          <Icon name="chevron-down" size={14} />
        </button>
      }
    >
      <MenuRadioGroup<V>
        value={value}
        onChange={onChange}
        options={hosts.map((host) => ({
          value: host.value,
          label: host.label,
          ...(host.detail === undefined ? {} : { detail: host.detail }),
        }))}
      />
    </Menu>
  );
}
