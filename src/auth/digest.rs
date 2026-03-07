use anyhow::{anyhow, Result};
use md5::{Digest as Md5Digest, Md5};
use reqwest::blocking::{RequestBuilder, Response};
use reqwest::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use sha2::Sha256;

use super::AuthPlugin;

/// CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-42:
/// Digest authentication plugin implementing RFC 7616 challenge-response flow.
pub struct DigestAuth {
    username: String,
    password: String,
}

/// CAUS-SESSIONAUT-41:
/// Parsed digest challenge state from `WWW-Authenticate`.
#[derive(Clone, Debug)]
struct DigestChallenge {
    realm: String,
    nonce: String,
    opaque: Option<String>,
    qop: Option<String>,
    algorithm: DigestAlgorithm,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum DigestAlgorithm {
    MD5,
    SHA256,
}

impl DigestAuth {
    /// CAUS-SESSIONAUT-41:
    /// Parses digest credentials from `user:pass`.
    pub fn new(credentials: &str) -> Result<Self> {
        let (user, pass) = credentials
            .split_once(':')
            .ok_or_else(|| anyhow!("digest auth requires user:password format"))?;
        if user.is_empty() {
            return Err(anyhow!("digest auth user cannot be empty"));
        }

        Ok(Self {
            username: user.to_string(),
            password: pass.to_string(),
        })
    }

    pub(crate) fn placeholder() -> Self {
        Self {
            username: "registry".to_string(),
            password: "registry".to_string(),
        }
    }

    fn compute_response_with_values(
        &self,
        challenge: &DigestChallenge,
        method: &str,
        uri: &str,
        cnonce: &str,
        nc: &str,
    ) -> String {
        let ha1 = hash_hex(
            challenge.algorithm,
            &format!("{}:{}:{}", self.username, challenge.realm, self.password),
        );
        let ha2 = hash_hex(challenge.algorithm, &format!("{}:{}", method, uri));

        match challenge.qop.as_deref() {
            Some(qop)
                if qop.eq_ignore_ascii_case("auth") || qop.eq_ignore_ascii_case("auth-int") =>
            {
                hash_hex(
                    challenge.algorithm,
                    &format!(
                        "{}:{}:{}:{}:{}:{}",
                        ha1, challenge.nonce, nc, cnonce, qop, ha2
                    ),
                )
            }
            _ => hash_hex(
                challenge.algorithm,
                &format!("{}:{}:{}", ha1, challenge.nonce, ha2),
            ),
        }
    }

    fn build_auth_header(&self, challenge: &DigestChallenge, method: &str, uri: &str) -> String {
        let cnonce = generate_cnonce();
        let nc = "00000001";
        let response = self.compute_response_with_values(challenge, method, uri, &cnonce, nc);

        let mut parts = vec![
            format!("username=\"{}\"", self.username),
            format!("realm=\"{}\"", challenge.realm),
            format!("nonce=\"{}\"", challenge.nonce),
            format!("uri=\"{}\"", uri),
            format!(
                "algorithm={}",
                match challenge.algorithm {
                    DigestAlgorithm::MD5 => "MD5",
                    DigestAlgorithm::SHA256 => "SHA-256",
                }
            ),
            format!("response=\"{}\"", response),
        ];

        if let Some(qop) = &challenge.qop {
            parts.push(format!("qop={}", qop));
            parts.push(format!("nc={}", nc));
            parts.push(format!("cnonce=\"{}\"", cnonce));
        }

        if let Some(opaque) = &challenge.opaque {
            parts.push(format!("opaque=\"{}\"", opaque));
        }

        format!("Digest {}", parts.join(", "))
    }

