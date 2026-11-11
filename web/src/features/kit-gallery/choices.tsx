// Choosing controls: Select at both sizes and looks, with groups, details,
// a disabled option, a long Korean option, and one at the window's right
// edge; Segmented, Switch, Checkbox and Tabs in each state.

import { useState } from "react";

import {
  Checkbox,
  type Choice,
  Field,
  FieldGroup,
  Segmented,
  Select,
  Switch,
  Tabs,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

type Model = "opus" | "sonnet" | "haiku" | "long" | "gone";

const MODELS: readonly Choice<Model>[] = [
  { value: "opus", label: "Opus", detail: "route: default", group: "Claude" },
  { value: "sonnet", label: "Sonnet", detail: "route: fast", group: "Claude" },
  { value: "haiku", label: "Haiku", group: "Claude" },
  {
    value: "long",
    label:
      "아주 긴 이름의 모델 경로: 회사 내부 게이트웨이를 거쳐 가는 기본 경로",
    detail: "route: gateway-internal-default",
    group: "Gateway",
  },
  { value: "gone", label: "Retired route", disabled: true, group: "Gateway" },
];

type Mode = "structured" | "terminal";
type Scope = "mine" | "all" | "archived";
type Pane = "general" | "harnesses" | "korean";

export function Choices() {
  const [model, setModel] = useState<Model | null>("opus");
  const [none, setNone] = useState<Model | null>(null);
  const [mode, setMode] = useState<Mode>("structured");
  const [scope, setScope] = useState<Scope>("all");
  const [on, setOn] = useState(true);
  const [off, setOff] = useState(false);
  const [pane, setPane] = useState<Pane>("general");
  return (
    <Section title="Choices">
      <div className="kit-grid">
        <Field label="Model (filled, groups and details)">
          <Select value={model} onChange={setModel} options={MODELS} />
        </Field>
        <Field label="Nothing chosen yet">
          <Select
            value={none}
            onChange={setNone}
            options={MODELS}
            placeholder="Choose a model"
          />
        </Field>
        <Field label="Disabled">
          <Select
            value="sonnet"
            onChange={setModel}
            options={MODELS}
            disabled
          />
        </Field>
      </div>
      <Specimens caption="small, ghost, fitted to the label; the last at the window's edge">
        <Select
          size="sm"
          look="ghost"
          width="fit"
          value={model}
          onChange={setModel}
          options={MODELS}
        />
        <span className="kit-push" />
        <Select
          size="sm"
          width="fit"
          align="end"
          value={model}
          onChange={setModel}
          options={MODELS}
        />
      </Specimens>
      <Specimens caption="segmented: labelled, small with icons, one option disabled">
        <FieldGroup label="Mode">
          <Segmented
            label="Mode"
            value={mode}
            onChange={setMode}
            options={[
              { value: "structured", label: "Structured" },
              { value: "terminal", label: "터미널" },
            ]}
          />
        </FieldGroup>
        <Segmented
          label="Scope"
          size="sm"
          value={scope}
          onChange={setScope}
          options={[
            { value: "mine", label: "Mine", icon: "home" },
            { value: "all", label: "All", icon: "sessions" },
            {
              value: "archived",
              label: "Archived",
              icon: "archive",
              disabled: true,
            },
          ]}
        />
      </Specimens>
      <Specimens caption="switch and checkbox: on, off, disabled, a long Korean label">
        <Switch
          checked={on}
          onChange={setOn}
          label="Notify when a session waits"
        />
        <Switch checked={off} onChange={setOff} label="Quiet" />
        <Switch checked onChange={() => undefined} label="Locked on" disabled />
        <Checkbox checked={on} onChange={setOn} label="Include archived" />
        <Checkbox
          checked={off}
          onChange={setOff}
          label="권한 요청이 오면 이 기계에서 소리로 알리기"
        />
        <Checkbox
          checked={false}
          onChange={() => undefined}
          label="Disabled"
          disabled
        />
      </Specimens>
      <Tabs
        label="Settings sections"
        value={pane}
        onChange={setPane}
        tabs={[
          {
            value: "general",
            label: "General",
            icon: "settings",
            panel: <p className="kit-note">General settings go here.</p>,
          },
          {
            value: "harnesses",
            label: "Harnesses",
            icon: "terminal",
            panel: <p className="kit-note">Harness settings go here.</p>,
          },
          {
            value: "korean",
            label: "한국어 이름의 탭",
            panel: <p className="kit-note">탭 내용이 여기에 옵니다.</p>,
          },
        ]}
      />
    </Section>
  );
}
