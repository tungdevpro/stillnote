import { useState } from "react";
import {
  BookOpen,
  CloudOff,
  FileText,
  Hash,
  Pencil,
  Plus,
  RefreshCw,
  Trash2,
  UserRound,
  X,
} from "lucide-react";
import type { AuthStatus, Notebook, TagCount } from "../api";
import { shortDate } from "../format";
import { useI18n } from "../i18n";

export type View =
  | { kind: "all" }
  | { kind: "notebook"; id: string }
  | { kind: "tag"; name: string }
  | { kind: "trash" };

export interface SyncState {
  busy: boolean;
  error: string | null;
}

interface SidebarProps {
  view: View;
  onView: (v: View) => void;
  notebooks: Notebook[];
  tags: TagCount[];
  onCreateNotebook: (name: string) => Promise<void>;
  onRenameNotebook: (id: string, name: string) => Promise<void>;
  onDeleteNotebook: (nb: Notebook) => void;
  auth: AuthStatus | null;
  sync: SyncState;
  onSync: () => void;
  onAccount: () => void;
}

export function Sidebar(props: SidebarProps) {
  const { view, onView, notebooks, tags } = props;
  const { t } = useI18n();
  const [adding, setAdding] = useState(false);
  const [renaming, setRenaming] = useState<string | null>(null);

  const isActive = (v: View) =>
    v.kind === view.kind &&
    (v.kind !== "notebook" || (view.kind === "notebook" && view.id === v.id)) &&
    (v.kind !== "tag" || (view.kind === "tag" && view.name === v.name));

  return (
    <aside className="sidebar">
      <div className="drag-strip" data-tauri-drag-region />

      <nav className="sidebar-scroll">
        <button className={navClass(isActive({ kind: "all" }))} onClick={() => onView({ kind: "all" })}>
          <FileText size={15} />
          <span className="nav-label">{t.allNotes}</span>
        </button>

        <div className="section-head">
          <span>{t.notebooks}</span>
          <button className="icon-btn" title={t.newNotebook} onClick={() => setAdding(true)}>
            <Plus size={14} />
          </button>
        </div>

        {adding && (
          <InlineName
            initial=""
            placeholder={t.notebookName}
            onCancel={() => setAdding(false)}
            onSubmit={async (name) => {
              await props.onCreateNotebook(name);
              setAdding(false);
            }}
          />
        )}

        {notebooks.map((nb) =>
          renaming === nb.id ? (
            <InlineName
              key={nb.id}
              initial={nb.name}
              placeholder={t.notebookName}
              onCancel={() => setRenaming(null)}
              onSubmit={async (name) => {
                await props.onRenameNotebook(nb.id, name);
                setRenaming(null);
              }}
            />
          ) : (
            <div key={nb.id} className="nav-row">
              <button
                className={navClass(isActive({ kind: "notebook", id: nb.id }))}
                onClick={() => onView({ kind: "notebook", id: nb.id })}
                onDoubleClick={() => setRenaming(nb.id)}
              >
                <BookOpen size={15} />
                <span className="nav-label">{nb.name}</span>
                <span className="nav-count">{nb.noteCount || ""}</span>
              </button>
              <span className="row-actions">
                <button className="icon-btn" title={t.rename} onClick={() => setRenaming(nb.id)}>
                  <Pencil size={12} />
                </button>
                <button className="icon-btn" title={t.deleteNotebook} onClick={() => props.onDeleteNotebook(nb)}>
                  <X size={13} />
                </button>
              </span>
            </div>
          ),
        )}
        {notebooks.length === 0 && !adding && <p className="nav-empty">{t.noNotebooks}</p>}

        {tags.length > 0 && (
          <>
            <div className="section-head">
              <span>{t.tags}</span>
            </div>
            {tags.map((t) => (
              <button
                key={t.name}
                className={navClass(isActive({ kind: "tag", name: t.name }))}
                onClick={() => onView({ kind: "tag", name: t.name })}
              >
                <Hash size={14} />
                <span className="nav-label">{t.name}</span>
                <span className="nav-count">{t.count}</span>
              </button>
            ))}
          </>
        )}

        <div className="section-gap" />
        <button className={navClass(isActive({ kind: "trash" }))} onClick={() => onView({ kind: "trash" })}>
          <Trash2 size={15} />
          <span className="nav-label">{t.trash}</span>
        </button>
      </nav>

      <AccountFooter {...props} />
    </aside>
  );
}

function AccountFooter({ auth, sync, onSync, onAccount }: SidebarProps) {
  const { t, locale } = useI18n();
  if (!auth?.email) {
    return (
      <footer className="sidebar-footer">
        <button className="account-btn" onClick={onAccount}>
          <CloudOff size={15} />
          <span className="account-text">
            <span className="account-name">{t.localOnly}</span>
            <span className="account-sub">{t.signInToSync}</span>
          </span>
        </button>
      </footer>
    );
  }
  const status = sync.busy
    ? t.syncing
    : sync.error
      ? sync.error
      : auth.lastSyncAt
        ? t.syncedAt(shortDate(auth.lastSyncAt, locale, t.yesterday))
        : t.notSynced;
  return (
    <footer className="sidebar-footer">
      <button className="account-btn" onClick={onAccount} title={sync.error ?? undefined}>
        <UserRound size={15} />
        <span className="account-text">
          <span className="account-name">{auth.email}</span>
          <span className={sync.error ? "account-sub is-error" : "account-sub"}>{status}</span>
        </span>
      </button>
      <button className="icon-btn" title={t.syncNow} onClick={onSync} disabled={sync.busy}>
        <RefreshCw size={14} className={sync.busy ? "spin" : undefined} />
      </button>
    </footer>
  );
}

function InlineName(props: {
  initial: string;
  placeholder: string;
  onSubmit: (name: string) => Promise<void>;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(props.initial);
  return (
    <form
      className="inline-name"
      onSubmit={async (e) => {
        e.preventDefault();
        if (value.trim()) await props.onSubmit(value.trim());
        else props.onCancel();
      }}
    >
      <BookOpen size={15} />
      <input
        autoFocus
        value={value}
        placeholder={props.placeholder}
        onChange={(e) => setValue(e.target.value)}
        onBlur={props.onCancel}
        onKeyDown={(e) => e.key === "Escape" && props.onCancel()}
      />
    </form>
  );
}

function navClass(active: boolean) {
  return active ? "nav-item is-active" : "nav-item";
}
