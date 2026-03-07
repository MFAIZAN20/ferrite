use anyhow::{anyhow, Result};
use reqwest::blocking::{RequestBuilder, Response};

use super::AuthPlugin;

/// CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-42:
/// Basic authentication plugin with explicit username/password state.
#[derive(Debug)]
pub struct BasicAuth {
    username: String,
    password: Option<String>,
}

impl BasicAuth {
    /// CAUS-SESSIONAUT-41:
    /// Parses `user:pass` (or `user`) into auth credentials.
    pub fn new(credentials: &str) -> Result<Self> {
        let raw = credentials.trim();
        if raw.is_empty() {
            return Err(anyhow!("basic auth credentials cannot be empty"));
        }

        if let Some((user, pass)) = raw.split_once(':') {
            if user.is_empty() {
                return Err(anyhow!("basic auth username cannot be empty"));
            }
            return Ok(Self {
                username: user.to_string(),
                password: Some(pass.to_string()),
            });
        }

        Ok(Self {
            username: raw.to_string(),
            password: None,
        })
    }

    pub(crate) fn placeholder() -> Self {
        Self {
            username: "registry".to_string(),
            password: None,
        }
    }
}

impl AuthPlugin for BasicAuth {
    fn name(&self) -> &'static str {
        "basic"
    }

    fn apply(&self, req: RequestBuilder) -> RequestBuilder {
        req.basic_auth(&self.username, self.password.as_deref())
    }

    fn handle_401(&self, _req: RequestBuilder, _res: &Response) -> Option<RequestBuilder> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::BasicAuth;
    use crate::auth::AuthPlugin;

    #[test]
    fn new_user_pass() {
        let auth = BasicAuth::new("user:pass").expect("basic credentials should parse");
        let req = auth
            .apply(reqwest::blocking::Client::new().get("http://example.com"))
            .build()
            .expect("request should build");
        let header = req
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .expect("authorization header should exist")
            .to_str()
            .expect("header should be utf8");
        assert_eq!(header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn new_user_empty_pass() {
        let auth = BasicAuth::new("user:").expect("empty password should be allowed");
        let req = auth
            .apply(reqwest::blocking::Client::new().get("http://example.com"))
            .build()
            .expect("request should build");
        let header = req
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .expect("authorization header should exist")
            .to_str()
            .expect("header should be utf8");
        assert_eq!(header, "Basic dXNlcjo=");
    }

    #[test]
    fn new_user_without_pass() {
        let auth = BasicAuth::new("user").expect("username-only should be accepted");
        let req = auth
            .apply(reqwest::blocking::Client::new().get("http://example.com"))
            .build()
            .expect("request should build");
        let header = req
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .expect("authorization header should exist")
            .to_str()
            .expect("header should be utf8");
        assert!(header.starts_with("Basic "));
    }

    #[test]
    fn new_empty_error() {
        let err = BasicAuth::new("").expect_err("empty credentials must fail");
        assert!(err.to_string().contains("cannot be empty"));
    }
}
