use anyhow::{anyhow, Context, Result};
use serde_json::Value;

/// CAUS-CLI-21, CAUS-CLI-25:
/// Parsed request-item variants from HTTPie-style token syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum RequestItem {
    Header {
        key: String,
        value: String,
    },
    DataString {
        key: String,
        value: String,
    },
    DataJson {
        key: String,
        value: Value,
    },
    QueryParam {
        key: String,
        value: String,
    },
    FileUpload {
        key: String,
        path: String,
    },
    FileUploadType {
        key: String,
        path: String,
        content_type: String,
    },
    DataFromFile {
        key: String,
        path: String,
    },
    JsonFromFile {
        key: String,
        path: String,
    },
}

/// CAUS-CLI-21:
/// File upload payload from request items.
#[derive(Clone, Debug, PartialEq)]
pub struct FileInput {
    pub key: String,
    pub path: String,
    pub content_type: Option<String>,
}

/// CAUS-CLI-21, CAUS-CLI-22:
/// Aggregated request item groups for request builder generation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectedItems {
    pub headers: Vec<(String, String)>,
    pub data_strings: Vec<(String, String)>,
    pub data_json: Vec<(String, Value)>,
    pub query_params: Vec<(String, String)>,
    pub files: Vec<FileInput>,
}

/// CAUS-CLI-21:
/// Parses a list of request-item tokens into structured variants.
pub fn parse_request_items(tokens: &[String]) -> Result<Vec<RequestItem>> {
    tokens.iter().map(|raw| parse_item(raw)).collect()
}

/// CAUS-CLI-21:
/// Parses one request-item token with required operator precedence.
pub fn parse_item(raw: &str) -> Result<RequestItem> {
    let token = raw.trim();
    if token.is_empty() {
        return Err(anyhow!("request item cannot be empty"));
    }

    // Operator precedence: :=@ and := before == before : before =@ and = before @
    if let Some((key, path)) = token.split_once(":=@") {
        let key = validate_key(key, token)?;
        let path = validate_non_empty(path.trim(), token, "JSON file path")?;
        return Ok(RequestItem::JsonFromFile {
            key: key.to_string(),
            path: path.to_string(),
        });
    }

    if let Some((key, value_raw)) = token.split_once(":=") {
        let key = validate_key(key, token)?;
        let value_str = validate_non_empty(value_raw.trim(), token, "JSON value")?;
        let value: Value = serde_json::from_str(value_str)
            .map_err(|e| anyhow!("invalid JSON value in '{token}': {e}"))?;
        return Ok(RequestItem::DataJson {
            key: key.to_string(),
            value,
        });
    }

    if let Some((key, value)) = token.split_once("==") {
        let key = validate_key(key, token)?;
        let value = validate_non_empty(value.trim(), token, "query value")?;
        return Ok(RequestItem::QueryParam {
            key: key.to_string(),
            value: value.to_string(),
        });
    }

    // Treat ":" as a header separator only when it appears before any
    // request-item operator that would consume the key/value boundary.
    // This keeps Windows file paths like C:\... valid in =@ and @ forms.
    if let Some(colon_idx) = token.find(':') {
        let before_eq_at = token.find("=@").is_none_or(|i| colon_idx < i);
        let before_eq = token.find('=').is_none_or(|i| colon_idx < i);
        let before_at = token.find('@').is_none_or(|i| colon_idx < i);
        if before_eq_at && before_eq && before_at {
            let (key, value) = token.split_at(colon_idx);
            let key = validate_key(key, token)?;
            let value = validate_non_empty(value[1..].trim(), token, "header value")?;
            return Ok(RequestItem::Header {
                key: key.to_string(),
                value: value.to_string(),
            });
        }
    }

    if let Some((key, path)) = token.split_once("=@") {
        let key = validate_key(key, token)?;
        let path = validate_non_empty(path.trim(), token, "data file path")?;
        return Ok(RequestItem::DataFromFile {
            key: key.to_string(),
            path: path.to_string(),
        });
    }

    if let Some((key, value)) = token.split_once('=') {
        if token.contains('@') && token.contains(";type=") {
            // Keep typed upload parsing on the @ branch even though it includes '=' in ";type=".
            // Example: field@/tmp/file;type=text/plain
        } else {
            let key = validate_key(key, token)?;
            let value = validate_non_empty(value.trim(), token, "data string value")?;
            return Ok(RequestItem::DataString {
                key: key.to_string(),
                value: value.to_string(),
            });
        }
    }

    if let Some((key, rest)) = token.split_once('@') {
        let key = validate_key(key, token)?;
        if let Some((path, content_type)) = rest.split_once(";type=") {
            let path = validate_non_empty(path.trim(), token, "file path")?;
            let content_type = validate_non_empty(content_type.trim(), token, "content type")?;
            return Ok(RequestItem::FileUploadType {
                key: key.to_string(),
                path: path.to_string(),
                content_type: content_type.to_string(),
            });
        }

        let path = validate_non_empty(rest.trim(), token, "file path")?;
        return Ok(RequestItem::FileUpload {
            key: key.to_string(),
            path: path.to_string(),
        });
    }

    Err(anyhow!("unsupported request item token: '{token}'"))
}

