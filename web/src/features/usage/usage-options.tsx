// Paseo's usage options (usage/options.tsx), behind the cog that trails the
// Plan usage title and the modal's title row: a 340px popover with no
// visible heading on wide screens, rows in Paseo's SettingsSwitch and
// SettingsRow shape (label and hint at the left, the control at the
// right, a hairline between rows). The machine-wide lookup switch keeps
// our consent hint (ADR 0042); "Summary in sidebar" and "Percentages" are
// the person's choices, kept by the bridge (ADR 0107).
import type { ReactNode } from "react";

import type { QuotaDisplay } from "../../data/account-usage/values";
import {
  type Choice,
  IconButton,
  Popover,
  Segmented,
  Switch,
} from "../../ui/kit";
import { useUsagePrefs } from "./use-usage-prefs";

const DISPLAY: readonly Choice<QuotaDisplay>[] = [
  { value: "used", label: "Used" },
  { value: "remaining", label: "Remaining" },
];

/** Paseo's SettingsSwitch / SettingsRow: words at the left, the control
 * at the right. */
function OptionRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="quota-option">
      <div className="quota-option-text">
        <span className="quota-option-label">{label}</span>
        {hint === undefined ? null : (
          <span className="quota-option-hint">{hint}</span>
        )}
      </div>
      {children}
    </div>
  );
}

export function UsageOptions({
  lookup,
}: {
  /** The machine's switch; absent until its reports are read. */
  lookup: { on: boolean; set: (on: boolean) => void } | null;
}) {
  const { usage, inSidebar, change } = useUsagePrefs();
  return (
    <Popover
      title="Usage options"
      showTitle={false}
      width="settings"
      align="end"
      trigger={<IconButton icon="settings" label="Usage options" />}
    >
      <div className="quota-options">
        <OptionRow
          label="Check account limits"
          hint="Allow this machine to read configured agents’ saved sign-ins and send them only to their vendors to check account limits. Only an administrator can change this setting."
        >
          <Switch
            label="Check account limits"
            checked={lookup?.on ?? false}
            disabled={lookup === null}
            onChange={(checked) => {
              lookup?.set(checked);
            }}
          />
        </OptionRow>
        <OptionRow
          label="Summary in sidebar"
          hint="Pinned windows show in the sidebar footer"
        >
          <Switch
            label="Summary in sidebar"
            checked={inSidebar}
            onChange={(checked) => {
              change({ set: "sidebar_usage", value: checked });
            }}
          />
        </OptionRow>
        <OptionRow label="Percentages">
          <Segmented
            label="Percentages"
            size="sm"
            value={usage.display}
            options={DISPLAY}
            onChange={(display) => {
              change({ set: "usage_display", value: display });
            }}
          />
        </OptionRow>
      </div>
    </Popover>
  );
}
