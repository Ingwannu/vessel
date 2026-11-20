// A card's kebab in the Pull request pane (TRACKER 430 on the page):
// Paseo's `ActivityKebab` (`pull-request-panel/pane.tsx`, pinned 0.15.4):
// Add to chat, Copy (the markdown source as it came), Open on the forge,
// as `data/workspace/pull/card-menu.ts` offers them; the kit's Menu,
// aligned to the kebab's end.

import type { CardAct } from "../../../data/workspace/pull/card-menu";
import type { PullWords } from "../../../data/workspace/pull/words";
import { IconButton, Menu, MenuItem } from "../../../ui/kit";
import { toast } from "../../../ui/toasts";

export function Kebab({
  acts,
  label,
  words,
  forge,
  url,
  copyText,
  onAdd,
}: {
  acts: readonly CardAct[];
  label: string;
  words: PullWords;
  forge: string;
  url: string | null;
  copyText: string | null;
  onAdd: () => void;
}) {
  if (acts.length === 0) return null;
  return (
    <span className="pull-kebab">
      <Menu
        align="end"
        trigger={<IconButton label={label} icon="more" size="xs" />}
      >
        {acts.map((act) => {
          switch (act) {
            case "addToChat":
              return (
                <MenuItem key={act} icon="message" onSelect={onAdd}>
                  {words.addToChat}
                </MenuItem>
              );
            case "copy":
              return (
                <MenuItem
                  key={act}
                  icon="copy"
                  onSelect={() => {
                    if (copyText === null) return;
                    navigator.clipboard.writeText(copyText).catch(() => {
                      toast("The clipboard refused the text", "error");
                    });
                  }}
                >
                  {words.copy}
                </MenuItem>
              );
            case "open":
              return (
                <MenuItem
                  key={act}
                  icon="move"
                  onSelect={() => {
                    if (url !== null) window.open(url, "_blank", "noreferrer");
                  }}
                >
                  {words.openOn(forge)}
                </MenuItem>
              );
          }
        })}
      </Menu>
    </span>
  );
}
