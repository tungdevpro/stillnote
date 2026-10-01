use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Errors reach the frontend as `{ code, detail }`; the UI translates `code`
/// into the user's language (see `src/i18n.tsx`).
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("data error: {0}")]
    Json(#[from] serde_json::Error),
    /// Reported by Supabase. `code` is e.g. "invalidCredentials" or "server".
    #[error("{code}: {detail}")]
    Api { code: &'static str, detail: String },
    #[error("Supabase is not configured")]
    NotConfigured,
    #[error("not signed in")]
    NotSignedIn,
    /// What was not found: "note" or "notebook".
    #[error("{0} not found")]
    NotFound(&'static str),
    /// Which input was rejected, e.g. "blankNotebookName".
    #[error("invalid input: {0}")]
    Invalid(&'static str),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Db(_) => "database",
            AppError::Http(_) => "network",
            AppError::Json(_) => "data",
            AppError::Api { code, .. } => code,
            AppError::NotConfigured => "notConfigured",
            AppError::NotSignedIn => "notSignedIn",
            AppError::NotFound(_) => "notFound",
            AppError::Invalid(_) => "invalid",
        }
    }

    pub fn detail(&self) -> String {
        match self {
            AppError::Db(e) => e.to_string(),
            AppError::Http(e) => e.to_string(),
            AppError::Json(e) => e.to_string(),
            AppError::Api { detail, .. } => detail.clone(),
            AppError::NotConfigured | AppError::NotSignedIn => String::new(),
            AppError::NotFound(what) | AppError::Invalid(what) => what.to_string(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("detail", &self.detail())?;
        s.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_as_code_and_detail() {
        let json = serde_json::to_value(AppError::NotFound("note")).unwrap();
        assert_eq!(json, serde_json::json!({ "code": "notFound", "detail": "note" }));
        let json = serde_json::to_value(AppError::NotSignedIn).unwrap();
        assert_eq!(json["code"], "notSignedIn");
    }
}
