import "./chrome.css";

import { useState } from "react";

import { tabKey } from "../../data/workspace/tab-layout";
import { Button, Dialog, Icon, IconButton, Menu } from "../../ui/kit";
import { useMedia } from "../shell/use-media";
import {
  menuRows,
  tabMenu,
  TabTitle,
  type TabAction,
  type TabChromeProps,
} from "./chrome-parts";
import { DesktopTabs } from "./desktop-tabs";
import type { TabWords } from "./words";

export function Launcher({
  actions,
  words,
}: {
  actions: readonly TabAction[];
  words: TabWords;
}) {
  return (
    <div className="workspace-launcher" aria-label={words.newTab}>
      {actions.map((action) => (
        <Button
          key={action.label}
          variant="ghost"
          disabled={action.disabled ?? false}
          onClick={action.run}
          start={<Icon name={action.icon} size={16} />}
        >
          {action.label}
        </Button>
      ))}
    </div>
  );
}

function CompactTabs(props: TabChromeProps) {
  const [open, setOpen] = useState(false);
  const { items, words } = props;
  const startTerminal = props.launch.find(
    (action) => action.label === words.terminal,
  );
  return (
    <>
      <div className="workspace-tab-bar compact">
        <span className="workspace-current-tab">
          {items.find(({ tab }) => tabKey(tab) === props.active)?.label ??
            words.newTab}
        </span>
        <Button
          variant="ghost"
          size="sm"
          aria-label={words.switcher(items.length)}
          aria-haspopup="dialog"
          aria-expanded={open}
          onClick={() => setOpen(true)}
        >
          <span className="workspace-tab-count">{items.length}</span>
        </Button>
        <Menu
          trigger={
            <IconButton
              icon="plus"
              label={words.newTab}
              size="sm"
              variant="ghost"
            />
          }
          align="end"
        >
          {menuRows(props.launch)}
        </Menu>
      </div>
      {open ? (
        <Dialog
          title={words.tabs}
          hideClose
          onClose={() => setOpen(false)}
          footer={
            <div className="workspace-switcher-footer">
              <Button
                icon="compose"
                onClick={() => {
                  setOpen(false);
                  props.select({ kind: "draft" });
                }}
              >
                {words.newAgent}
              </Button>
              <Button
                icon="terminal"
                disabled={
                  startTerminal === undefined || startTerminal.disabled === true
                }
                onClick={() => {
                  setOpen(false);
                  startTerminal?.run();
                }}
              >
                {words.newTerminal}
              </Button>
            </div>
          }
        >
          <div className="workspace-switcher">
            {items.map((item) => {
              const key = tabKey(item.tab);
              const selected = key === props.active;
              return (
                <div
                  className="workspace-switcher-row"
                  data-selected={selected}
                  key={key}
                >
                  <Button
                    variant="ghost"
                    aria-current={selected ? "true" : undefined}
                    onClick={() => {
                      props.select(item.tab);
                      setOpen(false);
                    }}
                  >
                    <TabTitle item={item} />
                  </Button>
                  <Menu
                    trigger={
                      <IconButton
                        icon="more"
                        label={`${words.actions} · ${item.label}`}
                        size="sm"
                        variant="ghost"
                      />
                    }
                    align="end"
                  >
                    {tabMenu(props, item, true)}
                  </Menu>
                </div>
              );
            })}
          </div>
        </Dialog>
      ) : null}
    </>
  );
}

export function TabChrome(props: TabChromeProps) {
  return useMedia("(width < 600px)") ? (
    <CompactTabs {...props} />
  ) : (
    <DesktopTabs {...props} />
  );
}
