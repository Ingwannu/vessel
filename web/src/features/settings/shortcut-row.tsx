// One action in Settings › Shortcuts (agent-orchestrator's row): its
// name and a "Modified" mark, its keys as the platform writes them ("or"
// between two), then Edit and a menu of Add another, Remove a key and
// Restore default. A fixed action says "Fixed"; a clash kept in the file
// is said under the name.

import type { ReactNode } from "react";

import type { ShortcutRow as Row } from "../../data/shortcuts/listing";
import { definition } from "../../data/shortcuts/registry";
import {
  Badge,
  IconButton,
  Kbd,
  Menu,
  MenuItem,
  MenuSeparator,
  Tooltip,
} from "../../ui/kit";

export function ShortcutRowView({
  row,
  recorder,
  onEdit,
  onAdd,
  onRemove,
  onRestore,
}: {
  row: Row;
  /** The recorder, while this row records. */
  recorder: ReactNode;
  onEdit: () => void;
  onAdd: () => void;
  onRemove: (index: number) => void;
  onRestore: () => void;
}) {
  return (
    <li className="shortcut-row">
      <div className="shortcut-name">
        <span className="shortcut-label">{row.label}</span>
        {row.modified ? <Badge look="accent">Modified</Badge> : null}
        {row.clashes.length === 0 ? null : (
          <span className="shortcut-clash">
            Also used by{" "}
            {row.clashes.map((action) => definition(action).label).join(", ")}
          </span>
        )}
      </div>
      {recorder ?? (
        <div className="shortcut-keys">
          {row.words.length === 0 ? (
            <span className="shortcut-unset">Not set</span>
          ) : (
            row.words.map((words, index) => (
              <span key={words} className="shortcut-key">
                {index > 0 ? <span className="shortcut-or">or</span> : null}
                <Kbd keys={[words]} />
              </span>
            ))
          )}
          {row.fixed ? (
            <span className="shortcut-fixed">Fixed</span>
          ) : (
            <>
              <Tooltip label={`Change ${row.label}`}>
                <IconButton
                  label={`Change ${row.label}`}
                  icon="edit"
                  size="xs"
                  onClick={onEdit}
                />
              </Tooltip>
              <Menu
                align="end"
                trigger={
                  <IconButton
                    label={`More options for ${row.label}`}
                    icon="more"
                    size="xs"
                  />
                }
              >
                <MenuItem
                  icon="plus"
                  onSelect={onAdd}
                  disabled={row.bindings.length >= 2}
                >
                  Add alternative
                </MenuItem>
                {row.words.map((words, index) => (
                  <MenuItem
                    key={words}
                    icon="close"
                    onSelect={() => {
                      onRemove(index);
                    }}
                  >
                    Remove {words}
                  </MenuItem>
                ))}
                {row.modified ? (
                  <>
                    <MenuSeparator />
                    <MenuItem icon="back" onSelect={onRestore}>
                      Restore default
                    </MenuItem>
                  </>
                ) : null}
              </Menu>
            </>
          )}
        </div>
      )}
    </li>
  );
}
