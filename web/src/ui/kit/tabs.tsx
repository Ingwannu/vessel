// Tabs (DESIGN.md §6.13): sections of one page, one shown at a time.
// agent-orchestrator's page tabs: an underline under the chosen one (2px
// in the ink), not a pill inside a pill; 14px medium labels 36px tall,
// 20px apart, over a hairline.

import "./tabs.css";

import { Tabs as Base } from "@base-ui/react/tabs";
import type { ReactNode } from "react";

import { choiceOf } from "./choice";
import { Icon, type IconName } from "./icon";

export interface Tab<V extends string> {
  value: V;
  label: string;
  icon?: IconName;
  /** What the tab shows. */
  panel: ReactNode;
}

export function Tabs<V extends string>({
  label,
  value,
  onChange,
  tabs,
}: {
  /** The tab list's name for a screen reader. */
  label: string;
  value: V;
  onChange: (value: V) => void;
  tabs: readonly Tab<V>[];
}) {
  return (
    <Base.Root
      className="tabs"
      value={value}
      onValueChange={(next: unknown) => {
        const tab = choiceOf(
          tabs.map((candidate) => ({
            value: candidate.value,
            label: candidate.label,
          })),
          next,
        );
        if (tab !== undefined) {
          onChange(tab.value);
        }
      }}
    >
      <Base.List className="tab-list" aria-label={label}>
        {tabs.map((tab) => (
          <Base.Tab key={tab.value} className="tab" value={tab.value}>
            {tab.icon === undefined ? null : <Icon name={tab.icon} size={16} />}
            {tab.label}
          </Base.Tab>
        ))}
        <Base.Indicator className="tab-indicator" />
      </Base.List>
      {tabs.map((tab) => (
        <Base.Panel key={tab.value} className="tab-panel" value={tab.value}>
          {tab.panel}
        </Base.Panel>
      ))}
    </Base.Root>
  );
}
