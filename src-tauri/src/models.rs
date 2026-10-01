use serde::{Deserialize, Serialize};

/// A note as shown in the editor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub notebook_id: Option<String>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub trashed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A lightweight row for the note list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub notebook_id: Option<String>,
    pub title: String,
    pub snippet: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub trashed_at: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notebook {
    pub id: String,
    pub name: String,
    pub note_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteFilter {
    pub notebook_id: Option<String>,
    pub tag: Option<String>,
    #[serde(default)]
    pub trash: bool,
    pub query: Option<String>,
}

/// Row shape of the `notes` table in Supabase (snake_case, like Postgres).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteNote {
    pub id: String,
    pub notebook_id: Option<String>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub trashed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted: bool,
    #[serde(default, skip_serializing)]
    pub server_seq: i64,
}

/// Row shape of the `notebooks` table in Supabase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteNotebook {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted: bool,
    #[serde(default, skip_serializing)]
    pub server_seq: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds.
    pub expires_at: i64,
    pub user_id: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub supabase_url: String,
    pub supabase_anon_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub configured: bool,
    pub email: Option<String>,
    pub last_sync_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub pushed: usize,
    pub pulled: usize,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SignUpOutcome {
    SignedIn { email: String },
    ConfirmEmail { email: String },
}
