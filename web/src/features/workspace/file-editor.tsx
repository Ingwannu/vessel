import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { bracketMatching } from "@codemirror/language";
import { search, searchKeymap } from "@codemirror/search";
import {
  Annotation,
  Compartment,
  EditorState,
  Transaction,
} from "@codemirror/state";
import {
  drawSelection,
  EditorView,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { getCM, vim } from "@replit/codemirror-vim";
import { useEffect, useEffectEvent, useRef } from "react";

import { prefs } from "../../data/prefs";
import { useStore } from "../../data/store";
import type { RelPath } from "../../wire/proto";
import "./file-editor.css";

const replaceDraft = Annotation.define<boolean>();
const editability = new Compartment();
const bindings = new Compartment();

/** CodeMirror owns input and selection. The file view owns the draft and its
 * revision-checked save so an editor mode cannot bypass a disk conflict. */
export function FileEditor({
  path,
  draft,
  saving,
  onChange,
  onSave,
}: {
  path: RelPath;
  draft: string;
  saving: boolean;
  onChange: (draft: string) => void;
  onSave: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<EditorView | null>(null);
  const status = useRef<HTMLDivElement>(null);
  const vimEnabled = useStore(
    prefs,
    (state) => state.values.editor.vim_keybindings,
  );
  const currentDraft = useEffectEvent(() => draft);
  const changeDraft = useEffectEvent(onChange);

  useEffect(() => {
    if (host.current === null) {
      return;
    }
    const source = currentDraft();
    // Keep CRLF as the inserted line break for files already using it.
    // An explicit LF for mixed input also retains their literal CR bytes.
    const separator =
      source.includes("\r\n") && !/(^|[^\r])\n/u.test(source) ? "\r\n" : "\n";
    const view = new EditorView({
      parent: host.current,
      state: EditorState.create({
        doc: source,
        extensions: [
          EditorState.lineSeparator.of(separator),
          EditorView.contentAttributes.of({ "aria-label": `Edit ${path}` }),
          lineNumbers(),
          history(),
          drawSelection(),
          bracketMatching(),
          search({ top: true }),
          keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
          /\.(?:md|markdown)$/iu.test(path) ? EditorView.lineWrapping : [],
          bindings.of([]),
          editability.of([]),
          EditorView.updateListener.of((update) => {
            if (
              update.docChanged &&
              !update.transactions.some((transaction) =>
                transaction.annotation(replaceDraft),
              )
            ) {
              changeDraft(update.state.sliceDoc());
            }
          }),
        ],
      }),
    });
    editor.current = view;
    return () => {
      editor.current = null;
      view.destroy();
    };
  }, [path]);

  useEffect(() => {
    const view = editor.current;
    if (view === null || draft === view.state.sliceDoc()) {
      return;
    }
    const replacement = view.state.toText(draft);
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: replacement },
      selection: {
        anchor: Math.min(view.state.selection.main.head, replacement.length),
      },
      annotations: [replaceDraft.of(true), Transaction.addToHistory.of(false)],
    });
  }, [draft]);

  useEffect(() => {
    editor.current?.dispatch({
      effects: editability.reconfigure([
        EditorState.readOnly.of(saving),
        EditorView.editable.of(!saving),
      ]),
    });
  }, [path, saving]);

  useEffect(() => {
    const view = editor.current;
    if (view === null) {
      return;
    }
    view.dispatch({ effects: bindings.reconfigure(vimEnabled ? vim() : []) });
    const cm = vimEnabled ? getCM(view) : null;
    const statusLine = status.current;
    if (cm === null || statusLine === null) {
      return;
    }
    statusLine.textContent = "Vim normal";
    const showMode = (event: { mode: string }) => {
      statusLine.textContent = `Vim ${event.mode}`;
    };
    cm.on("vim-mode-change", showMode);
    return () => cm.off("vim-mode-change", showMode);
  }, [path, vimEnabled]);

  return (
    <div
      className="file-editor"
      onKeyDownCapture={(event) => {
        if (
          (event.metaKey || event.ctrlKey) &&
          event.key.toLowerCase() === "s"
        ) {
          // Save belongs to the enclosing file, even in Vim Normal mode.
          event.preventDefault();
          event.stopPropagation();
          onSave();
        }
      }}
    >
      <div className="file-editor-host" ref={host} />
      {vimEnabled ? (
        <div ref={status} role="status" className="file-editor-mode" />
      ) : null}
    </div>
  );
}
