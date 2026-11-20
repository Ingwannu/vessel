import { useState } from "react";

import { checkTitle, titleMax } from "../../data/titles";
import { Button, Dialog, TextField } from "../../ui/kit";
import type { TabWords } from "./words";

export function RenameTab({
  label,
  protocol,
  words,
  cancel,
  rename,
}: {
  label: string;
  protocol: number | null;
  words: TabWords;
  cancel: () => void;
  rename: (title: string) => Promise<string | null>;
}) {
  const [text, setText] = useState(label);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const checked = checkTitle(text, titleMax(protocol));
  const submit = () => {
    if (busy || !checked.ok) {
      return;
    }
    setBusy(true);
    setError(null);
    void rename(checked.title).then((failure) => {
      if (failure !== null) {
        setError(failure);
        setBusy(false);
      }
    });
  };
  return (
    <Dialog
      title={words.rename}
      onClose={() => {
        if (!busy) {
          cancel();
        }
      }}
      footer={
        <>
          <Button variant="ghost" disabled={busy} onClick={cancel}>
            {words.cancel}
          </Button>
          <Button
            variant="primary"
            busy={busy}
            disabled={!checked.ok}
            onClick={submit}
          >
            {words.rename}
          </Button>
        </>
      }
    >
      <TextField
        value={text}
        aria-label={words.name}
        disabled={busy}
        onChange={setText}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            submit();
          }
        }}
      />
      {error === null ? null : <p role="alert">{error}</p>}
    </Dialog>
  );
}
