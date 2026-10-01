import { invoke } from "@tauri-apps/api/core";

export interface Note {
  id: string;
  notebookId: string | null;
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  trashedAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface NoteSummary {
  id: string;
  notebookId: string | null;
  title: string;
  snippet: string;
  tags: string[];
  pinned: boolean;
  trashedAt: number | null;
  updatedAt: number;
}

export interface Notebook {
  id: string;
  name: string;
  noteCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface TagCount {
  name: string;
  count: number;
}

export interface NoteFilter {
  notebookId?: string | null;
  tag?: string | null;
  trash?: boolean;
  query?: string | null;
}

export interface Settings {
  supabaseUrl: string;
  supabaseAnonKey: string;
}

export interface AuthStatus {
  configured: boolean;
  email: string | null;
  lastSyncAt: number | null;
}

export interface SyncReport {
  pushed: number;
  pulled: number;
  at: number;
}

export type SignUpOutcome =
  | { kind: "signedIn"; email: string }
  | { kind: "confirmEmail"; email: string };

export const api = {
  listNotebooks: () => invoke<Notebook[]>("list_notebooks"),
  createNotebook: (name: string) => invoke<Notebook>("create_notebook", { name }),
  renameNotebook: (id: string, name: string) => invoke<void>("rename_notebook", { id, name }),
  deleteNotebook: (id: string) => invoke<void>("delete_notebook", { id }),
  listTags: () => invoke<TagCount[]>("list_tags"),

  listNotes: (filter: NoteFilter) => invoke<NoteSummary[]>("list_notes", { filter }),
  getNote: (id: string) => invoke<Note>("get_note", { id }),
  createNote: (notebookId: string | null) => invoke<Note>("create_note", { notebookId }),
  updateNote: (id: string, title: string, body: string) =>
    invoke<Note>("update_note", { id, title, body }),
  moveNote: (id: string, notebookId: string | null) => invoke<Note>("move_note", { id, notebookId }),
  setNoteTags: (id: string, tags: string[]) => invoke<Note>("set_note_tags", { id, tags }),
  setNotePinned: (id: string, pinned: boolean) => invoke<Note>("set_note_pinned", { id, pinned }),
  trashNote: (id: string) => invoke<Note>("trash_note", { id }),
  restoreNote: (id: string) => invoke<Note>("restore_note", { id }),
  deleteNoteForever: (id: string) => invoke<void>("delete_note_forever", { id }),
  emptyTrash: () => invoke<number>("empty_trash"),
  renderMarkdown: (source: string) => invoke<string>("render_markdown", { source }),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (s: Settings) => invoke<void>("save_settings", { ...s }),
  authStatus: () => invoke<AuthStatus>("auth_status"),
  signUp: (email: string, password: string) => invoke<SignUpOutcome>("sign_up", { email, password }),
  signIn: (email: string, password: string) => invoke<string>("sign_in", { email, password }),
  signOut: (wipeLocal: boolean) => invoke<void>("sign_out", { wipeLocal }),
  syncNow: () => invoke<SyncReport>("sync_now"),
};
