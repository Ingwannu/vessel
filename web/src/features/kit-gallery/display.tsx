// What only shows: badges, keys, spinners, dots, a banner, a skeleton, an
// empty state, rows, and the icon set at its sizes.

import { useState } from "react";

import {
  Badge,
  type BadgeLook,
  Banner,
  Button,
  EmptyState,
  Icon,
  type IconName,
  IconButton,
  type IconSize,
  Kbd,
  Menu,
  MenuItem,
  Row,
  RowList,
  Skeleton,
  Spinner,
  StatusDot,
  Switch,
  type Tone,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

const TONES: readonly Tone[] = ["neutral", "ok", "warn", "danger", "interrupt"];
const LOOKS: readonly BadgeLook[] = ["word", "accent", "mono"];
/** The reference's sizes: 16 in buttons and tabs, 15 in menus, 14 in the
 * composer. */
const ICON_SIZES: readonly IconSize[] = [16, 15, 14];
const ICONS: readonly IconName[] = [
  "fleet",
  "sessions",
  "sync",
  "devices",
  "settings",
  "plus",
  "search",
  "close",
  "chevron",
  "chevron-down",
  "chevron-up",
  "terminal",
  "message",
  "copy",
  "more",
  "home",
  "warn",
  "check",
  "kill",
  "move",
  "link",
  "plan",
  "read",
  "edit",
  "exec",
  "search-kind",
  "qr",
  "archive",
  "dot",
  "sun",
  "moon",
  "monitor",
  "server",
  "circle-check",
  "circle-x",
  "circle-dashed",
];

export function Display() {
  const [chosen, setChosen] = useState("laptop");
  const [notify, setNotify] = useState(true);
  return (
    <Section title="Display">
      {LOOKS.map((look) => (
        <Specimens key={look} caption={`badges, ${look}`}>
          {TONES.map((tone) => (
            <Badge key={tone} tone={tone} look={look}>
              {look === "mono" ? `exit ${tone.length}` : tone}
            </Badge>
          ))}
        </Specimens>
      ))}
      <Specimens caption="keys, spinners, status dots">
        <Kbd keys={["⌘", "K"]} />
        <Kbd keys={["Ctrl", "Enter"]} look="text" />
        <Spinner size={12} />
        <Spinner size={14} />
        <Spinner size={16} label="Loading" />
        {TONES.map((tone) => (
          <StatusDot key={tone} tone={tone} label={tone} />
        ))}
      </Specimens>
      <Banner tone="warn" action={<Button size="sm">Retry</Button>}>
        Reconnecting to laptop. 연결이 끊겨 다시 잇는 중입니다.
      </Banner>
      <Banner tone="error">The node refused the pairing link.</Banner>
      <Specimens caption="a skeleton, an empty state">
        <div className="kit-wide">
          <Skeleton />
          <EmptyState
            title="No sessions yet"
            icon="sessions"
            action={
              <Button variant="primary" icon="plus">
                New session
              </Button>
            }
          >
            Start one on any node. 아무 기계에서나 새 세션을 시작하세요.
          </EmptyState>
        </div>
      </Specimens>
      <RowList label="Rows">
        {[
          "laptop",
          "build-server",
          "한국어-이름의-아주-긴-기계-이름-서울-사무실",
        ].map((name, index) => (
          <Row
            key={name}
            mark={{
              node: (
                <StatusDot
                  tone={index === 2 ? "warn" : "ok"}
                  label={index === 2 ? "Reconnecting" : "Online"}
                />
              ),
            }}
            title={name}
            detail={{
              facts: [
                "0.1.0",
                "3 sessions",
                index === 2 ? "seen 2 minutes ago" : "online",
              ],
            }}
            control={
              <Badge tone={index === 2 ? "warn" : "ok"}>
                {index === 2 ? "reconnecting" : "online"}
              </Badge>
            }
            menu={
              <Menu
                trigger={<IconButton label={`More for ${name}`} icon="more" />}
                align="end"
              >
                <MenuItem
                  icon="check"
                  onSelect={() => {
                    setChosen(name);
                  }}
                >
                  Select
                </MenuItem>
              </Menu>
            }
            selected={chosen === name}
          />
        ))}
        <Row
          mark={{ icon: "message" }}
          title="Notify when a session waits"
          detail={{
            description:
              "A notification when a session asks for permission or ends.",
          }}
          control={
            <Switch
              checked={notify}
              onChange={setNotify}
              label="Notify when a session waits"
              showLabel={false}
            />
          }
        />
      </RowList>
      {ICON_SIZES.map((size) => (
        <Specimens key={size} caption={`icons at ${String(size)}px`}>
          {ICONS.map((name) => (
            <span key={name} className="kit-icon" aria-label={name} role="img">
              <Icon name={name} size={size} />
            </span>
          ))}
        </Specimens>
      ))}
    </Section>
  );
}
