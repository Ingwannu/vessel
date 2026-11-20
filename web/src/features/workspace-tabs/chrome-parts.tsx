import {
  type CloseScope,
  tabKey,
  tabsToClose,
  type WorkspaceTab,
} from "../../data/workspace/tab-layout";
import { Icon, type IconName, MenuItem, MenuSeparator } from "../../ui/kit";
import type { TabWords } from "./words";

export interface TabItem {
  tab: WorkspaceTab;
  label: string;
  icon: IconName;
  dirty: boolean;
}

export interface TabAction {
  label: string;
  icon: IconName;
  run: () => void;
  disabled?: boolean;
  danger?: boolean;
  tooltip?: string;
  shortcut?: string;
}

export interface TabChromeProps {
  items: readonly TabItem[];
  active: string | null;
  words: TabWords;
  launch: readonly TabAction[];
  actions: (item: TabItem) => readonly TabAction[];
  select: (tab: WorkspaceTab) => void;
  close: (item: TabItem) => void;
  closeMany: (tabs: readonly WorkspaceTab[], title: string) => void;
  move: (source: string, target: string, side: "before" | "after") => void;
}

function actionsFor(
  props: TabChromeProps,
  item: TabItem,
  compact: boolean,
): TabAction[] {
  const { words } = props;
  const directions: readonly {
    scope: CloseScope;
    label: string;
    icon: IconName;
  }[] = [
    {
      scope: "before",
      label: compact ? words.above : words.before,
      icon: "arrow-left-to-line",
    },
    {
      scope: "after",
      label: compact ? words.below : words.after,
      icon: "arrow-right-to-line",
    },
    { scope: "others", label: words.others, icon: "copy-x" },
  ];
  const tabs = props.items.map(({ tab }) => tab);
  return directions.map(({ scope, label, icon }) => {
    const targets = tabsToClose(tabs, tabKey(item.tab), scope);
    return {
      label,
      icon,
      disabled: targets.length === 0,
      run: () => {
        props.closeMany(targets, label);
      },
    };
  });
}

export function menuRows(actions: readonly TabAction[]) {
  return actions.map((action) => (
    <MenuItem
      key={action.label}
      icon={action.icon}
      onSelect={action.run}
      disabled={action.disabled ?? false}
      {...(action.shortcut === undefined ? {} : { shortcut: action.shortcut })}
      {...(action.tooltip === undefined ? {} : { tooltip: action.tooltip })}
      tone={action.danger === true ? "danger" : "default"}
    >
      {action.label}
    </MenuItem>
  ));
}

export function tabMenu(
  props: TabChromeProps,
  item: TabItem,
  compact: boolean,
) {
  const actions = props.actions(item);
  return (
    <>
      {menuRows(actions)}
      {actions.length === 0 ? null : <MenuSeparator />}
      {menuRows(actionsFor(props, item, compact))}
      <MenuSeparator />
      {menuRows([
        {
          label: props.words.close,
          icon: "close",
          danger: true,
          run: () => {
            props.close(item);
          },
        },
      ])}
    </>
  );
}

export function TabTitle({ item }: { item: TabItem }) {
  return (
    <>
      <Icon name={item.icon} size={14} />
      <span className="workspace-tab-name">{item.label}</span>
      {item.dirty ? (
        <span className="workspace-tab-dirty" aria-hidden="true" />
      ) : null}
    </>
  );
}
