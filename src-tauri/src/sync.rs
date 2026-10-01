//! Two-way sync with Supabase.
//!
//! Push: every locally changed (dirty) row is upserted; the server ignores
//! writes older than what it has (see `supabase/migrations`).
//! Pull: rows are fetched by `server_seq`, a server-assigned millisecond
//! timestamp, starting a few seconds before the last cursor so that rows
//! committed slightly out of order are not missed. Applying a row twice is
//! harmless because the local merge is last-write-wins on `updated_at`.

use serde::{de::DeserializeOwned, Serialize};

use crate::db::{now_ms, Db, SyncTable};
use crate::error::AppResult;
use crate::models::{RemoteNote, RemoteNotebook, SyncReport};
use crate::state::{AppState, META_CURSORS, META_LAST_SYNC};
use crate::supabase::Supabase;

const PAGE: usize = 500;
const OVERLAP_MS: i64 = 5_000;

pub async fn sync(state: &AppState) -> AppResult<SyncReport> {
    let _guard = state.sync_lock.lock().await;
    let cfg = state.require_config()?;
    let session = state.fresh_session(&cfg).await?;
    let sb = Supabase::new(&state.http, &cfg);
    let token = session.access_token.as_str();

    // Notebooks first so pulled notes on other devices find their notebook.
    let notebooks = state.with_db(|db| db.dirty_notebooks())?;
    let notes = state.with_db(|db| db.dirty_notes())?;
    let mut pushed = push(state, &sb, token, SyncTable::Notebooks, &notebooks, |b| (b.id.clone(), b.updated_at)).await?;
    pushed += push(state, &sb, token, SyncTable::Notes, &notes, |n| (n.id.clone(), n.updated_at)).await?;

    let mut pulled = pull::<RemoteNotebook>(state, &sb, token, SyncTable::Notebooks, META_CURSORS[0], |b| b.server_seq, Db::apply_remote_notebooks).await?;
    pulled += pull::<RemoteNote>(state, &sb, token, SyncTable::Notes, META_CURSORS[1], |n| n.server_seq, Db::apply_remote_notes).await?;

    let at = now_ms();
    state.with_db(|db| db.set_meta(META_LAST_SYNC, &at.to_string()))?;
    Ok(SyncReport { pushed, pulled, at })
}

async fn push<T: Serialize>(
    state: &AppState,
    sb: &Supabase<'_>,
    token: &str,
    table: SyncTable,
    rows: &[T],
    key: impl Fn(&T) -> (String, i64),
) -> AppResult<usize> {
    for chunk in rows.chunks(PAGE) {
        sb.upsert(table.name(), chunk, token).await?;
        let keys: Vec<_> = chunk.iter().map(&key).collect();
        state.with_db(|db| db.mark_clean(table, &keys))?;
    }
    Ok(rows.len())
}

async fn pull<T: DeserializeOwned>(
    state: &AppState,
    sb: &Supabase<'_>,
    token: &str,
    table: SyncTable,
    cursor_key: &str,
    seq: impl Fn(&T) -> i64,
    apply: impl Fn(&mut Db, &[T]) -> AppResult<usize>,
) -> AppResult<usize> {
    let saved: i64 = state
        .with_db(|db| db.get_meta(cursor_key))?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut after = (saved - OVERLAP_MS).max(0);
    let mut newest = saved;
    let mut applied = 0;
    loop {
        let rows: Vec<T> = sb.changes_since(table.name(), after, PAGE, token).await?;
        let Some(last) = rows.last() else { break };
        after = seq(last);
        newest = newest.max(after);
        applied += state.with_db(|db| apply(db, &rows))?;
        state.with_db(|db| db.set_meta(cursor_key, &newest.to_string()))?;
        if rows.len() < PAGE {
            break;
        }
    }
    Ok(applied)
}
