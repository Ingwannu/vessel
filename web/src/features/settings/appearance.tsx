// Settings › Appearance: how this page looks on this device. Today the
// colour scheme only; the theme's three choices are the kit's Segmented
// (as the sidebar's switch and the palette offer the same three).

import { useStore } from "../../data/store";
import { chooseTheme, theme } from "../../data/theme";
import { Row, RowList, Segmented } from "../../ui/kit";
import { THEMES } from "../theme/theme";
import { SettingsPage, SettingsSection } from "./settings-page";

export function AppearanceSection() {
  const preference = useStore(theme, (value) => value);
  return (
    <SettingsPage lead="How this page looks on this device.">
      <SettingsSection title="Theme">
        <RowList label="Theme">
          <Row
            title="Colour scheme"
            detail={{
              description: "Follow the system, or stay light or dark.",
            }}
            control={
              <Segmented
                label="Colour scheme"
                size="sm"
                value={preference}
                onChange={chooseTheme}
                options={THEMES}
              />
            }
          />
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}
