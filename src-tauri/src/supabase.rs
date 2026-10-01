//! Minimal Supabase client: GoTrue (auth) and PostgREST (tables) over HTTPS.

use reqwest::{RequestBuilder, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::Session;

#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub url: String,
    pub anon_key: String,
}

impl SupabaseConfig {
    pub fn new(url: &str, anon_key: &str) -> Option<Self> {
        let url = url.trim().trim_end_matches('/');
        let anon_key = anon_key.trim();
        (!url.is_empty() && !anon_key.is_empty())
            .then(|| SupabaseConfig { url: url.to_string(), anon_key: anon_key.to_string() })
    }
}

pub enum SignUp {
    Session(Session),
    NeedsConfirmation,
}

pub struct Supabase<'a> {
    http: &'a reqwest::Client,
    cfg: &'a SupabaseConfig,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: Option<i64>,
    expires_in: Option<i64>,
    user: TokenUser,
}

#[derive(Deserialize)]
struct TokenUser {
    id: String,
    email: Option<String>,
}

impl TokenResponse {
    fn into_session(self) -> Session {
        let expires_at = self.expires_at.unwrap_or_else(|| {
            crate::db::now_ms() / 1000 + self.expires_in.unwrap_or(3600)
        });
        Session {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at,
            user_id: self.user.id,
            email: self.user.email.unwrap_or_default(),
        }
    }
}

impl<'a> Supabase<'a> {
    pub fn new(http: &'a reqwest::Client, cfg: &'a SupabaseConfig) -> Self {
        Supabase { http, cfg }
    }

    fn auth_url(&self, path: &str) -> String {
        format!("{}/auth/v1/{path}", self.cfg.url)
    }

    fn rest_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{table}", self.cfg.url)
    }

    fn with_key(&self, req: RequestBuilder) -> RequestBuilder {
        req.header("apikey", &self.cfg.anon_key)
    }

    pub async fn sign_up(&self, email: &str, password: &str) -> AppResult<SignUp> {
        let res = self
            .with_key(self.http.post(self.auth_url("signup")))
            .json(&json!({ "email": email, "password": password }))
            .send()
            .await?;
        let body: Value = check(res).await?.json().await?;
        // With "Confirm email" enabled, GoTrue returns only the user object.
        if body.get("access_token").is_some() {
            Ok(SignUp::Session(serde_json::from_value::<TokenResponse>(body)?.into_session()))
        } else {
            Ok(SignUp::NeedsConfirmation)
        }
    }

    pub async fn sign_in(&self, email: &str, password: &str) -> AppResult<Session> {
        self.token("password", json!({ "email": email, "password": password })).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> AppResult<Session> {
        self.token("refresh_token", json!({ "refresh_token": refresh_token })).await
    }

    async fn token(&self, grant_type: &str, body: Value) -> AppResult<Session> {
        let res = self
            .with_key(self.http.post(self.auth_url("token")))
            .query(&[("grant_type", grant_type)])
            .json(&body)
            .send()
            .await?;
        Ok(check(res).await?.json::<TokenResponse>().await?.into_session())
    }

    pub async fn sign_out(&self, access_token: &str) -> AppResult<()> {
        let res = self
            .with_key(self.http.post(self.auth_url("logout")))
            .bearer_auth(access_token)
            .send()
            .await?;
        check(res).await?;
        Ok(())
    }

    /// Inserts or updates rows by primary key.
    pub async fn upsert<T: Serialize>(&self, table: &str, rows: &[T], token: &str) -> AppResult<()> {
        let res = self
            .with_key(self.http.post(self.rest_url(table)))
            .bearer_auth(token)
            .query(&[("on_conflict", "id")])
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .json(rows)
            .send()
            .await?;
        check(res).await?;
        Ok(())
    }

    /// Fetches rows with `server_seq > after`, oldest first.
    pub async fn changes_since<T: DeserializeOwned>(
        &self,
        table: &str,
        after: i64,
        limit: usize,
        token: &str,
    ) -> AppResult<Vec<T>> {
        let res = self
            .with_key(self.http.get(self.rest_url(table)))
            .bearer_auth(token)
            .query(&[
                ("select", "*".to_string()),
                ("server_seq", format!("gt.{after}")),
                ("order", "server_seq.asc".to_string()),
                ("limit", limit.to_string()),
            ])
            .send()
            .await?;
        Ok(check(res).await?.json().await?)
    }
}

/// Turns a non-2xx response into an error the UI can translate.
async fn check(res: Response) -> AppResult<Response> {
    let status = res.status();
    if status.is_success() {
        return Ok(res);
    }
    let text = res.text().await.unwrap_or_default();
    Err(api_error(status.as_u16(), &text))
}

fn api_error(status: u16, body: &str) -> AppError {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let field = |k: &str| parsed.as_ref().and_then(|v| v.get(k)).and_then(Value::as_str);
    let detail = field("msg")
        .or_else(|| field("error_description"))
        .or_else(|| field("message"))
        .unwrap_or(body)
        .to_string();
    let code = match field("error_code").or_else(|| field("error")) {
        Some("invalid_credentials") | Some("invalid_grant") => "invalidCredentials",
        Some("email_not_confirmed") => "emailNotConfirmed",
        Some("user_already_exists") => "userExists",
        Some("weak_password") => "weakPassword",
        _ if status == 401 => "sessionInvalid",
        _ => "server",
    };
    let detail = if code == "server" { format!("{status}: {detail}") } else { detail };
    AppError::Api { code, detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_both_values() {
        assert!(SupabaseConfig::new("", "key").is_none());
        assert!(SupabaseConfig::new("https://x.supabase.co", " ").is_none());
        let cfg = SupabaseConfig::new(" https://x.supabase.co/ ", "key").unwrap();
        assert_eq!(cfg.url, "https://x.supabase.co");
    }

    #[test]
    fn errors_map_to_codes() {
        let code = |status, body| api_error(status, body).code();
        let body = r#"{"code":400,"error_code":"invalid_credentials","msg":"Invalid login credentials"}"#;
        assert_eq!(code(400, body), "invalidCredentials");
        assert_eq!(code(401, r#"{"message":"JWT expired"}"#), "sessionInvalid");
        let err = api_error(404, r#"{"message":"relation \"public.notes\" does not exist"}"#);
        assert_eq!(err.code(), "server");
        assert!(err.detail().starts_with("404: relation"));
        assert_eq!(api_error(500, "boom").detail(), "500: boom");
    }
}
