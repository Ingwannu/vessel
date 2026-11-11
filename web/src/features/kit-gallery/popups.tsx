// Popups: a menu with every kind of item, a popover, tooltips, a dialog
// and the command palette, each opened from a button.

import { useState } from "react";

import {
  Button,
  type Command,
  CommandPalette,
  Dialog,
  Field,
  IconButton,
  Menu,
  MenuCheckboxItem,
  MenuGroup,
  MenuItem,
  MenuRadioGroup,
  MenuSeparator,
  MenuSubmenu,
  Popover,
  Sheet,
  TextField,
  Tooltip,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

type Sort = "newest" | "oldest" | "name";

const COMMANDS: readonly Command[] = [
  {
    id: "new",
    label: "New session",
    icon: "plus",
    group: "Actions",
    run: () => undefined,
  },
  {
    id: "machines",
    label: "Machines",
    icon: "machine",
    group: "Go to",
    run: () => undefined,
  },
  {
    id: "board",
    label: "Board",
    icon: "board",
    group: "Go to",
    run: () => undefined,
  },
  {
    id: "long",
    label: "아주 긴 세션 이름: 설정 화면의 동기화 상태 표시를 고치는 작업",
    detail: "running · /srv/build/한글-경로/프로젝트",
    group: "Sessions",
    run: () => undefined,
  },
  {
    id: "dark",
    label: "Theme: dark",
    icon: "moon",
    group: "Theme",
    run: () => undefined,
  },
];

export function Popups() {
  const [sort, setSort] = useState<Sort>("newest");
  const [archived, setArchived] = useState(false);
  const [dialog, setDialog] = useState(false);
  const [palette, setPalette] = useState(false);
  const [sheet, setSheet] = useState(false);
  const [name, setName] = useState("laptop");
  return (
    <Section title="Popups">
      <Specimens caption="a menu, a popover, tooltips (hover or focus for 400ms)">
        <Menu trigger={<Button icon="more">Menu</Button>}>
          <MenuGroup label="Session">
            <MenuItem icon="terminal" onSelect={() => undefined} shortcut="T">
              Open terminal
            </MenuItem>
            <MenuItem
              icon="move"
              onSelect={() => undefined}
              description="To another machine"
            >
              Move
            </MenuItem>
            <MenuItem icon="archive" onSelect={() => undefined} disabled>
              Archive (disabled)
            </MenuItem>
          </MenuGroup>
          <MenuSeparator />
          <MenuCheckboxItem checked={archived} onChange={setArchived}>
            Show archived
          </MenuCheckboxItem>
          <MenuSubmenu label="Sort" icon="plan">
            <MenuRadioGroup
              value={sort}
              onChange={setSort}
              options={[
                { value: "newest", label: "Newest first" },
                { value: "oldest", label: "Oldest first" },
                { value: "name", label: "이름순" },
              ]}
            />
          </MenuSubmenu>
          <MenuSeparator />
          <MenuItem
            icon="kill"
            tone="danger"
            onSelect={() => undefined}
            shortcut="⌘."
          >
            Kill
          </MenuItem>
        </Menu>
        <Popover trigger={<Button icon="link">Popover</Button>} title="Pairing">
          <p className="kit-note">
            Scan the code on the other device, or open the link there. 다른
            기기에서 코드를 찍거나 링크를 여세요.
          </p>
        </Popover>
        <Tooltip label="Copy the session id">
          <IconButton label="Copy the session id" icon="copy" />
        </Tooltip>
        <Tooltip label="Search" shortcut="⌘K">
          <Button icon="search">With a shortcut</Button>
        </Tooltip>
      </Specimens>
      <Specimens caption="a dialog and the command palette">
        <Button
          onClick={() => {
            setDialog(true);
          }}
        >
          Dialog
        </Button>
        <Button
          onClick={() => {
            setPalette(true);
          }}
        >
          Command palette
        </Button>
        <Button
          icon="menu"
          onClick={() => {
            setSheet(true);
          }}
        >
          Sheet
        </Button>
      </Specimens>
      {dialog ? (
        <Dialog
          title="Rename machine"
          onClose={() => {
            setDialog(false);
          }}
          footer={
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setDialog(false);
                }}
              >
                Cancel
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  setDialog(false);
                }}
              >
                Rename
              </Button>
            </>
          }
        >
          <Field label="Name" hint="How this page and the CLI call it.">
            <TextField value={name} onChange={setName} />
          </Field>
        </Dialog>
      ) : null}
      {sheet ? (
        <Sheet
          label="A sheet"
          onClose={() => {
            setSheet(false);
          }}
        >
          <p className="kit-note">
            A panel from the left edge, for what has no room on a phone.
          </p>
        </Sheet>
      ) : null}
      {palette ? (
        <CommandPalette
          commands={COMMANDS}
          onClose={() => {
            setPalette(false);
          }}
        />
      ) : null}
    </Section>
  );
}
