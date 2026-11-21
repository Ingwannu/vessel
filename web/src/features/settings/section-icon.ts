// Each Settings section's mark: in the sidebar's list of sections
// (features/shell) and in the command palette (features/palette), so the
// two show a section the same way.

import type { SettingsSection } from "../../data/settings";
import type { IconName } from "../../ui/kit";

/** Each section's mark, in this list and in the command palette. */
export function sectionIcon(section: SettingsSection): IconName {
  switch (section) {
    case "general":
      return "settings";
    case "appearance":
      return "sun";
    case "shortcuts":
      return "keyboard";
    case "notifications":
      return "bell";
    case "terminal":
      return "square-terminal";
    case "editor":
      return "code-xml";
    case "files":
      return "files";
    case "agents":
      return "terminal";
    case "models":
      return "plan";
    case "usage":
      return "plan";
    case "terminals":
      return "terminal";
    case "machines":
      return "machine";
    case "projects":
      return "folder";
    case "devices":
      return "monitor";
    case "sync":
      return "sync";
    case "plugins":
      return "blocks";
    case "diagnostics":
      return "stethoscope";
    case "about":
      return "home";
  }
}
