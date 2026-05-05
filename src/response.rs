use serde::{Deserialize, Serialize};

/// CAUS-CORERUNTIM-01, CAUS-CORERUNTIM-05:
/// Canonical normalized response contract for rendering and session updates.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseData {
    pub status_code: u16,
    pub reason: String,
    pub final_url: String,
    pub headers: Vec<(String, String)>,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

/// CAUS-CORERUNTIM-01:
/// Request trace used for `--print` request-side sections.
#[derive(Clone, Debug)]
pub struct RequestTrace {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body_preview: Option<String>,
}
