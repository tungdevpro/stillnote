import { useCallback, useEffect, useRef, useState, type MouseEvent } from "react";
import CodeMirror, { ReactCodeMirrorRef } from "@uiw/react-codemirror";
import { markdown } from "@codemirror/lang-markdown";
import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";
import { openUrl } from "@tauri-apps/plugin-opener";
import { BookOpen, Eye, PencilLine, Pin, RotateCcw, Trash2 } from "lucide-react";
import { api, type Note, type Notebook } from "../api";
import { longDate } from "../format";
import { useI18n } from "../i18n";
import { TagEditor } from "./TagEditor";

const SAVE_DELAY_MS = 500;

const highlight = HighlightStyle.define([
  { tag: t.heading1, fontSize: "1.35em", fontWeight: "650" },
  { tag: t.heading2, fontSize: "1.2em", fontWeight: "650" },
  { tag: [t.heading3, t.heading4, t.heading5, t.heading6], fontWeight: "650" },
  { tag: t.strong, fontWeight: "700" },
  { tag: t.emphasis, fontStyle: "italic" },
  { tag: t.strikethrough, textDecoration: "line-through" },
  { tag: [t.link, t.url], color: "var(--accent)" },
  { tag: t.monospace, fontFamily: "var(--font-mono)", fontSize: "0.92em" },
  { tag: [t.processingInstruction, t.contentSeparator, t.meta], color: "var(--muted)" },
  { tag: t.quote, color: "var(--text-soft)", fontStyle: "italic" },
]);

const extensions = [
  markdown(),
  EditorView.lineWrapping,
  syntaxHighlighting(highlight),
  EditorView.theme({
    "&": { backgroundColor: "transparent", color: "var(--text)", fontSize: "15px" },
    "&.cm-focused": { outline: "none" },
    // Grow with the content; the page around it does the scrolling.
    ".cm-scroller": { fontFamily: "var(--font-ui)", lineHeight: "1.7", overflow: "visible" },
    ".cm-content": { padding: "0 0 40vh", caretColor: "var(--accent)" },
    ".cm-line": { padding: "0" },
    ".cm-cursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "var(--selection) !important",
    },
    ".cm-placeholder": { color: "var(--muted)" },
  }),
];

const basicSetup = {
  lineNumbers: false,
  foldGutter: false,
  highlightActiveLine: false,
  highlightActiveLineGutter: false,
  highlightSelectionMatches: false,
  syntaxHighlighting: false,
  autocompletion: false,
  searchKeymap: true,
};

type Mode = "edit" | "preview";

function initialMode(): Mode {
  try {
    return localStorage.getItem("editor-mode") === "preview" ? "preview" : "edit";
  } catch {
    return "edit";
  }
}

interface EditorProps {
  noteId: string;
  notebooks: Notebook[];
  /** Bumped when a sync pulled changes, so the open note can refresh. */
  reloadToken: number;
  onChanged: (note: Note) => void;
  onTrash: (id: string) => void;
  onRestore: (id: string) => void;
  onDeleteForever: (id: string) => void;
  onError: (message: string) => void;
}