/// CAUS-CLI-21, CAUS-CLI-22:
/// Parses tokens and returns grouped items for request construction.
pub fn collect(items: Vec<String>) -> Result<CollectedItems> {
    let parsed = parse_request_items(&items)?;
    collect_from_parsed(&parsed)
}

/// CAUS-CLI-21:
/// Groups parsed request items, including file-to-data expansion operators.
pub fn collect_from_parsed(parsed: &[RequestItem]) -> Result<CollectedItems> {
    let mut out = CollectedItems::default();

    for item in parsed {
        match item {
            RequestItem::Header { key, value } => out.headers.push((key.clone(), value.clone())),
            RequestItem::DataString { key, value } => {
                out.data_strings.push((key.clone(), value.clone()))
            }
            RequestItem::DataJson { key, value } => {
                out.data_json.push((key.clone(), value.clone()))
            }
            RequestItem::QueryParam { key, value } => {
                out.query_params.push((key.clone(), value.clone()))
            }
            RequestItem::FileUpload { key, path } => out.files.push(FileInput {
                key: key.clone(),
                path: path.clone(),
                content_type: None,
            }),
            RequestItem::FileUploadType {
                key,
                path,
                content_type,
            } => out.files.push(FileInput {
                key: key.clone(),
                path: path.clone(),
                content_type: Some(content_type.clone()),
            }),
            RequestItem::DataFromFile { key, path } => {
                let value = std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read data file: {path}"))?;
                out.data_strings.push((key.clone(), value));
            }
            RequestItem::JsonFromFile { key, path } => {
                let raw = std::fs::read_to_string(path)
                    .with_context(|| format!("failed to read JSON file: {path}"))?;
                let value: Value = serde_json::from_str(&raw)
                    .with_context(|| format!("failed to parse JSON file: {path}"))?;
                out.data_json.push((key.clone(), value));
            }
        }
    }

    Ok(out)
}

/// CAUS-CLI-21:
/// Validates request-item key rules.
fn validate_key<'a>(key: &'a str, raw: &str) -> Result<&'a str> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("request item key is empty: '{raw}'"));
    }
    if trimmed
        .chars()
        .any(|ch| matches!(ch, ' ' | '\t' | ':' | '=' | '@'))
    {
        return Err(anyhow!(
            "request item key contains invalid characters: '{trimmed}'"
        ));
    }
    Ok(trimmed)
}

