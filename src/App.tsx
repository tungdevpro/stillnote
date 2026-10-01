import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, type AuthStatus, type Note, type NoteSummary, type Notebook, type TagCount } from "./api";
import { Sidebar, type SyncState, type View } from "./components/Sidebar";
import { NoteList } from "./components/NoteList";
import { Editor } from "./components/Editor";
import { AccountDialog } from "./components/AccountDialog";
import { ConfirmDialog, type ConfirmRequest } from "./components/Dialog";
import { useI18n } from "./i18n";
import "./App.css";

const SYNC_AFTER_EDIT_MS = 3_000;
const SYNC_INTERVAL_MS = 60_000;

export default function App() {
  const [view, setView] = useState<View>({ kind: "all" });
  const [query, setQuery] = useState("");
  const [notebooks, setNotebooks] = useState<Notebook[]>([]);
  const [tags, setTags] = useState<TagCount[]>([]);
  const [notes, setNotes] = useState<NoteSummary[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [sync, setSync] = useState<SyncState>({ busy: false, error: null });
  const [reloadToken, setReloadToken] = useState(0);
  const [accountOpen, setAccountOpen] = useState(false);
  const [confirm, setConfirm] = useState<ConfirmRequest | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const { t, errorText } = useI18n();
  const errorTextRef = useRef(errorText);
  errorTextRef.current = errorText;
  const errorMessage = useCallback((e: unknown) => errorTextRef.current(e), []);

  const showError = useCallback((message: string) => setToast(message), []);
  useEffect(() => {
    if (!toast) return;
    const t = window.setTimeout(() => setToast(null), 5000);
    return () => window.clearTimeout(t);
  }, [toast]);

  const filter = useMemo(
    () => ({
      notebookId: view.kind === "notebook" ? view.id : null,
      tag: view.kind === "tag" ? view.name : null,
      trash: view.kind === "trash",
      query: query.trim() || null,
    }),
    [view, query],
  );

  // ---------- data loading ----------

  const refreshSidebar = useCallback(async () => {
    const [nb, tg] = await Promise.all([api.listNotebooks(), api.listTags()]);
    setNotebooks(nb);
    setTags(tg);
  }, []);

  const refreshNotes = useCallback(async () => {
    const list = await api.listNotes(filter);
    setNotes(list);
    setSelectedId((cur) => (cur && list.some((n) => n.id === cur) ? cur : (list[0]?.id ?? null)));
  }, [filter]);

  const refreshAll = useCallback(async () => {
    await Promise.all([refreshSidebar(), refreshNotes()]);
  }, [refreshSidebar, refreshNotes]);

  const refreshAuth = useCallback(async () => setAuth(await api.authStatus()), []);

  useEffect(() => {
    refreshNotes().catch((e) => showError(errorMessage(e)));
  }, [refreshNotes, showError, errorMessage]);

  useEffect(() => {
    refreshSidebar().catch((e) => showError(errorMessage(e)));
    refreshAuth().catch((e) => showError(errorMessage(e)));
  }, [refreshSidebar, refreshAuth, showError, errorMessage]);

  // ---------- sync ----------

  const signedIn = Boolean(auth?.email);
  const syncing = useRef(false);
  const syncTimer = useRef<number | undefined>(undefined);
  const refreshAllRef = useRef(refreshAll);
  refreshAllRef.current = refreshAll;

  const runSync = useCallback(async () => {
    if (!signedIn || syncing.current) return;
    syncing.current = true;
    setSync({ busy: true, error: null });
    try {
      const report = await api.syncNow();
      if (report.pulled > 0) {
        await refreshAllRef.current();
        setReloadToken((t) => t + 1);
      }
      setAuth((a) => (a ? { ...a, lastSyncAt: report.at } : a));
      setSync({ busy: false, error: null });
    } catch (e) {
      setSync({ busy: false, error: errorMessage(e) });
      await refreshAuth().catch(() => {});
    } finally {
      syncing.current = false;
    }
  }, [signedIn, refreshAuth, errorMessage]);

  const scheduleSync = useCallback(() => {
    window.clearTimeout(syncTimer.current);
    syncTimer.current = window.setTimeout(runSync, SYNC_AFTER_EDIT_MS);
  }, [runSync]);

  useEffect(() => {
    if (!signedIn) return;
    void runSync();
    const interval = window.setInterval(runSync, SYNC_INTERVAL_MS);
    const onFocus = () => void runSync();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, [signedIn, runSync]);

  // ---------- actions ----------

  const act = useCallback(
    async (fn: () => Promise<unknown>) => {
      try {
        await fn();
        await refreshAll();
        scheduleSync();
      } catch (e) {
        showError(errorMessage(e));
      }
    },
    [refreshAll, scheduleSync, showError, errorMessage],
  );

  const newNote = useCallback(async () => {
    try {
      const notebookId = view.kind === "notebook" ? view.id : null;
      const note = await api.createNote(notebookId);
      if (view.kind === "tag") await api.setNoteTags(note.id, [view.name]);
      if (view.kind === "trash") setView({ kind: "all" });
      setQuery("");
      setSelectedId(note.id);
      await refreshAll();
      scheduleSync();
    } catch (e) {
      showError(errorMessage(e));
    }
  }, [view, refreshAll, scheduleSync, showError, errorMessage]);

  const onNoteChanged = useCallback(
    (_note: Note) => {
      refreshAll().catch(() => {});
      scheduleSync();
    },
    [refreshAll, scheduleSync],
  );

  const deleteNotebook = (nb: Notebook) =>
    setConfirm({
      title: t.deleteNotebookTitle(nb.name),
      message: t.deleteNotebookBody(nb.noteCount),
      confirmLabel: t.deleteNotebook,
      danger: true,
      onConfirm: () =>
        act(async () => {
          await api.deleteNotebook(nb.id);
          if (view.kind === "notebook" && view.id === nb.id) setView({ kind: "all" });
        }),
    });

  const signOutAndWipe = () =>
    setConfirm({
      title: t.wipeTitle,
      message: t.wipeBody,
      confirmLabel: t.wipeConfirm,
      danger: true,
      onConfirm: async () => {
        try {
          await api.syncNow();
          await api.signOut(true);
        } catch (e) {
          showError(t.wipeFailed(errorMessage(e)));
          return;
        }
        setSelectedId(null);
        await Promise.all([refreshAuth(), refreshAll()]);
      },
    });

  const signOutKeep = async () => {
    await api.signOut(false).catch((e) => showError(errorMessage(e)));
    await refreshAuth();
  };

  // ---------- keyboard ----------

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      const k = e.key.toLowerCase();
      if (k === "n") {
        e.preventDefault();
        void newNote();
      } else if (k === "f" || k === "k") {
        e.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [newNote]);

  const listTitle =
    view.kind === "all"
      ? t.allNotes
      : view.kind === "trash"
        ? t.trash
        : view.kind === "tag"
          ? `#${view.name}`
          : (notebooks.find((n) => n.id === view.id)?.name ?? t.notebookFallback);

  return (
    <div className="app">
      <Sidebar
        view={view}
        onView={(v) => {
          setView(v);
          setQuery("");
        }}
        notebooks={notebooks}
        tags={tags}
        onCreateNotebook={(name) =>
          act(async () => {
            const nb = await api.createNotebook(name);
            setView({ kind: "notebook", id: nb.id });
          })
        }
        onRenameNotebook={(id, name) => act(() => api.renameNotebook(id, name))}
        onDeleteNotebook={deleteNotebook}
        auth={auth}
        sync={sync}
        onSync={runSync}
        onAccount={() => setAccountOpen(true)}
      />

      <NoteList
        title={listTitle}
        notes={notes}
        selectedId={selectedId}
        onSelect={setSelectedId}
        query={query}
        onQuery={setQuery}
        searchRef={searchRef}
        inTrash={view.kind === "trash"}
        onNewNote={newNote}
        onEmptyTrash={() =>
          setConfirm({
            title: t.emptyTrashTitle,
            message: t.emptyTrashBody(notes.length),
            confirmLabel: t.deleteForever,
            danger: true,
            onConfirm: () => act(() => api.emptyTrash()),
          })
        }
      />

      {selectedId ? (
        <Editor
          key={selectedId}
          noteId={selectedId}
          notebooks={notebooks}
          reloadToken={reloadToken}
          onChanged={onNoteChanged}
          onTrash={(id) => act(() => api.trashNote(id))}
          onRestore={(id) => act(() => api.restoreNote(id))}
          onDeleteForever={(id) =>
            setConfirm({
              title: t.deleteNoteTitle,
              message: t.deleteNoteBody,
              confirmLabel: t.deleteForever,
              danger: true,
              onConfirm: () => act(() => api.deleteNoteForever(id)),
            })
          }
          onError={showError}
        />
      ) : (
        <main className="editor editor-empty" data-tauri-drag-region>
          <p>{t.emptyEditor}</p>
        </main>
      )}

      {accountOpen && auth && (
        <AccountDialog
          auth={auth}
          onClose={() => setAccountOpen(false)}
          onAuthChanged={refreshAuth}
          onSignOut={() => {
            setAccountOpen(false);
            setConfirm({
              title: t.signOutTitle,
              message: t.signOutBody,
              confirmLabel: t.signOutKeep,
              onConfirm: signOutKeep,
              secondary: { label: t.removeFromMac, onClick: signOutAndWipe },
            });
          }}
        />
      )}
      {confirm && <ConfirmDialog req={confirm} onClose={() => setConfirm(null)} />}
      {toast && (
        <div className="toast" role="alert" onClick={() => setToast(null)}>
          {toast}
        </div>
      )}
    </div>
  );
}
