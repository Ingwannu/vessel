// The "Terminal profiles" group of New tab (ADR 0117): in the "+" menu and
// on an empty pane, after the built-ins, as Paseo's launcher lists it
// (`workspace-tabs/launcher/index.tsx` 222-252; `workspace-new-tab-menu.tsx`
// 37-130; `panels/new-tab-panel.tsx` 74-110, 247-262).
//
// [Decision Log]
// - Purpose: open a terminal running an agent's CLI from where a shell is
//   opened.
// - Reference: the group's label, then its rows in the list's order, each
//   with the profile's mark and name; an "Edit profiles" pencil at the
//   label's end opens the machine's Settings › Terminals. No chords.
// - Choice: the kit menu's group with its accessory, and a row's leading
//   mark (`ProviderMark`, else the terminal glyph, else the generic agent
//   mark). A row is disabled exactly when the Terminal row is.

import { useNavigate } from "@tanstack/react-router";

import { profileMark } from "../../data/terminal-profiles";
import {
  Button,
  Icon,
  IconButton,
  MenuGroup,
  MenuItem,
  MenuSeparator,
} from "../../ui/kit";
import { ProviderMark } from "../../ui/kit/provider-mark";
import type { NodeKey } from "../../wire/bridge";
import type { TerminalProfile } from "../../wire/proto";
import type { TabWords } from "./words";

/** The group a workspace offers. */
export interface ProfileGroup {
  machine: NodeKey;
  rows: readonly TerminalProfile[];
  disabled: boolean;
  launch: (profile: TerminalProfile) => void;
}

/** A profile's mark, at a menu's or a panel's size. */
export function ProfileGlyph({
  profile,
  size,
}: {
  profile: TerminalProfile;
  size: 14 | 16;
}) {
  const mark = profileMark(profile);
  switch (mark.kind) {
    case "mark":
      return <ProviderMark name={mark.name} size={size} />;
    case "terminal":
      return <Icon name="terminal" size={size} />;
    case "generic":
      return <Icon name="bot" size={size} />;
  }
}

/** Paseo's pencil: Settings › Terminals for `machine`. */
function EditProfiles({
  machine,
  words,
}: {
  machine: NodeKey;
  words: TabWords;
}) {
  const navigate = useNavigate();
  return (
    <IconButton
      icon="edit"
      label={words.editProfiles}
      size="sm"
      variant="ghost"
      onClick={() => {
        void navigate({
          to: "/settings/$section",
          params: { section: "terminals" },
          search: { machine },
        });
      }}
    />
  );
}

/** The group in a menu, after a separator; nothing without rows. */
export function ProfileMenuGroup({
  group,
  words,
}: {
  group: ProfileGroup | null;
  words: TabWords;
}) {
  if (group === null || group.rows.length === 0) {
    return null;
  }
  return (
    <>
      <MenuSeparator />
      <MenuGroup
        label={words.terminalProfiles}
        accessory={<EditProfiles machine={group.machine} words={words} />}
      >
        {group.rows.map((profile) => (
          <MenuItem
            key={profile.id}
            leading={<ProfileGlyph profile={profile} size={14} />}
            disabled={group.disabled}
            onSelect={() => {
              group.launch(profile);
            }}
          >
            {profile.name}
          </MenuItem>
        ))}
      </MenuGroup>
    </>
  );
}

/** The group on an empty pane: its label with the pencil, then a button
 * per profile. */
export function ProfilePanelGroup({
  group,
  words,
}: {
  group: ProfileGroup | null;
  words: TabWords;
}) {
  if (group === null || group.rows.length === 0) {
    return null;
  }
  return (
    <section
      className="workspace-launcher-group"
      aria-label={words.terminalProfiles}
    >
      <div className="workspace-launcher-label">
        <span>{words.terminalProfiles}</span>
        <EditProfiles machine={group.machine} words={words} />
      </div>
      {group.rows.map((profile) => (
        <Button
          key={profile.id}
          variant="ghost"
          disabled={group.disabled}
          onClick={() => {
            group.launch(profile);
          }}
        >
          <ProfileGlyph profile={profile} size={16} />
          {profile.name}
        </Button>
      ))}
    </section>
  );
}
