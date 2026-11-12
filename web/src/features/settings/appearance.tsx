// Settings › Appearance: how this page looks (DESIGN.md §3.1). The
// colour scheme; GOAT's interface and code font sizes and its chat
// width; Paseo's "Chat display" (how much of an agent's work the
// conversation shows). Each is a preference the bridge keeps (ADR 0028),
// so it holds across launches and in every tab.

import { useFleet } from "../../context";
import { CODE_SIZES, INTERFACE_SIZES, prefer, prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import { chooseTheme } from "../../data/theme";
import { toast } from "../../ui/toasts";
import { type Choice, Row, RowList, Segmented, Select } from "../../ui/kit";
import type {
  ContentWidth,
  Conversation,
  PreferenceChange,
} from "../../wire/bridge";
import { THEMES } from "../theme/theme";
import { KeptNote } from "./kept-note";
import { SettingsPage, SettingsSection } from "./settings-page";

/** `sizes` as a select's options ("14 px"). */
function sizeChoices(sizes: readonly number[]): Choice<string>[] {
  return sizes.map((size) => ({
    value: String(size),
    label: `${String(size)} px`,
  }));
}

const WIDTHS: readonly Choice<ContentWidth>[] = [
  { value: "comfortable", label: "Comfortable" },
  { value: "wide", label: "Wide" },
  { value: "full", label: "Full" },
];

const CONVERSATION: readonly Choice<Conversation>[] = [
  { value: "full", label: "Full" },
  { value: "collapse_tools", label: "Collapse tool calls" },
];

export function AppearanceSection() {
  const fleet = useFleet();
  const appearance = useStore(prefs, (state) => state.values.appearance);
  const change = (next: PreferenceChange) => {
    void prefer(fleet, next).then((outcome) => {
      if (!outcome.ok) {
        toast(outcome.problem.message, "error");
      }
    });
  };
  return (
    <SettingsPage lead="How this page looks, on every device that opens it from this machine.">
      <KeptNote />
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
                value={appearance.theme}
                onChange={(theme) => {
                  void chooseTheme(fleet, theme);
                }}
                options={THEMES}
              />
            }
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="Typography">
        <RowList label="Typography">
          <Row
            title="Interface font"
            detail={{
              description: "Everything outside code blocks and the terminal.",
            }}
            control={
              <Select
                aria-label="Interface font size"
                size="sm"
                width="fit"
                align="end"
                value={String(appearance.interface_size)}
                options={sizeChoices(INTERFACE_SIZES)}
                onChange={(size) => {
                  change({ set: "interface_size", value: Number(size) });
                }}
              />
            }
          />
          <Row
            title="Code font"
            detail={{
              description: "Code blocks, tool output and the terminal.",
            }}
            control={
              <Select
                aria-label="Code font size"
                size="sm"
                width="fit"
                align="end"
                value={String(appearance.code_size)}
                options={sizeChoices(CODE_SIZES)}
                onChange={(size) => {
                  change({ set: "code_size", value: Number(size) });
                }}
              />
            }
          />
        </RowList>
      </SettingsSection>
      <SettingsSection title="Conversation">
        <RowList label="Conversation">
          <Row
            title="Content width"
            detail={{
              description:
                "How wide the conversation and the composer grow on large screens.",
            }}
            control={
              <Select
                aria-label="Content width"
                size="sm"
                width="fit"
                align="end"
                value={appearance.content_width}
                options={WIDTHS}
                onChange={(width) => {
                  change({ set: "content_width", value: width });
                }}
              />
            }
          />
          <Row
            title="Conversation display"
            detail={{
              description:
                "How much of an agent's work the conversation shows.",
            }}
            control={
              <Select
                aria-label="Conversation display"
                size="sm"
                width="fit"
                align="end"
                value={appearance.conversation}
                options={CONVERSATION}
                onChange={(display) => {
                  change({ set: "conversation", value: display });
                }}
              />
            }
          />
        </RowList>
      </SettingsSection>
    </SettingsPage>
  );
}
