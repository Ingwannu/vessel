// The theme's three choices and the sidebar's switch for them (DESIGN.md
// §2.2, ADR 0021): an icon button at the sidebar's foot that shows the
// scheme in use and opens a menu of system, light and dark. Settings and
// the command palette offer the same three.

import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import type { ThemePreference } from "../../data/theme";
import {
  type Choice,
  IconButton,
  Menu,
  MenuGroup,
  MenuRadioGroup,
  Tooltip,
} from "../../ui/kit";

/** The choices, in the order every switch lists them. */
export const THEMES: readonly (Choice<ThemePreference> & {
  icon: "monitor" | "sun" | "moon";
})[] = [
  { value: "system", label: "System", icon: "monitor" },
  { value: "light", label: "Light", icon: "sun" },
  { value: "dark", label: "Dark", icon: "moon" },
];

/** The page's theme as the store holds it. */
export function useTheme(): ThemePreference {
  return useStore(prefs, (state) => state.values.appearance.theme);
}

export function ThemeMenu({
  choose,
}: {
  /** Keep and show a choice (the gallery only shows it). */
  choose: (preference: ThemePreference) => void;
}) {
  const preference = useTheme();
  const shown = THEMES.find((choice) => choice.value === preference);
  return (
    <Menu
      side="top"
      align="start"
      trigger={
        <Tooltip label={`Theme: ${shown?.label ?? "System"}`} side="right">
          <IconButton label="Theme" icon={shown?.icon ?? "monitor"} size="sm" />
        </Tooltip>
      }
    >
      <MenuGroup label="Theme">
        <MenuRadioGroup value={preference} onChange={choose} options={THEMES} />
      </MenuGroup>
    </Menu>
  );
}
