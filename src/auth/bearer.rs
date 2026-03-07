use anyhow::{anyhow, Result};
use reqwest::blocking::{RequestBuilder, Response};

use super::AuthPlugin;

/// CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-42:
/// Bearer authentication plugin with token state ownership.
#[derive(Debug)]
pub struct BearerAuth {
    token: String,
}

impl BearerAuth {
    /// CAUS-SESSIONAUT-41:
    /// Validates and stores bearer token.
    pub fn new(credentials: &str) -> Result<Self> {
        let token = credentials.trim();
        if token.is_empty() {
            return Err(anyhow!("bearer token cannot be empty"));
        }

        Ok(Self {
            token: token.to_string(),
        })
    }

    pub(crate) fn placeholder() -> Self {
        Self {
            token: "registry-token".to_string(),
        }
    }
}

impl AuthPlugin for BearerAuth {
    fn name(&self) -> &'static str {
        "bearer"
    }

    fn apply(&self, req: RequestBuilder) -> RequestBuilder {
        req.header("Authorization", format!("Bearer {}", self.token))
    }

    fn handle_401(&self, _req: RequestBuilder, _res: &Response) -> Option<RequestBuilder> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::BearerAuth;
    use crate::auth::AuthPlugin;

    #[test]
    fn new_token_ok() {
        let auth = BearerAuth::new("mytoken").expect("token should parse");
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
        assert_eq!(header, "Bearer mytoken");
    }

    #[test]
    fn new_empty_err() {
        let err = BearerAuth::new("").expect_err("empty token must fail");
        assert!(err.to_string().contains("cannot be empty"));
    }
}
