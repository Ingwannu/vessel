// The theme's three choices and the sidebar's switch for them (DESIGN.md
// §2.2, ADR 0021): an icon button at the sidebar's foot that shows the
// scheme in use and opens a menu of system, light and dark. Settings and
// the command palette offer the same three.

import { chooseTheme, theme, type ThemePreference } from "../../data/theme";
import { useStore } from "../../data/store";
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

export function ThemeMenu() {
  const preference = useStore(theme, (value) => value);
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
        <MenuRadioGroup
          value={preference}
          onChange={chooseTheme}
          options={THEMES}
        />
      </MenuGroup>
    </Menu>
  );
}
