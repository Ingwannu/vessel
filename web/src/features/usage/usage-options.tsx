// Paseo's usage options (usage/options.tsx), behind the cog that trails the
// Plan usage title and the modal's header: the machine-wide lookup switch
// with its disclosure (Admin), whether the sidebar shows the summary, and
// whether percentages read as used or remaining. The last two are the
// person's choices, kept by the bridge (ADR 0107).
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
      align="end"
      trigger={<IconButton icon="settings" label="Usage options" />}
    >
      <div className="quota-options">
        <Switch
          label="Check account limits"
          showLabel
          checked={lookup?.on ?? false}
          disabled={lookup === null}
          onChange={(checked) => {
            lookup?.set(checked);
          }}
        />
        <p className="quota-card-quiet">
          Allow this machine to read configured agents’ saved sign-ins and send
          them only to their vendors to check account limits. Only an
          administrator can change this setting.
        </p>
        <Switch
          label="Show in sidebar"
          showLabel
          checked={inSidebar}
          onChange={(checked) => {
            change({ set: "sidebar_usage", value: checked });
          }}
        />
        <Segmented
          label="Display as"
          value={usage.display}
          options={DISPLAY}
          onChange={(display) => {
            change({ set: "usage_display", value: display });
          }}
        />
      </div>
    </Popover>
  );
}
