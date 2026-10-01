//! Commands callable from the frontend via `invoke(...)`.

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::state::*;
use crate::supabase::{SignUp, Supabase, SupabaseConfig};

type S<'a> = State<'a, AppState>;

// ---------- notebooks & tags ----------

#[tauri::command]
pub fn list_notebooks(state: S) -> AppResult<Vec<Notebook>> {
    state.with_db(|db| db.list_notebooks())
}

#[tauri::command]
pub fn create_notebook(state: S, name: String) -> AppResult<Notebook> {
    state.with_db(|db| db.create_notebook(&name))
}

#[tauri::command]
pub fn rename_notebook(state: S, id: String, name: String) -> AppResult<()> {
    state.with_db(|db| db.rename_notebook(&id, &name))
}

#[tauri::command]
pub fn delete_notebook(state: S, id: String) -> AppResult<()> {
    state.with_db(|db| db.delete_notebook(&id))
}

#[tauri::command]
pub fn list_tags(state: S) -> AppResult<Vec<TagCount>> {
    state.with_db(|db| db.list_tags())
}

// ---------- notes ----------

#[tauri::command]
pub fn list_notes(state: S, filter: NoteFilter) -> AppResult<Vec<NoteSummary>> {
    state.with_db(|db| db.list_notes(&filter))
}

#[tauri::command]
pub fn get_note(state: S, id: String) -> AppResult<Note> {
    state.with_db(|db| db.get_note(&id))
}

#[tauri::command]
pub fn create_note(state: S, notebook_id: Option<String>) -> AppResult<Note> {
    state.with_db(|db| db.create_note(notebook_id.as_deref()))
}

#[tauri::command]
pub fn update_note(state: S, id: String, title: String, body: String) -> AppResult<Note> {
    state.with_db(|db| db.update_note_content(&id, &title, &body))
}

#[tauri::command]
pub fn move_note(state: S, id: String, notebook_id: Option<String>) -> AppResult<Note> {
    state.with_db(|db| db.set_note_notebook(&id, notebook_id.as_deref()))
}

#[tauri::command]
pub fn set_note_tags(state: S, id: String, tags: Vec<String>) -> AppResult<Note> {
    state.with_db(|db| db.set_note_tags(&id, &tags))
}

#[tauri::command]
pub fn set_note_pinned(state: S, id: String, pinned: bool) -> AppResult<Note> {
    state.with_db(|db| db.set_note_pinned(&id, pinned))
}

#[tauri::command]
pub fn trash_note(state: S, id: String) -> AppResult<Note> {
    state.with_db(|db| db.trash_note(&id))
}

#[tauri::command]
pub fn restore_note(state: S, id: String) -> AppResult<Note> {
    state.with_db(|db| db.restore_note(&id))
}

#[tauri::command]
pub fn delete_note_forever(state: S, id: String) -> AppResult<()> {
    state.with_db(|db| db.delete_note_forever(&id))
}

#[tauri::command]
pub fn empty_trash(state: S) -> AppResult<usize> {
    state.with_db(|db| db.empty_trash())
}

#[tauri::command]
pub fn render_markdown(source: String) -> String {
    crate::markdown::render(&source)
}

// ---------- settings & account ----------

#[tauri::command]
pub fn get_settings(state: S) -> AppResult<Settings> {
    let (supabase_url, supabase_anon_key) = state.raw_settings()?;
    Ok(Settings { supabase_url, supabase_anon_key })
}

/// Saving a different project signs the user out; local notes are kept and
/// uploaded to whichever account signs in next.
#[tauri::command]
pub fn save_settings(state: S, supabase_url: String, supabase_anon_key: String) -> AppResult<()> {
    let url = supabase_url.trim().trim_end_matches('/').to_string();
    if !url.is_empty() && !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(AppError::Invalid("badUrl"));
    }
    let (old_url, old_key) = state.raw_settings()?;
    if old_url == url && old_key == supabase_anon_key.trim() {
        return Ok(());
    }
    state.with_db(|db| {
        db.set_meta(META_URL, &url)?;
        db.set_meta(META_KEY, supabase_anon_key.trim())?;
        db.delete_meta(META_SESSION)?;
        db.delete_meta(META_OWNER)?;
        for k in META_CURSORS {
            db.delete_meta(k)?;
        }
        Ok(())
    })
}

#[tauri::command]
pub fn auth_status(state: S) -> AppResult<AuthStatus> {
    let configured = state.config()?.is_some();
    let email = state.session()?.map(|s| s.email);
    let last_sync_at = state
        .with_db(|db| db.get_meta(META_LAST_SYNC))?
        .and_then(|v| v.parse().ok());
    Ok(AuthStatus { configured, email, last_sync_at })
}

#[tauri::command]
pub async fn sign_up(state: S<'_>, email: String, password: String) -> AppResult<SignUpOutcome> {
    let cfg = state.require_config()?;
    let email = email.trim().to_string();
    match Supabase::new(&state.http, &cfg).sign_up(&email, &password).await? {
        SignUp::Session(session) => {
            start_session(&state, &session)?;
            Ok(SignUpOutcome::SignedIn { email: session.email })
        }
        SignUp::NeedsConfirmation => Ok(SignUpOutcome::ConfirmEmail { email }),
    }
}

#[tauri::command]
pub async fn sign_in(state: S<'_>, email: String, password: String) -> AppResult<String> {
    let cfg: SupabaseConfig = state.require_config()?;
    let session = Supabase::new(&state.http, &cfg).sign_in(email.trim(), &password).await?;
    start_session(&state, &session)?;
    Ok(session.email)
}

/// Signing in as a different account than last time re-uploads every local
/// note to that account and pulls its notes from scratch.
fn start_session(state: &AppState, session: &crate::models::Session) -> AppResult<()> {
    state.with_db(|db| {
        if db.get_meta(META_OWNER)?.as_deref() != Some(session.user_id.as_str()) {
            db.mark_all_dirty()?;
            for k in META_CURSORS {
                db.delete_meta(k)?;
            }
            db.delete_meta(META_LAST_SYNC)?;
            db.set_meta(META_OWNER, &session.user_id)?;
        }
        Ok(())
    })?;
    state.save_session(session)
}

#[tauri::command]
pub async fn sign_out(state: S<'_>, wipe_local: bool) -> AppResult<()> {
    if let (Some(cfg), Some(session)) = (state.config()?, state.session()?) {
        // Best effort: the local session is cleared even when offline.
        let _ = Supabase::new(&state.http, &cfg).sign_out(&session.access_token).await;
    }
    state.clear_session()?;
    if wipe_local {
        state.with_db(|db| {
            db.wipe_local_data()?;
            db.delete_meta(META_OWNER)?;
            db.delete_meta(META_LAST_SYNC)?;
            for k in META_CURSORS {
                db.delete_meta(k)?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn sync_now(state: S<'_>) -> AppResult<SyncReport> {
    crate::sync::sync(&state).await
}
