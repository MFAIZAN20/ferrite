use anyhow::{anyhow, Context, Result};

/// CAUS-CORERUNTIM-04:
/// Removes UTF-8 BOM from byte stream when present.
pub fn strip_bom(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    }
}

/// CAUS-CORERUNTIM-04:
/// Detects text encoding from leading BOM when available.
pub fn detect_encoding(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        "utf-8"
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        "utf-16le"
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        "utf-16be"
    } else {
        "utf-8"
    }
}

/// CAUS-CORERUNTIM-03:
/// Marks data as binary when null bytes exist or printable ratio is too low.
pub fn is_binary(bytes: &[u8]) -> bool {
    let sample = if bytes.len() > 512 {
        &bytes[..512]
    } else {
        bytes
    };
    if sample.is_empty() {
        return false;
    }

    if sample.contains(&0) {
        return true;
    }

    let mut non_printable = 0usize;
    for b in sample {
        let printable = matches!(*b, 0x09 | 0x0A | 0x0D | 0x20..=0x7E);
        if !printable {
            non_printable += 1;
        }
    }

    let ratio = non_printable as f64 / sample.len() as f64;
    ratio > 0.30
}

/// CAUS-OUTPUT-11:
/// Converts byte count to a human-readable size string.
pub fn humanize_bytes(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else if n < 1024 * 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", n as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// CAUS-OUTPUT-11:
/// Converts millisecond duration to compact display value.
pub fn humanize_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else {
        format!("{:.1}s", ms as f64 / 1000.0)
    }
}

/// CAUS-CLI-21, CAUS-CORERUNTIM-01:
/// Ensures URL has a scheme and validates it through `url` parser.
pub fn normalize_url(url: &str, default_scheme: &str) -> Result<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("URL cannot be empty"));
    }

    let candidate = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("{}://{}", default_scheme.trim(), trimmed)
    };

    let parsed =
        url::Url::parse(&candidate).with_context(|| format!("invalid URL: {candidate}"))?;

    let scheme = parsed.scheme();
    if scheme.is_empty() {
        return Err(anyhow!("invalid URL, missing scheme: {candidate}"));
    }
    if parsed.host_str().is_none() {
        return Err(anyhow!("invalid URL, missing host: {candidate}"));
    }

    Ok(parsed.to_string())
}

/// CAUS-CLI-21, CAUS-CORERUNTIM-01:
/// Backward-compatible wrapper kept for existing call sites.
pub fn build_usable_url(url: &str, default_scheme: &str) -> Result<String> {
    normalize_url(url, default_scheme)
}

/// CAUS-OUTPUT-11:
/// Reads active terminal width in columns, with a stable fallback.
pub fn terminal_width() -> usize {
    let term = console::Term::stdout();
    let (rows, cols) = term.size();
    let _ = rows;
    if cols > 0 {
        cols as usize
    } else {
        80
    }
}

/// CAUS-OUTPUT-11:
/// Truncates a string by character count and appends an ellipsis when needed.
pub fn truncate_str(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }

    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }

    if max == 1 {
        return "…".to_string();
    }

    let truncated: String = s.chars().take(max - 1).collect();
    format!("{truncated}…")
}