    fn retry_from_challenge(
        &self,
        req: RequestBuilder,
        challenge_value: Option<&str>,
    ) -> Option<RequestBuilder> {
        let challenge_text = challenge_value?;
        let challenge = DigestChallenge::parse(challenge_text).ok()?;

        let request_for_parse = req.try_clone()?;
        let built = request_for_parse.build().ok()?;
        let method = built.method().as_str().to_string();
        let mut uri = built.url().path().to_string();
        if let Some(query) = built.url().query() {
            uri.push('?');
            uri.push_str(query);
        }

        let auth_header = self.build_auth_header(&challenge, &method, &uri);
        Some(req.header(AUTHORIZATION, auth_header))
    }
}

impl AuthPlugin for DigestAuth {
    fn name(&self) -> &'static str {
        "digest"
    }

    fn apply(&self, req: RequestBuilder) -> RequestBuilder {
        req
    }

    fn handle_401(&self, req: RequestBuilder, response: &Response) -> Option<RequestBuilder> {
        let challenge_value = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .and_then(|v| v.to_str().ok());
        self.retry_from_challenge(req, challenge_value)
    }
}

impl DigestChallenge {
    fn parse(header_value: &str) -> Result<Self> {
        let raw = header_value.trim();
        let payload = raw
            .strip_prefix("Digest")
            .or_else(|| raw.strip_prefix("digest"))
            .ok_or_else(|| anyhow!("WWW-Authenticate is not a Digest challenge"))?
            .trim();

        let mut realm = None;
        let mut nonce = None;
        let mut opaque = None;
        let mut qop = None;
        let mut algorithm = DigestAlgorithm::MD5;
        for part in split_comma_aware(payload) {
            let Some((k, v)) = part.split_once('=') else {
                continue;
            };
            let key = k.trim().to_ascii_lowercase();
            let mut value = v.trim().to_string();
            if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                value = value[1..value.len() - 1].to_string();
            }

            match key.as_str() {
                "realm" => realm = Some(value),
                "nonce" => nonce = Some(value),
                "opaque" => opaque = Some(value),
                "domain" | "stale" => {}
                "qop" => {
                    qop = choose_qop(&value);
                }
                "algorithm" => {
                    algorithm = parse_algorithm(&value);
                }
                _ => {}
            }
        }

        let realm = realm.ok_or_else(|| anyhow!("digest challenge missing realm"))?;
        let nonce = nonce.ok_or_else(|| anyhow!("digest challenge missing nonce"))?;

        Ok(Self {
            realm,
            nonce,
            opaque,
            qop,
            algorithm,
        })
    }
}

fn parse_algorithm(raw: &str) -> DigestAlgorithm {
    if raw.eq_ignore_ascii_case("sha-256") || raw.eq_ignore_ascii_case("sha256") {
        DigestAlgorithm::SHA256
    } else if raw.eq_ignore_ascii_case("md5") {
        DigestAlgorithm::MD5
    } else {
        eprintln!("digest warning: unsupported algorithm '{raw}', falling back to MD5");
        DigestAlgorithm::MD5
    }
}

fn choose_qop(raw: &str) -> Option<String> {
    for candidate in raw.split(',').map(|v| v.trim()) {
        if candidate.eq_ignore_ascii_case("auth") {
            return Some("auth".to_string());
        }
        if candidate.eq_ignore_ascii_case("auth-int") {
            return Some("auth-int".to_string());
        }
    }
    None
}

