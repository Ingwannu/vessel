// Buttons and icon buttons: every variant at every size, with an icon,
// busy, disabled, and a long Korean label.

import {
  Button,
  type ButtonSize,
  type ButtonVariant,
  IconButton,
  Tooltip,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

const VARIANTS: readonly ButtonVariant[] = [
  "primary",
  "secondary",
  "ghost",
  "danger",
  "danger-solid",
];
const SIZES: readonly ButtonSize[] = ["md", "sm", "xs"];

export function Buttons() {
  return (
    <Section title="Buttons">
      {SIZES.map((size) => (
        <Specimens key={size} caption={`${size}: each variant, with an icon`}>
          {VARIANTS.map((variant) => (
            <Button key={variant} variant={variant} size={size} icon="plus">
              {variant}
            </Button>
          ))}
        </Specimens>
      ))}
      <Specimens caption="busy, disabled, a long Korean label">
        <Button variant="primary" busy>
          Creating…
        </Button>
        <Button disabled>Disabled</Button>
        <Button variant="primary" disabled>
          Disabled
        </Button>
        <Button icon="terminal">새 세션을 이 기계에서 터미널로 열기</Button>
      </Specimens>
      <Specimens caption="icon buttons, named by tooltips">
        {SIZES.map((size) => (
          <Tooltip key={size} label={`Copy (${size})`}>
            <IconButton label={`Copy (${size})`} icon="copy" size={size} />
          </Tooltip>
        ))}
        <Tooltip label="Kill" shortcut="⌘.">
          <IconButton label="Kill" icon="kill" variant="danger" />
        </Tooltip>
        <IconButton label="Disabled" icon="more" disabled />
      </Specimens>
    </Section>
  );
}
