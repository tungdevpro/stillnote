# Stillnote

A plain-text (Markdown) note-taking app for macOS, built with **Rust + Tauri 2** and synced through **Supabase**.

- Notes are stored locally (SQLite) and work offline
- Notebooks, tags, pinning, trash
- Full-text search that ignores diacritics ("ke hoach" finds "Kế hoạch")
- Markdown editing with preview (⌘E)
- Sign in with Supabase to sync across Macs (last write wins)
- English (default) and Vietnamese UI: switch it in the account/settings dialog (bottom-left corner)

## Keyboard shortcuts

| Key | Action |
|---|---|
| ⌘N | New note |
| ⌘F / ⌘K | Search |
| ⌘E | Toggle edit / preview |
| Enter (in the title) | Jump to the note body |

## Development

```bash
pnpm install
pnpm tauri dev
pnpm test          # Rust tests
```

## Building for macOS (Apple Silicon + Intel)

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin   # once
pnpm build:mac
```

The output goes to `src-tauri/target/universal-apple-darwin/release/bundle/`
(`macos/Stillnote.app` and `dmg/Stillnote_<version>_universal.dmg`).

To bake the Supabase settings into the build so users don't have to enter them:

```bash
STILLNOTE_SUPABASE_URL=https://xxxx.supabase.co \
STILLNOTE_SUPABASE_ANON_KEY=eyJ... \
pnpm build:mac
```

## Setting up Supabase

1. Create a project at https://supabase.com/dashboard.
2. Open the **SQL Editor**, paste the contents of `supabase/migrations/20261001000000_init.sql`, and run it
   (or use `supabase db push` if you have the Supabase CLI).
3. Under **Authentication → Providers → Email**, enable Email. With "Confirm email" turned off, users can
   sign in right after signing up. With it on, they must click the confirmation link in their email first.
4. In the app, click "Local only" in the bottom-left corner, then "Configure Supabase". Paste the
   **Project URL** and the **anon/publishable key** (Project Settings → API), then sign in or sign up.

## Architecture

```
src/                    React + TypeScript (UI)
  api.ts                calls the Rust commands via invoke()
  i18n.tsx              en/vi UI strings; Rust errors arrive as { code, detail } and are translated here
  components/           Sidebar, NoteList, Editor (CodeMirror), AccountDialog
src-tauri/src/
  db.rs                 SQLite: schema, migrations, CRUD, FTS5 search
  supabase.rs           client for Auth (GoTrue) and REST (PostgREST)
  sync.rs               pushes changed rows, pulls changes by server_seq
  commands.rs           #[tauri::command] handlers for the UI
  markdown.rs           Markdown rendering (blocks raw HTML and unsafe links)
supabase/migrations/    tables, last-write-wins trigger, Row Level Security
```

**Sync:** each row has an `updated_at` (written by the app) and a `dirty` flag (not pushed yet).
During a sync, the app upserts every dirty row to Supabase. A server-side trigger ignores writes older than
the stored row and assigns a new `server_seq`. The app then pulls the rows whose `server_seq` is newer
than its last sync. Permanently deleted notes are kept as tombstones so that deletions sync too.

Local data: `~/Library/Application Support/com.yhlabs.stillnote/stillnote.db`.

## Next steps

- Store the login session in the macOS Keychain (currently kept in the local SQLite database)
- Sign and notarize the app for distribution (Apple Developer ID)
- Auto-update (`tauri-plugin-updater`)
- Markdown file import/export, note version history
