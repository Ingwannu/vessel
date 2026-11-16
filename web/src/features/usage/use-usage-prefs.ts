// The page's usage choices, kept by the bridge (ADR 0107): display, host
// and pins, plus whether the sidebar shows the summary. A change shows at
// once and the bridge's answer replaces it (data/prefs.ts).
import { useFleet } from "../../context";
import { prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { toast } from "../../ui/toasts";
import type { PreferenceChange, UsagePreferences } from "../../wire/bridge";

export function useUsagePrefs(): {
  usage: UsagePreferences;
  inSidebar: boolean;
  change: (next: PreferenceChange) => void;
} {
  const fleet = useFleet();
  const usage = useStore(prefs, (state) => state.values.usage);
  const inSidebar = useStore(prefs, (state) => state.values.sidebar.usage);
  const change = (next: PreferenceChange) => {
    void prefer(fleet, next).then((outcome) => {
      if (!outcome.ok) toast(outcome.problem.message, "error");
    });
  };
  return { usage, inSidebar, change };
}
