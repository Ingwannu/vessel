// Inside Settings the sidebar lists its sections in place of the machines,
// under their headings, with a way back (agent-orchestrator and GOAT both
// do this). The sections are data/settings.ts's, the one list the
// Settings page renders from; this file only draws it.
//
// The rows are links to `/settings/<section>`, the Settings page's own
// route (`routes/settings.tsx`), which renders every section this list
// has.

import { Link } from "@tanstack/react-router";

import {
  DEFAULT_SETTINGS_SECTION,
  SETTINGS_GROUPS,
  settingsGroupLabel,
  settingsSectionLabel,
  settingsSectionOf,
  settingsSectionsIn,
} from "../../data/settings";
import { Icon } from "../../ui/kit";
import { sectionIcon } from "../settings/section-icon";
import { rowClass } from "./sidebar";

export function SidebarSettings({ current }: { current: string | null }) {
  const shown =
    current === null
      ? DEFAULT_SETTINGS_SECTION
      : (settingsSectionOf(current) ?? DEFAULT_SETTINGS_SECTION);
  return (
    <div className="settings-nav">
      {/* Paseo's settings header: a 36px strip, the row centred, a
          hairline under it. It is not a row of the list, so it does not
          sit against Board. */}
      <div className="settings-back">
        <Link className={rowClass(false)} to="/sessions">
          <Icon name="back" size={16} />
          <span className="label">Back</span>
        </Link>
      </div>
      <div className="side-scroll" aria-label="Settings sections">
        {SETTINGS_GROUPS.map((group) => (
          <div
            key={group}
            className="side-group"
            role="group"
            aria-label={settingsGroupLabel(group)}
          >
            <div className="side-label">
              <span>{settingsGroupLabel(group)}</span>
            </div>
            {settingsSectionsIn(group).map((section) => (
              <Link
                key={section}
                className={rowClass(section === shown)}
                to="/settings/$section"
                params={{ section }}
                aria-current={section === shown ? "page" : undefined}
              >
                <Icon name={sectionIcon(section)} size={16} />
                <span className="label">{settingsSectionLabel(section)}</span>
              </Link>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
