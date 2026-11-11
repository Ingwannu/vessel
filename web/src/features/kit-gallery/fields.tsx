// Fields: text, text area and combobox shells at rest, filled, focused (by
// the screens), disabled and invalid, with hints, errors and Korean text.

import { useState } from "react";

import {
  Combobox,
  CommandLine,
  Field,
  Kbd,
  TextArea,
  TextField,
} from "../../ui/kit";
import { Section, Specimens } from "./section";

const FOLDERS = [
  { value: "~/work/vessel", detail: "3 sessions" },
  { value: "~/work/vessel-docs", detail: "yesterday" },
  { value: "/srv/build/한글-경로/프로젝트", detail: "last week" },
];

export function Fields() {
  const [name, setName] = useState("");
  const [path, setPath] = useState("~/work/vessel");
  const [search, setSearch] = useState("");
  const [folder, setFolder] = useState("");
  const [prompt, setPrompt] = useState(
    "설정 화면의 동기화 상태가 오래된 세대 번호를 보여 줍니다. 구성원이 적용한 세대를 어떻게 보고하는지 확인하고 표시를 고쳐 주세요.\nThen run the tests.",
  );
  return (
    <Section title="Fields">
      <div className="kit-grid">
        <Field label="Name" hint="How this page and the CLI call it.">
          <TextField value={name} onChange={setName} placeholder="laptop" />
        </Field>
        <Field label="Folder">
          <TextField mono value={path} onChange={setPath} />
        </Field>
        <Field label="Search, small, with an icon and a shortcut">
          <TextField
            size="sm"
            icon="search"
            role="searchbox"
            value={search}
            onChange={setSearch}
            placeholder="Search"
            end={<Kbd keys={["⌘", "K"]} look="text" />}
          />
        </Field>
        <Field
          label="Pairing link"
          error="This link has expired; ask for a new one."
        >
          <TextField
            mono
            value="vessel://pair#expired"
            onChange={() => undefined}
          />
        </Field>
        <Field label="Disabled" disabled>
          <TextField value="read only" onChange={() => undefined} disabled />
        </Field>
        <Field label="Folder (suggests, takes anything)">
          <Combobox
            mono
            value={folder}
            onChange={setFolder}
            suggestions={FOLDERS}
            placeholder="the machine's home folder"
          />
        </Field>
      </div>
      <Specimens caption="a command to run elsewhere, short and long">
        <div className="kit-wide">
          <CommandLine command="vessel node pair" />
          <CommandLine command="vessel node start && vessel node pair && vessel node status && vessel nodes list" />
        </div>
      </Specimens>
      <Specimens caption="a text area grows with its text, then scrolls">
        <div className="kit-wide">
          <Field label="Prompt">
            <TextArea value={prompt} onChange={setPrompt} maxRows={6} />
          </Field>
        </div>
      </Specimens>
    </Section>
  );
}
