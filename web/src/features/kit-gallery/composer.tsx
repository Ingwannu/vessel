// The composer's parts (milestone 10): the quiet triggers and the round
// Send, the agent menu with its submenus, grouped values, switch rows and
// a note, the folder field that stays quiet until used, and the
// inspector's collapsible sections.

import { useState } from "react";

import {
  Avatar,
  Button,
  Collapsible,
  Combobox,
  Icon,
  IconButton,
  Menu,
  MenuNote,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
  MenuSwitchItem,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

type Effort = "low" | "high" | "max";

const FOLDERS = [
  { value: "~/work/vessel", detail: "3h ago" },
  { value: "/srv/build/한글-경로/프로젝트", detail: "5d ago" },
];

export function Composer() {
  const [effort, setEffort] = useState<Effort>("high");
  const [fast, setFast] = useState(false);
  const [thinking, setThinking] = useState(true);
  const [folder, setFolder] = useState("");
  return (
    <Section title="Composer">
      <Specimens caption="an agent's mark, as a card and a title bar show it">
        <Avatar letter="C" label="Claude Code" />
        <Avatar letter="X" label="Codex" />
      </Specimens>
      <Specimens caption="quiet triggers (soft), Send and Stop (round), the add button">
        <IconButton label="Add" icon="plus" shape="round" />
        <Menu
          side="top"
          width="narrow"
          trigger={
            <Button variant="ghost" size="sm" shape="soft">
              Opus <span className="kit-note">{effort}</span>
            </Button>
          }
        >
          <MenuSubmenu label="Agent" value="Claude Code">
            <MenuRadioGroup
              value="claude-code"
              onChange={() => undefined}
              options={[{ value: "claude-code", label: "Claude Code" }]}
            />
          </MenuSubmenu>
          <MenuSubmenu label="Effort" value={effort}>
            <MenuRadioGroup
              value={effort}
              onChange={setEffort}
              options={[
                { value: "low", label: "Low", group: "Quick" },
                { value: "high", label: "High", group: "Deep" },
                { value: "max", label: "Max", group: "Deep" },
              ]}
            />
          </MenuSubmenu>
          <MenuSeparator />
          <MenuSwitchItem checked={fast} onChange={setFast}>
            Fast mode
          </MenuSwitchItem>
          <MenuSwitchItem checked={thinking} onChange={setThinking}>
            사고 과정 보이기
          </MenuSwitchItem>
          <MenuNote>The agent offers no other choice.</MenuNote>
        </Menu>
        <Button variant="ghost" size="sm" shape="soft" icon="shield">
          Default
          <Icon name="chevron-down" size={13} />
        </Button>
        <IconButton
          label="Send"
          icon="arrow-up"
          variant="primary"
          shape="round"
        />
        <IconButton label="Stop" icon="stop" variant="primary" shape="round" />
        <IconButton
          label="Send (disabled)"
          icon="arrow-up"
          variant="primary"
          shape="round"
          disabled
        />
      </Specimens>
      <Specimens caption="a quiet folder field: no shell until hovered or focused">
        <div className="kit-wide">
          <Combobox
            look="ghost"
            size="sm"
            mono
            icon="folder"
            aria-label="Folder"
            value={folder}
            onChange={setFolder}
            suggestions={FOLDERS}
            placeholder="Home folder"
          />
        </div>
      </Specimens>
      <Specimens caption="collapsible sections, open and closed">
        <div className="kit-wide">
          <Collapsible title="Plan" end="1 of 3">
            <p className="kit-note">Read the overview, then fix the display.</p>
          </Collapsible>
          <Collapsible title="세부 정보" defaultOpen={false}>
            <p className="kit-note">Hidden until opened.</p>
          </Collapsible>
        </div>
      </Specimens>
    </Section>
  );
}
