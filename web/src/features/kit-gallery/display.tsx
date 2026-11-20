// What only shows: badges, keys, spinners, dots, a banner, a skeleton, an
// empty state, rows, and the icon set at its sizes.

import { useState } from "react";

import {
  Badge,
  type BadgeLook,
  Alert,
  Banner,
  Button,
  EmptyState,
  Icon,
  ICON_NAMES,
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
      <Banner tone="error">The machine refused the pairing link.</Banner>
      <Specimens caption="alerts: Paseo's in-flow notice, by variant and size">
        <div className="kit-wide">
          <Alert
            size="sm"
            variant="warning"
            title="Plugins are trusted code"
            description="They run on this machine without sandboxing. 신뢰하는 코드만 설치하세요."
          />
          <Alert size="sm" variant="success" title="Plugins enabled" />
          <Alert
            size="sm"
            variant="error"
            title="Unable to load plugins"
            description="The machine refused it."
          >
            <Button size="sm" variant="outline">
              Retry
            </Button>
          </Alert>
          <Alert size="sm" title="Loading plugins…" />
          <Alert
            size="md"
            variant="info"
            title="Plugin host is offline"
            description="Reconnect to this machine to manage its plugins."
          />
        </div>
      </Specimens>
      <Specimens caption="a skeleton, an empty state">
        <div className="kit-wide">
          <Skeleton />
          <EmptyState
            title="No sessions yet"
            icon="board"
            action={
              <Button variant="primary" icon="plus">
                New session
              </Button>
            }
          >
            Start one on any machine. 아무 기계에서나 새 세션을 시작하세요.
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
                  label={index === 2 ? "Reconnecting" : "Connected"}
                />
              ),
            }}
            title={name}
            detail={{
              facts: [
                "Linux x86_64",
                "Direct",
                index === 2 ? "Reconnecting" : "Connected",
                "3 live sessions",
              ],
            }}
            control={
              <Badge tone={index === 2 ? "warn" : "ok"}>
                {index === 2 ? "Reconnecting" : "Connected"}
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
          {ICON_NAMES.map((name) => (
            <span key={name} className="kit-icon" aria-label={name} role="img">
              <Icon name={name} size={size} />
            </span>
          ))}
        </Specimens>
      ))}
    </Section>
  );
}
