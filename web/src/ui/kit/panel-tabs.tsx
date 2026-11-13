// A side panel's tabs (DESIGN.md §6.13): agent-orchestrator's inspector
// strip, icons only. 28px squares, 8px corners, 4px apart, in the passive
// ink; the hovered one on a 4% wash, the chosen one in the full ink with
// no fill; a count rides on a tab's corner. Each icon's name is its
// tooltip and its accessible name; arrows, Home and End move between
// them (Base UI's roving focus).
//
// Not the page tabs (`Tabs`): those underline labelled sections of one
// page; these switch what a narrow panel shows, where a label would not
// fit beside its neighbours.

import "./panel-tabs.css";

import { Tabs as Base } from "@base-ui/react/tabs";
import type { ReactNode } from "react";

import { choiceOf } from "./choice";
import { Icon, type IconName } from "./icon";
import { Tooltip } from "./tooltip";

export interface PanelTab<V extends string> {
  value: V;
  /** Its name: the tooltip and what a screen reader says. */
  label: string;
  icon: IconName;
  /** A count on its corner (the changed files); none when zero. */
  count?: number;
}

/** A count as the badge writes it: past 99, "99+". */
function countText(count: number): string {
  return count > 99 ? "99+" : String(count);
}

export function PanelTabs<V extends string>({
  label,
  value,
  onChange,
  tabs,
  end,
  children,
}: {
  /** The tab list's name for a screen reader. */
  label: string;
  value: V;
  onChange: (value: V) => void;
  tabs: readonly PanelTab<V>[];
  /** At the head's right: the panel's own actions. */
  end?: ReactNode;
  /** What the chosen tab shows. */
  children: ReactNode;
}) {
  return (
    <Base.Root
      className="panel-tabs"
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
      <div className="panel-tabs-head">
        <Base.List className="panel-tab-list" aria-label={label}>
          {tabs.map((tab) => {
            const count =
              tab.count === undefined || tab.count === 0
                ? null
                : countText(tab.count);
            return (
              <Tooltip
                key={tab.value}
                label={count === null ? tab.label : `${tab.label} · ${count}`}
                side="bottom"
              >
                <Base.Tab
                  className="panel-tab"
                  value={tab.value}
                  aria-label={tab.label}
                >
                  <Icon name={tab.icon} size={14} />
                  {count === null ? null : (
                    <span className="panel-tab-count" aria-hidden="true">
                      {count}
                    </span>
                  )}
                </Base.Tab>
              </Tooltip>
            );
          })}
        </Base.List>
        {end === undefined ? null : <div className="panel-tabs-end">{end}</div>}
      </div>
      <Base.Panel className="panel-tab-panel" value={value}>
        {children}
      </Base.Panel>
    </Base.Root>
  );
}