/// CAUS-CLI-21:
/// Validates non-empty values for all operators.
fn validate_non_empty<'a>(value: &'a str, raw: &str, field_name: &str) -> Result<&'a str> {
    if value.is_empty() {
        return Err(anyhow!("{field_name} is empty in token '{raw}'"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{collect, parse_item, RequestItem};
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_file(name: &str, contents: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("httpie_rust_items_{name}_{nanos}.tmp"));
        fs::write(&path, contents).expect("temp file write should succeed");
        path
    }

    #[test]
    fn parse_header_item_happy() {
        let parsed = parse_item("Accept:application/json").expect("header parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::Header {
                key: "Accept".to_string(),
                value: "application/json".to_string()
            }
        );
    }

    #[test]
    fn parse_data_string_item_happy() {
        let parsed = parse_item("name=faizan").expect("data string parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::DataString {
                key: "name".to_string(),
                value: "faizan".to_string()
            }
        );
    }

    #[test]
    fn parse_data_json_item_happy() {
        let parsed = parse_item("payload:={\"x\":1}").expect("data json parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::DataJson {
                key: "payload".to_string(),
                value: json!({"x": 1})
            }
        );
    }

    #[test]
    fn parse_query_item_happy() {
        let parsed = parse_item("page==2").expect("query parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::QueryParam {
                key: "page".to_string(),
                value: "2".to_string()
            }
        );
    }

    #[test]
    fn parse_file_upload_item_happy() {
        let parsed = parse_item("f@/tmp/a.txt").expect("file upload parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::FileUpload {
                key: "f".to_string(),
                path: "/tmp/a.txt".to_string()
            }
        );
    }

    #[test]
    fn parse_file_upload_type_item_happy() {
        let parsed =
            parse_item("f@/tmp/a.txt;type=text/plain").expect("typed upload parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::FileUploadType {
                key: "f".to_string(),
                path: "/tmp/a.txt".to_string(),
                content_type: "text/plain".to_string()
            }
        );
    }

    #[test]
    fn parse_data_from_file_item_happy() {
        let parsed = parse_item("bio=@/tmp/bio.txt").expect("data-from-file parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::DataFromFile {
                key: "bio".to_string(),
                path: "/tmp/bio.txt".to_string()
            }
        );
    }

    #[test]
    fn parse_json_from_file_item_happy() {
        let parsed =
            parse_item("cfg:=@/tmp/cfg.json").expect("json-from-file parse should succeed");
        assert_eq!(
            parsed,
            RequestItem::JsonFromFile {
                key: "cfg".to_string(),
                path: "/tmp/cfg.json".to_string()
            }
        );
    }

    #[test]
    fn empty_key_returns_error() {
        let err = parse_item("=value").expect_err("empty key should fail");
        assert!(err.to_string().contains("key is empty"));
    }

    #[test]
    fn empty_value_returns_error() {
        let err = parse_item("name=").expect_err("empty value should fail");
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn operator_precedence_json_before_string() {
        let parsed = parse_item("x:={\"a\":1}").expect("json precedence should win");
        match parsed {
            RequestItem::DataJson { .. } => {}
            _ => panic!("expected DataJson"),
        }
    }

    #[test]
    fn operator_precedence_query_before_string() {
        let parsed = parse_item("x==1").expect("query precedence should win");
        match parsed {
            RequestItem::QueryParam { .. } => {}
            _ => panic!("expected QueryParam"),
        }
    }

    #[test]
    fn unicode_key_value_supported() {
        let parsed = parse_item("na_me=سلام").expect("unicode value should parse");
        assert_eq!(
            parsed,
            RequestItem::DataString {
                key: "na_me".to_string(),
                value: "سلام".to_string()
            }
        );
    }

    #[test]
    fn whitespace_is_trimmed() {
        let parsed = parse_item("  role = admin  ").expect("whitespace token should parse");
        assert_eq!(
            parsed,
            RequestItem::DataString {
                key: "role".to_string(),
                value: "admin".to_string()
            }
        );
    }

    #[test]
    fn invalid_key_chars_are_rejected() {
        let err = parse_item("bad key=value").expect_err("space in key should fail");
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn collect_expands_file_data_tokens() {
        let text_file = make_temp_file("text", "hello world");
        let json_file = make_temp_file("json", "{\"ok\":true}");

        let items = vec![
            format!("bio=@{}", text_file.display()),
            format!("cfg:=@{}", json_file.display()),
            "X-Test:1".to_string(),
        ];

        let collected = collect(items).expect("collection should succeed");
        assert_eq!(collected.headers.len(), 1);
        assert_eq!(collected.data_strings.len(), 1);
        assert_eq!(collected.data_json.len(), 1);
        assert_eq!(collected.data_strings[0].0, "bio");
        assert_eq!(collected.data_strings[0].1, "hello world");
        assert_eq!(collected.data_json[0].0, "cfg");
        assert_eq!(collected.data_json[0].1, json!({"ok": true}));

        fs::remove_file(text_file).expect("cleanup text temp file should succeed");
        fs::remove_file(json_file).expect("cleanup json temp file should succeed");
    }
}
