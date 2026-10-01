import { RefObject } from "react";
import { Pin, Search, SquarePen, X } from "lucide-react";
import type { NoteSummary } from "../api";
import { shortDate } from "../format";
import { useI18n } from "../i18n";

interface NoteListProps {
  title: string;
  notes: NoteSummary[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  query: string;
  onQuery: (q: string) => void;
  searchRef: RefObject<HTMLInputElement | null>;
  inTrash: boolean;
  onNewNote: () => void;
  onEmptyTrash: () => void;
}

export function NoteList(props: NoteListProps) {
  const { notes, selectedId, query, inTrash } = props;
  const { t, locale } = useI18n();

  return (
    <section className="note-list">
      <header className="list-head" data-tauri-drag-region>
        <div className="list-title-row" data-tauri-drag-region>
          <h1 className="list-title" data-tauri-drag-region>
            {props.title}
          </h1>
          {inTrash ? (
            notes.length > 0 && (
              <button className="btn btn-small" onClick={props.onEmptyTrash}>
                {t.emptyTrash}
              </button>
            )
          ) : (
            <button className="icon-btn icon-btn-lg" title={t.newNote} onClick={props.onNewNote}>
              <SquarePen size={17} />
            </button>
          )}
        </div>
        <label className="search">
          <Search size={14} />
          <input
            ref={props.searchRef}
            value={query}
            placeholder={t.search}
            spellCheck={false}
            onChange={(e) => props.onQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && props.onQuery("")}
          />
          {query && (
            <button className="icon-btn" title={t.clearSearch} onClick={() => props.onQuery("")}>
              <X size={13} />
            </button>
          )}
        </label>
        <div className="list-count">{query.trim() ? t.results(notes.length) : t.noteCount(notes.length)}</div>
      </header>

      <ul className="list-scroll" role="listbox" aria-label={t.noteList}>
        {notes.map((n) => (
          <li
            key={n.id}
            role="option"
            aria-selected={n.id === selectedId}
            className={n.id === selectedId ? "note-item is-selected" : "note-item"}
            onClick={() => props.onSelect(n.id)}
          >
            <div className="note-item-title">
              {n.pinned && !inTrash && <Pin size={12} className="pin-mark" />}
              <span className={n.title ? undefined : "is-untitled"}>{n.title || t.untitled}</span>
            </div>
            <div className="note-item-snippet">{n.snippet || t.noContent}</div>
            <div className="note-item-meta">
              <span>{shortDate(inTrash && n.trashedAt ? n.trashedAt : n.updatedAt, locale, t.yesterday)}</span>
              {n.tags.slice(0, 3).map((tag) => (
                <span key={tag} className="tag-chip tag-chip-small">
                  {tag}
                </span>
              ))}
            </div>
          </li>
        ))}
        {notes.length === 0 && <EmptyList query={query} inTrash={inTrash} onNewNote={props.onNewNote} />}
      </ul>
    </section>
  );
}

function EmptyList({ query, inTrash, onNewNote }: { query: string; inTrash: boolean; onNewNote: () => void }) {
  const { t } = useI18n();
  if (query.trim()) return <li className="list-empty">{t.noMatches(query.trim())}</li>;
  if (inTrash) return <li className="list-empty">{t.trashIsEmpty}</li>;
  return (
    <li className="list-empty">
      {t.noNotesYet}
      <button className="btn btn-primary" onClick={onNewNote}>
        {t.writeFirstNote}
      </button>
    </li>
  );
}