export function Editor(props: EditorProps) {
  const { noteId } = props;
  const [note, setNote] = useState<Note | null>(null);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [mode, setMode] = useState<Mode>(initialMode);
  const [html, setHtml] = useState("");
  const cmRef = useRef<ReactCodeMirrorRef>(null);
  const { t, locale, errorText } = useI18n();
  const errorTextRef = useRef(errorText);
  errorTextRef.current = errorText;

  // Latest callbacks, so the debounced save never calls a stale closure.
  const cb = useRef(props);
  cb.current = props;

  const pending = useRef<{ id: string; title: string; body: string } | null>(null);
  const timer = useRef<number | undefined>(undefined);

  const flush = useCallback(async () => {
    window.clearTimeout(timer.current);
    const p = pending.current;
    if (!p) return;
    pending.current = null;
    try {
      const saved = await api.updateNote(p.id, p.title, p.body);
      setNote((cur) => (cur?.id === saved.id ? saved : cur));
      cb.current.onChanged(saved);
    } catch (e) {
      cb.current.onError(errorTextRef.current(e));
    }
  }, []);

  const queueSave = (nextTitle: string, nextBody: string) => {
    pending.current = { id: noteId, title: nextTitle, body: nextBody };
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(flush, SAVE_DELAY_MS);
  };

  // Load the note; save any pending edits when switching away.
  useEffect(() => {
    let alive = true;
    setNote(null);
    api
      .getNote(noteId)
      .then((n) => {
        if (!alive) return;
        setNote(n);
        setTitle(n.title);
        setBody(n.body);
      })
      .catch((e) => cb.current.onError(errorTextRef.current(e)));
    return () => {
      alive = false;
      void flush();
    };
  }, [noteId, flush]);

  // Pick up changes from sync unless the user is mid-edit.
  useEffect(() => {
    if (props.reloadToken === 0) return;
    if (pending.current) return;
    api
      .getNote(noteId)
      .then((n) => {
        if (pending.current) return;
        setNote(n);
        setTitle(n.title);
        setBody(n.body);
      })
      .catch(() => {});
  }, [props.reloadToken, noteId]);

  // Flush before the window closes.
  useEffect(() => {
    const onHide = () => void flush();
    window.addEventListener("beforeunload", onHide);
    document.addEventListener("visibilitychange", onHide);
    return () => {
      window.removeEventListener("beforeunload", onHide);
      document.removeEventListener("visibilitychange", onHide);
    };
  }, [flush]);

  useEffect(() => {
    if (mode !== "preview") return;
    let alive = true;
    api.renderMarkdown(body).then((h) => alive && setHtml(h));
    return () => {
      alive = false;
    };
  }, [mode, body]);

  useEffect(() => {
    try {
      localStorage.setItem("editor-mode", mode);
    } catch {
      /* storage unavailable */
    }
  }, [mode]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.key.toLowerCase() === "e") {
        e.preventDefault();
        setMode((m) => (m === "edit" ? "preview" : "edit"));
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!note) return <div className="editor" />;

  const inTrash = note.trashedAt !== null;
  const update = async (fn: () => Promise<Note>) => {
    try {
      const n = await fn();
      setNote(n);
      props.onChanged(n);
    } catch (e) {
      props.onError(errorText(e));
    }
  };

  return (
    <main className="editor">
      <div className="editor-bar" data-tauri-drag-region>
        <label className="notebook-picker" title={t.notebook}>
          <BookOpen size={14} />
          <select
            value={note.notebookId ?? ""}
            disabled={inTrash}
            onChange={(e) => update(() => api.moveNote(note.id, e.target.value || null))}
          >
            <option value="">{t.noNotebook}</option>
            {props.notebooks.map((nb) => (
              <option key={nb.id} value={nb.id}>
                {nb.name}
              </option>
            ))}
          </select>
        </label>
        <div className="editor-bar-spacer" data-tauri-drag-region />
        {!inTrash && (
          <>
            <button
              className={mode === "preview" ? "icon-btn is-on" : "icon-btn"}
              title={mode === "edit" ? t.showPreview : t.showEditor}
              onClick={() => setMode(mode === "edit" ? "preview" : "edit")}
            >
              {mode === "edit" ? <Eye size={16} /> : <PencilLine size={16} />}
            </button>
            <button
              className={note.pinned ? "icon-btn is-on" : "icon-btn"}
              title={note.pinned ? t.unpin : t.pin}
              onClick={() => update(() => api.setNotePinned(note.id, !note.pinned))}
            >
              <Pin size={16} />
            </button>
            <button
              className="icon-btn"
              title={t.moveToTrash}
              onClick={async () => {
                await flush();
                props.onTrash(note.id);
              }}
            >
              <Trash2 size={16} />
            </button>
          </>
        )}
      </div>

      {inTrash && (
        <div className="trash-banner">
          <span>{t.inTrashBanner}</span>
          <button className="btn btn-small" onClick={() => props.onRestore(note.id)}>
            <RotateCcw size={13} /> {t.restore}
          </button>
          <button className="btn btn-small btn-danger" onClick={() => props.onDeleteForever(note.id)}>
            {t.deleteForever}
          </button>
        </div>
      )}

      <div className="editor-scroll">
        <div className="editor-page">
          <input
            className="title-input"
            value={title}
            placeholder={t.titlePlaceholder}
            readOnly={inTrash}
            onChange={(e) => {
              setTitle(e.target.value);
              queueSave(e.target.value, body);
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === "ArrowDown") {
                e.preventDefault();
                const view = cmRef.current?.view;
                if (view) view.focus();
                else {
                  setMode("edit");
                  requestAnimationFrame(() => cmRef.current?.view?.focus());
                }
              }
            }}
            autoFocus={!note.title && !note.body && !inTrash}
          />
          <div className="note-meta">
            <span title={t.created(longDate(note.createdAt, locale))}>{t.edited(longDate(note.updatedAt, locale))}</span>
          </div>
          <TagEditor
            tags={note.tags}
            readOnly={inTrash}
            onChange={(tags) => update(() => api.setNoteTags(note.id, tags))}
          />

          {mode === "edit" && !inTrash ? (
            <CodeMirror
              ref={cmRef}
              className="body-editor"
              value={body}
              theme="none"
              basicSetup={basicSetup}
              extensions={extensions}
              placeholder={t.bodyPlaceholder}
              onChange={(v) => {
                setBody(v);
                queueSave(title, v);
              }}
            />
          ) : (
            <Preview html={inTrash ? null : html} source={body} emptyText={t.previewEmpty} />
          )}
        </div>
      </div>
    </main>
  );
}

function Preview({ html, source, emptyText }: { html: string | null; source: string; emptyText: string }) {
  // Links open in the default browser instead of navigating the app window.
  const onClick = (e: MouseEvent) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (/^(https?:|mailto:)/i.test(href)) void openUrl(href);
  };
  if (html === null) return <pre className="preview-plain">{source}</pre>;
  if (!source.trim()) return <p className="preview-empty">{emptyText}</p>;
  // `html` comes from the Rust renderer, which escapes raw HTML and unsafe links.
  return <article className="preview" onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />;
}