fn split_comma_aware(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if !in_quotes => {
                let part = current.trim();
                if !part.is_empty() {
                    out.push(part.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let part = current.trim();
    if !part.is_empty() {
        out.push(part.to_string());
    }

    out
}

fn hash_hex(algorithm: DigestAlgorithm, payload: &str) -> String {
    match algorithm {
        DigestAlgorithm::MD5 => {
            let mut hasher = Md5::new();
            hasher.update(payload.as_bytes());
            let out = hasher.finalize();
            to_hex(&out)
        }
        DigestAlgorithm::SHA256 => {
            let mut hasher = Sha256::new();
            hasher.update(payload.as_bytes());
            let out = hasher.finalize();
            to_hex(&out)
        }
    }
}

fn generate_cnonce() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{DigestAlgorithm, DigestAuth, DigestChallenge};

    #[test]
    fn parse_challenge_md5_auth() {
        let challenge = DigestChallenge::parse(
            "Digest realm=\"http-auth@example.org\", nonce=\"abc\", qop=\"auth,auth-int\", algorithm=MD5",
        )
        .expect("digest challenge should parse");

        assert_eq!(challenge.realm, "http-auth@example.org");
        assert_eq!(challenge.nonce, "abc");
        assert_eq!(challenge.qop.as_deref(), Some("auth"));
        assert_eq!(challenge.algorithm, DigestAlgorithm::MD5);
    }

    #[test]
    fn parse_challenge_sha256() {
        let challenge = DigestChallenge::parse(
            "Digest realm=\"http-auth@example.org\", nonce=\"abc\", qop=\"auth\", algorithm=SHA-256",
        )
        .expect("digest challenge should parse");
        assert_eq!(challenge.algorithm, DigestAlgorithm::SHA256);
    }

    #[test]
    fn parse_missing_nonce_err() {
        let err =
            DigestChallenge::parse("Digest realm=\"x\"").expect_err("nonce is required for digest");
        assert!(err.to_string().contains("missing nonce"));
    }

    #[test]
    fn compute_response_md5_rfc_vector() {
        let auth = DigestAuth::new("Mufasa:Circle of Life").expect("credentials should parse");
        let challenge = DigestChallenge::parse(
            "Digest realm=\"http-auth@example.org\", nonce=\"7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v\", qop=\"auth\", algorithm=MD5, opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\"",
        )
        .expect("challenge should parse");

        let response = auth.compute_response_with_values(
            &challenge,
            "GET",
            "/dir/index.html",
            "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ",
            "00000001",
        );

        assert_eq!(response, "8ca523f5e9506fed4657c9700eebdbec");
    }

    #[test]
    fn compute_response_sha256_rfc_vector() {
        let auth = DigestAuth::new("Mufasa:Circle of Life").expect("credentials should parse");
        let challenge = DigestChallenge::parse(
            "Digest realm=\"http-auth@example.org\", nonce=\"7ypf/xlj9XXwfDPEoM4URrv/xwf94BcCAzFZH4GiTo0v\", qop=\"auth\", algorithm=SHA-256, opaque=\"FQhe/qaU925kfnzjCev0ciny7QMkPqMAFRtzCUYo5tdS\"",
        )
        .expect("challenge should parse");

        let response = auth.compute_response_with_values(
            &challenge,
            "GET",
            "/dir/index.html",
            "f2/wE4q74E6zIJEtWaHKaf5wv/H5QzzpXusqGemxURZJ",
            "00000001",
        );

        assert_eq!(
            response,
            "753927fa0e85d155564e2e272a28d1802ca10daf4496794697cf8db5856cb6c1"
        );
    }

    #[test]
    fn handle_401_valid_challenge_returns_retry_builder() {
        let client = reqwest::blocking::Client::new();
        let base_builder = client.get("https://example.com/dir/index.html");
        let challenge =
            "Digest realm=\"http-auth@example.org\", nonce=\"abc123\", qop=\"auth\", algorithm=MD5";

        let auth = DigestAuth::new("user:pass").expect("credentials should parse");
        let retry = auth
            .retry_from_challenge(base_builder, Some(challenge))
            .expect("retry builder should be produced");

        let built = retry.build().expect("retry request should build");
        let header = built
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .expect("authorization header should exist")
            .to_str()
            .expect("header should be utf8");
        assert!(header.starts_with("Digest "));
        assert!(header.contains("response=\""));
    }

    #[test]
    fn handle_401_missing_header_returns_none() {
        let client = reqwest::blocking::Client::new();
        let base_builder = client.get("https://example.com/x");
        let auth = DigestAuth::new("user:pass").expect("credentials should parse");
        let retry = auth.retry_from_challenge(base_builder, None);
        assert!(retry.is_none());
    }
}
