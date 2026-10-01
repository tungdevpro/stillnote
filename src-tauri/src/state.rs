use std::sync::Mutex;

use crate::db::{now_ms, Db};
use crate::error::{AppError, AppResult};
use crate::models::Session;
use crate::supabase::{Supabase, SupabaseConfig};

pub const META_URL: &str = "supabase_url";
pub const META_KEY: &str = "supabase_anon_key";
pub const META_SESSION: &str = "session";
pub const META_OWNER: &str = "owner_user_id";
pub const META_LAST_SYNC: &str = "last_sync_at";
pub const META_CURSORS: [&str; 2] = ["pull_seq_notebooks", "pull_seq_notes"];

/// Defaults baked in at build time, e.g.
/// `STILLNOTE_SUPABASE_URL=... STILLNOTE_SUPABASE_ANON_KEY=... pnpm tauri build`.
const DEFAULT_URL: Option<&str> = option_env!("STILLNOTE_SUPABASE_URL");
const DEFAULT_KEY: Option<&str> = option_env!("STILLNOTE_SUPABASE_ANON_KEY");

pub struct AppState {
    db: Mutex<Db>,
    pub http: reqwest::Client,
    /// Only one sync runs at a time.
    pub sync_lock: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(db: Db) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        AppState { db: Mutex::new(db), http, sync_lock: tokio::sync::Mutex::new(()) }
    }

    /// Runs `f` with the database locked. Never hold the lock across an `.await`.
    pub fn with_db<T>(&self, f: impl FnOnce(&mut Db) -> AppResult<T>) -> AppResult<T> {
        let mut db = self.db.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut db)
    }

    pub fn raw_settings(&self) -> AppResult<(String, String)> {
        self.with_db(|db| {
            let url = db.get_meta(META_URL)?.unwrap_or_else(|| DEFAULT_URL.unwrap_or("").into());
            let key = db.get_meta(META_KEY)?.unwrap_or_else(|| DEFAULT_KEY.unwrap_or("").into());
            Ok((url, key))
        })
    }

    pub fn config(&self) -> AppResult<Option<SupabaseConfig>> {
        let (url, key) = self.raw_settings()?;
        Ok(SupabaseConfig::new(&url, &key))
    }

    pub fn require_config(&self) -> AppResult<SupabaseConfig> {
        self.config()?.ok_or(AppError::NotConfigured)
    }

    pub fn session(&self) -> AppResult<Option<Session>> {
        self.with_db(|db| match db.get_meta(META_SESSION)? {
            Some(json) => Ok(serde_json::from_str(&json).ok()),
            None => Ok(None),
        })
    }

    pub fn save_session(&self, session: &Session) -> AppResult<()> {
        let json = serde_json::to_string(session)?;
        self.with_db(|db| db.set_meta(META_SESSION, &json))
    }

    pub fn clear_session(&self) -> AppResult<()> {
        self.with_db(|db| db.delete_meta(META_SESSION))
    }

    /// Returns a session whose access token is valid for at least another minute.
    pub async fn fresh_session(&self, cfg: &SupabaseConfig) -> AppResult<Session> {
        let session = self.session()?.ok_or(AppError::NotSignedIn)?;
        if session.expires_at - 60 > now_ms() / 1000 {
            return Ok(session);
        }
        match Supabase::new(&self.http, cfg).refresh(&session.refresh_token).await {
            Ok(fresh) => {
                self.save_session(&fresh)?;
                Ok(fresh)
            }
            // The refresh token was rejected: the user has to sign in again.
            Err(AppError::Api { .. }) => {
                self.clear_session()?;
                Err(AppError::NotSignedIn)
            }
            Err(e) => Err(e),
        }
    }
}
