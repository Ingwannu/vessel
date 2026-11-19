// The sidebar's usage summary (features/usage/sidebar-usage.tsx) at the
// sidebar's width, 240 (Paseo's desktop sidebar): specimens only, never
// account data, so the row's layout is tested by a real browser
// (e2e/sidebar-usage.spec.ts) without a saved login in the e2e fleet.
import "./sidebar-usage.css";

import { QuotaPercent } from "../../quota-percent";
import { type SummarySource, SummaryRow } from "../usage/sidebar-usage";
import { Section } from "./section";

/** The coordinator's P2 r3 frame: two Codex accounts with one pinned
 * window each, then a plan source with two. */
function sources(): SummarySource[] {
  const percent = (value: number): QuotaPercent => {
    const made = QuotaPercent.from(value);
    if (made === null) throw new Error("a percent from 0 to 100");
    return made;
  };
  return [
    {
      key: "codex-a",
      mark: "codex",
      windows: [
        { key: "5h", percent: percent(12), tone: "default", short: "5h" },
      ],
    },
    {
      key: "codex-b",
      mark: "codex",
      windows: [
        { key: "5h", percent: percent(12), tone: "default", short: "5h" },
      ],
    },
    {
      key: "plan",
      mark: null,
      windows: [
        { key: "5h", percent: percent(91), tone: "danger", short: "5h" },
        { key: "wk", percent: percent(70), tone: "warning", short: "wk" },
      ],
    },
  ];
}

export function SidebarUsageSpecimen() {
  return (
    <Section title="Sidebar usage">
      <div className="kit-sidebar-usage" data-testid="sidebar-usage-240">
        <SummaryRow
          sources={sources()}
          locale={navigator.language}
          onOpen={() => undefined}
        />
      </div>
    </Section>
  );
}
