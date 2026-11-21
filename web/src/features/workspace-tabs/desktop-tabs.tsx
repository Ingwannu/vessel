import { type KeyboardEvent, useRef, useState } from "react";

import { tabKey } from "../../data/workspace/tab-layout";
import { Button, ContextMenu, IconButton, Menu } from "../../ui/kit";
import {
  menuRows,
  tabMenu,
  TabTitle,
  type TabChromeProps,
} from "./chrome-parts";
import { ProfileMenuGroup } from "./profile-rows";

export function DesktopTabs(props: TabChromeProps) {
  const [over, setOver] = useState<{
    key: string;
    side: "before" | "after";
  } | null>(null);
  const drag = useRef<string | null>(null);
  const strip = useRef<HTMLDivElement>(null);
  const { items, words } = props;
  const onKey = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (
      event.key !== "ArrowLeft" &&
      event.key !== "ArrowRight" &&
      event.key !== "Home" &&
      event.key !== "End"
    ) {
      return;
    }
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? items.length - 1
          : (items.length + index + (event.key === "ArrowLeft" ? -1 : 1)) %
            items.length;
    event.preventDefault();
    const source = items[index];
    const target = items[next];
    if (target === undefined || source === undefined) {
      return;
    }
    if (event.altKey) {
      props.move(
        tabKey(source.tab),
        tabKey(target.tab),
        next < index ? "before" : "after",
      );
    } else {
      props.select(target.tab);
      const buttons =
        strip.current?.querySelectorAll<HTMLButtonElement>('[role="tab"]');
      buttons?.[next]?.focus();
    }
  };
  return (
    <div className="workspace-tab-bar">
      <div
        className="workspace-tab-strip"
        ref={strip}
        role="tablist"
        aria-label={words.workspace}
      >
        {items.map((item, index) => {
          const key = tabKey(item.tab);
          const selected = key === props.active;
          return (
            <div
              className="workspace-tab"
              data-selected={selected}
              data-drop={over?.key === key ? over.side : undefined}
              key={key}
              onDragOver={(event) => {
                if (drag.current === null || drag.current === key) {
                  setOver(null);
                  return;
                }
                event.preventDefault();
                const rect = event.currentTarget.getBoundingClientRect();
                setOver({
                  key,
                  side:
                    event.clientX < rect.left + rect.width / 2
                      ? "before"
                      : "after",
                });
              }}
              onDrop={(event) => {
                event.preventDefault();
                if (drag.current !== null && over?.key === key) {
                  props.move(drag.current, key, over.side);
                }
                drag.current = null;
                setOver(null);
              }}
            >
              <ContextMenu
                trigger={
                  <Button
                    role="tab"
                    aria-selected={selected}
                    tabIndex={
                      selected || (props.active === null && index === 0)
                        ? 0
                        : -1
                    }
                    title={item.label}
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      props.select(item.tab);
                    }}
                    onKeyDown={(event) => {
                      onKey(event, index);
                    }}
                    draggable
                    onDragStart={(event) => {
                      drag.current = key;
                      event.dataTransfer.effectAllowed = "move";
                      event.dataTransfer.setData(
                        "application/x-vessel-tab",
                        "tab",
                      );
                    }}
                    onDragEnd={() => {
                      drag.current = null;
                      setOver(null);
                    }}
                  >
                    <TabTitle item={item} />
                  </Button>
                }
              >
                {tabMenu(props, item, false)}
              </ContextMenu>
              <span className="workspace-tab-close">
                <IconButton
                  label={`${words.close} · ${item.label}`}
                  icon="close"
                  size="xs"
                  variant="ghost"
                  onClick={() => {
                    props.close(item);
                  }}
                />
              </span>
            </div>
          );
        })}
      </div>
      <Menu
        align="end"
        trigger={
          <IconButton
            icon="plus"
            label={words.newTab}
            size="sm"
            variant="ghost"
          />
        }
      >
        {menuRows(props.launch)}
        <ProfileMenuGroup group={props.profiles} words={words} />
      </Menu>
    </div>
  );
}
