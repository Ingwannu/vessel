// Inside Settings the sidebar lists its sections in place of the machines,
// under their headings, with a way back (agent-orchestrator and GOAT both
// do this). The sections are data/settings.ts's, the one list the
// Settings page renders from; this file only draws it.
//
// The rows are links to `/settings/<section>`, followed inside the app
// (`navigate({ href })`): the route that takes a section is the Settings
// page's own, so the address is built from the list rather than from a
// route this file would have to know.

import { Link, useNavigate } from "@tanstack/react-router";
import type { MouseEvent, ReactNode } from "react";

import {
  DEFAULT_SETTINGS_SECTION,
  SETTINGS_GROUPS,
  type SettingsSection,
  settingsGroupLabel,
  settingsSectionLabel,
  settingsSectionOf,
  settingsSectionsIn,
} from "../../data/settings";
import { Icon, type IconName } from "../../ui/kit";
import { rowClass } from "./sidebar";

/** Each section's mark in the list. */
function sectionIcon(section: SettingsSection): IconName {
  switch (section) {
    case "appearance":
      return "sun";
    case "agents":
      return "terminal";
    case "models":
      return "plan";
    case "machines":
      return "machine";
    case "devices":
      return "devices";
    case "sync":
      return "sync";
    case "about":
      return "home";
  }
}

/** A link followed inside the app; a modified click is left to the
 * browser (a new tab). */
function InAppLink({
  href,
  current,
  children,
}: {
  href: string;
  current: boolean;
  children: ReactNode;
}) {
  const navigate = useNavigate();
  const follow = (event: MouseEvent<HTMLAnchorElement>) => {
    if (
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.button !== 0
    ) {
      return;
    }
    event.preventDefault();
    void navigate({ href });
  };
  return (
    <a
      className={rowClass(current)}
      href={href}
      aria-current={current ? "page" : undefined}
      onClick={follow}
    >
      {children}
    </a>
  );
}

export function SidebarSettings({ current }: { current: string | null }) {
  const shown =
    current === null
      ? DEFAULT_SETTINGS_SECTION
      : (settingsSectionOf(current) ?? DEFAULT_SETTINGS_SECTION);
  return (
    <div className="side-scroll settings-nav" aria-label="Settings sections">
      <Link className={rowClass(false)} to="/sessions">
        <Icon name="back" size={16} />
        <span className="label">Back</span>
      </Link>
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
            <InAppLink
              key={section}
              href={`/settings/${section}`}
              current={section === shown}
            >
              <Icon name={sectionIcon(section)} size={16} />
              <span className="label">{settingsSectionLabel(section)}</span>
            </InAppLink>
          ))}
        </div>
      ))}
    </div>
  );
}
