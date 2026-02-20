use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::config_root_dir;
use crate::items::RequestItem;
use crate::response::ResponseData;

/// CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-45:
/// Persisted auth snapshot within a session file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionAuth {
    #[serde(rename = "type")]
    pub auth_type: String,
    pub username: String,
    pub password: String,
}

/// CAUS-SESSIONAUT-41:
/// Persisted cookie snapshot within a session file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_cookie_path")]
    pub path: String,
    #[serde(default)]
    pub secure: bool,
    pub expires: Option<String>,
}

/// CAUS-SESSIONAUT-45:
/// Session metadata used for lifecycle tracking.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionMeta {
    pub created: String,
    pub last_used: String,
}

/// CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-45:
/// Session persistence contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionData {
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub auth: Option<SessionAuth>,
    #[serde(default)]
    pub cookies: Vec<SessionCookie>,
    #[serde(default = "default_session_meta")]
    pub meta: SessionMeta,
}

impl Default for SessionData {
    fn default() -> Self {
        let now = now_iso();
        Self {
            headers: HashMap::new(),
            auth: None,
            cookies: Vec::new(),
            meta: SessionMeta {
                created: now.clone(),
                last_used: now,
            },
        }
    }
}

/// CAUS-SESSIONAUT-41:
/// Loads session state from disk if session is requested.
pub fn load_session(
    url: &str,
    session_ref: Option<&str>,
) -> Result<Option<(PathBuf, SessionData)>> {
    let Some(reference) = session_ref else {
        return Ok(None);
    };

    let path = resolve_session_path(url, reference)?;
    if !path.exists() {
        return Ok(Some((path, SessionData::default())));
    }

    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read session file: {}", path.display()))?;
    let mut session: SessionData = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse session file: {}", path.display()))?;
    session.meta.last_used = now_iso();
    if session.meta.created.trim().is_empty() {
        session.meta.created = session.meta.last_used.clone();
    }

    Ok(Some((path, session)))
}

/// CAUS-SESSIONAUT-45:
/// Saves session back to disk unless read-only was requested.
pub fn save_session(path: &Path, session: &SessionData) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create session dir: {}", parent.display()))?;
    }

    let mut staged = session.clone();
    if staged.meta.created.trim().is_empty() {
        staged.meta.created = now_iso();
    }
    staged.meta.last_used = now_iso();

    let text = serde_json::to_string_pretty(&staged).context("failed to encode session JSON")?;
    std::fs::write(path, text)
        .with_context(|| format!("failed to write session file: {}", path.display()))?;
    Ok(())
}

/// CAUS-SESSIONAUT-41:
/// Merges session headers/cookies/auth into request items and auth selection.
pub fn apply_session_to_request(
    request_items: &mut Vec<RequestItem>,
    auth_type: &mut String,
    auth_value: &mut Option<String>,
    session: &SessionData,
) {
    let mut existing_headers = HashMap::new();
    for item in request_items.iter() {
        if let RequestItem::Header { key, .. } = item {
            existing_headers.insert(key.to_ascii_lowercase(), true);
        }
    }

    for (k, v) in &session.headers {
        if !existing_headers.contains_key(&k.to_ascii_lowercase()) {
            request_items.push(RequestItem::Header {
                key: k.clone(),
                value: v.clone(),
            });
        }
    }

    if !session.cookies.is_empty() {
        let cookie_line = session
            .cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");

        if !cookie_line.is_empty() {
            request_items.push(RequestItem::Header {
                key: "Cookie".to_string(),
                value: cookie_line,
            });
        }
    }

    if auth_value.is_none() {
        if let Some(a) = &session.auth {
            *auth_type = a.auth_type.clone();
            *auth_value = if a.auth_type.eq_ignore_ascii_case("bearer") {
                Some(a.password.clone())
            } else {
                Some(format!("{}:{}", a.username, a.password))
            };
        }
    }
}

/// CAUS-SESSIONAUT-45:
/// Updates session data from outgoing auth and incoming response headers.
pub fn update_session_from_exchange(
    session: &mut SessionData,
    request_items: &[RequestItem],
    auth_type: &str,
    auth_value: Option<&str>,
    response: &ResponseData,
) {
    for item in request_items {
        if let RequestItem::Header { key, value } = item {
            if key.eq_ignore_ascii_case("cookie") {
                continue;
            }
            session.headers.insert(key.clone(), value.clone());
        }
    }

    if let Some(value) = auth_value {
        if auth_type.eq_ignore_ascii_case("basic") {
            if let Some((user, pass)) = value.split_once(':') {
                session.auth = Some(SessionAuth {
                    auth_type: "basic".to_string(),
                    username: user.to_string(),
                    password: pass.to_string(),
                });
            }
        } else if auth_type.eq_ignore_ascii_case("bearer") {
            session.auth = Some(SessionAuth {
                auth_type: "bearer".to_string(),
                username: String::new(),
                password: value.to_string(),
            });
        }
    }

    for (name, value) in &response.headers {
        if !name.eq_ignore_ascii_case("set-cookie") {
            continue;
        }
        if let Some(cookie) = parse_set_cookie(value, &response.final_url) {
            session.cookies.retain(|c| {
                !(c.name == cookie.name && c.domain == cookie.domain && c.path == cookie.path)
            });
            session.cookies.push(cookie);
        }
    }

    session.meta.last_used = now_iso();
}

/// CAUS-SESSIONAUT-41:
/// Resolves session path from host/name or explicit file path.
fn resolve_session_path(url: &str, reference: &str) -> Result<PathBuf> {
    let as_path = Path::new(reference);
    if reference.contains('/') || reference.contains('\\') || as_path.extension().is_some() {
        return Ok(as_path.to_path_buf());
    }

    let parsed = reqwest::Url::parse(url)
        .with_context(|| format!("failed to parse URL for session host: {url}"))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("URL does not include host: {url}"))?;

    let root = config_root_dir()?;
    Ok(root
        .join("sessions")
        .join(host)
        .join(format!("{reference}.json")))
}

/// CAUS-SESSIONAUT-45:
/// Parses Set-Cookie header into persisted session cookie.
fn parse_set_cookie(raw: &str, url: &str) -> Option<SessionCookie> {
    let mut parts = raw.split(';');
    let first = parts.next()?;
    let (name, value) = first.split_once('=')?;

    let mut domain = reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default();
    let mut path = default_cookie_path();
    let mut secure = false;
    let mut expires = None;

    for p in parts {
        let trimmed = p.trim();
        if trimmed.eq_ignore_ascii_case("secure") {
            secure = true;
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            if k.eq_ignore_ascii_case("domain") {
                domain = v.to_string();
            } else if k.eq_ignore_ascii_case("path") {
                path = v.to_string();
            } else if k.eq_ignore_ascii_case("expires") {
                expires = Some(v.to_string());
            }
        }
    }

    Some(SessionCookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain,
        path,
        secure,
        expires,
    })
}

/// CAUS-SESSIONAUT-45:
/// Returns an ISO-8601 UTC timestamp for session metadata.
fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// CAUS-SESSIONAUT-41:
/// Default cookie path used when Set-Cookie omits path.
fn default_cookie_path() -> String {
    "/".to_string()
}

fn default_session_meta() -> SessionMeta {
    let now = now_iso();
    SessionMeta {
        created: now.clone(),
        last_used: now,
    }
}
