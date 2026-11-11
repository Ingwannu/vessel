// The command palette (DESIGN.md §2.2, §6.13): `⌘K` or `/`, a search over
// places, machines, sessions and commands, on the kit's dialog surface.
//
// agent-orchestrator's palette, solid instead of its translucent glass (no
// blur anywhere in this design): up to 720px wide, 14vh from the top,
// radius 14, a 16px search line over 14px rows (28px tall, radius 8,
// inset 6px) grouped under small labels, the highlighted one on a 9% wash
// (10% in dark), and a 34px foot with the keys.
//
// Behaviour is Base UI's Autocomplete inline in a Dialog, as Base UI's own
// command palette example: typing filters, arrow keys move, Enter runs the
// highlighted command, Escape closes, focus goes back where it was.

import "./dialog.css";
import "./popup.css";
import "./kbd.css";
import "./command.css";

import { Autocomplete } from "@base-ui/react/autocomplete";
import { Dialog } from "@base-ui/react/dialog";

import { Icon, type IconName } from "./icon";

export interface Command {
  /** Unique within the palette. */
  id: string;
  label: string;
  /** Muted, after the label: a session's state and folder. */
  detail?: string;
  /** Searched with the label, not shown. */
  keywords?: string;
  icon?: IconName;
  /** The group it is listed under ("Go to", "Sessions", "Theme"). */
  group: string;
  run: () => void;
}

interface Group {
  value: string;
  items: Command[];
}

function grouped(commands: readonly Command[]): Group[] {
  const out: Group[] = [];
  for (const command of commands) {
    const group = out.find((candidate) => candidate.value === command.group);
    if (group === undefined) {
      out.push({ value: command.group, items: [command] });
    } else {
      group.items.push(command);
    }
  }
  return out;
}

export function CommandPalette({
  commands,
  onClose,
  placeholder = "Search machines, sessions and commands",
}: {
  commands: readonly Command[];
  onClose: () => void;
  placeholder?: string;
}) {
  return (
    <Dialog.Root
      open
      onOpenChange={(open) => {
        if (!open) {
          onClose();
        }
      }}
    >
      <Dialog.Portal>
        <Dialog.Backdrop className="dialog-backdrop" />
        <Dialog.Viewport className="dialog-viewport palette-viewport">
          <Dialog.Popup className="dialog palette" aria-label="Command palette">
            <Autocomplete.Root
              open
              inline
              items={grouped(commands)}
              itemToStringValue={(command: Command) =>
                `${command.label} ${command.detail ?? ""} ${command.keywords ?? ""}`
              }
              autoHighlight="always"
              keepHighlight
            >
              <Autocomplete.InputGroup className="palette-search">
                <Icon name="search" size={16} />
                <Autocomplete.Input
                  className="palette-input"
                  placeholder={placeholder}
                  aria-label="Search"
                  spellCheck={false}
                  autoComplete="off"
                />
              </Autocomplete.InputGroup>
              <div className="palette-scroll">
                <Autocomplete.Empty className="palette-empty">
                  Nothing matches.
                </Autocomplete.Empty>
                <Autocomplete.List className="palette-list">
                  {(group: Group) => (
                    <Autocomplete.Group
                      key={group.value}
                      items={group.items}
                      className="palette-group"
                    >
                      <Autocomplete.GroupLabel className="menu-label">
                        {group.value}
                      </Autocomplete.GroupLabel>
                      <Autocomplete.Collection>
                        {(command: Command) => (
                          <Autocomplete.Item
                            key={command.id}
                            value={command}
                            className="palette-item"
                            onClick={() => {
                              onClose();
                              command.run();
                            }}
                          >
                            {command.icon === undefined ? null : (
                              <Icon name={command.icon} size={15} />
                            )}
                            <span className="label">{command.label}</span>
                            {command.detail === undefined ? null : (
                              <span className="detail">{command.detail}</span>
                            )}
                          </Autocomplete.Item>
                        )}
                      </Autocomplete.Collection>
                    </Autocomplete.Group>
                  )}
                </Autocomplete.List>
              </div>
              <div className="palette-foot" aria-hidden="true">
                <span>
                  <kbd className="kbd key">↑↓</kbd> move
                </span>
                <span>
                  <kbd className="kbd key">↵</kbd> open
                </span>
                <span>
                  <kbd className="kbd key">esc</kbd> close
                </span>
              </div>
            </Autocomplete.Root>
          </Dialog.Popup>
        </Dialog.Viewport>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
