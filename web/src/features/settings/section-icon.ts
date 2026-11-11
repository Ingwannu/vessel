// Each Settings section's mark: in the sidebar's list of sections
// (features/shell) and in the command palette (features/palette), so the
// two show a section the same way.

import type { SettingsSection } from "../../data/settings";
import type { IconName } from "../../ui/kit";

/** Each section's mark, in this list and in the command palette. */
export function sectionIcon(section: SettingsSection): IconName {
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
