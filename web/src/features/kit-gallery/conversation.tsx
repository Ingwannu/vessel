// Milestone 11's conversation controls: the composer's suggestion list
// for `/` and `@`, with a note in place of rows.

import { useState } from "react";

import { Suggestions } from "../../ui/kit";
import { Section, Specimens } from "./section";

const COMMANDS = [
  {
    key: "compact",
    label: "/compact",
    detail: "Summarise the conversation · focus",
  },
  {
    key: "review",
    label: "/review",
    detail: "Review the changes on this branch",
  },
  { key: "init", label: "/init", detail: "Write a guide to this repository" },
];

const FILES = [
  {
    key: "a",
    label: "main.rs",
    detail: "crates/vessel-node/src/",
    icon: "file" as const,
  },
  {
    key: "b",
    label: "프로젝트.md",
    detail: "docs/한글-경로/",
    icon: "file" as const,
  },
];

export function Conversation() {
  const [active, setActive] = useState(0);
  return (
    <Section title="Conversation">
      <Specimens caption="the suggestion list: commands, files, and a note">
        <div className="kit-suggestions">
          <Suggestions
            id="kit-commands"
            label="Commands"
            items={COMMANDS}
            active={active}
            onPick={setActive}
            onHover={setActive}
          />
        </div>
        <div className="kit-suggestions">
          <Suggestions
            id="kit-files"
            label="Files"
            items={FILES}
            active={0}
            footer="Showing part of a large project. Type more to narrow it."
            onPick={() => undefined}
            onHover={() => undefined}
          />
        </div>
        <div className="kit-suggestions">
          <Suggestions
            id="kit-none"
            label="Files"
            items={[]}
            active={0}
            note="This session has no project to search."
            onPick={() => undefined}
            onHover={() => undefined}
          />
        </div>
      </Specimens>
    </Section>
  );
}
