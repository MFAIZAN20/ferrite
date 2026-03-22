use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Response;
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cli::CliArgs;
use crate::output::theme::Theme;
use crate::utils::humanize_bytes;

/// Result payload for download mode.
#[derive(Clone, Debug)]
pub struct DownloadResult {
    pub filename: String,
    pub size: u64,
    pub duration: Duration,
    pub resumed: bool,
}

/// Streams a response body to disk with progress display and optional resume append.
pub fn download(mut response: Response, cli: &CliArgs, theme: &Theme) -> Result<DownloadResult> {
    let filename = resolve_filename_from_response(&response, cli);
    let output_path = PathBuf::from(&filename);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create download dir: {}", parent.display()))?;
        }
    }

    let mut resumed = false;
    let mut resume_from = 0u64;
    if cli.continue_download && output_path.exists() {
        resume_from = output_path.metadata().map(|m| m.len()).unwrap_or(0);
        if resume_from > 0 {
            resumed = true;
            println!(
                "{}",
                format!("Resuming from {} bytes", resume_from)
                    .color(theme.offline_msg)
                    .bold()
            );
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&output_path)
        .with_context(|| format!("failed to open output file: {}", output_path.display()))?;

    let total = response
        .content_length()
        .map(|n| n + resume_from)
        .unwrap_or(0);
    let progress = if total > 0 {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
            )
            .context("failed to set progress style")?,
        );
        pb.set_position(resume_from);
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("{spinner} {elapsed_precise} {bytes} ({bytes_per_sec})")
                .context("failed to set spinner style")?,
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    };

    let started = Instant::now();
    let mut buf = [0u8; 16 * 1024];
    let mut written = resume_from;
    loop {
        let n = response
            .read(&mut buf)
            .context("failed while reading download stream")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .context("failed while writing download output")?;
        written += n as u64;
        progress.set_position(written);
    }
    progress.finish_and_clear();

    let duration = started.elapsed();
    let speed = if duration.as_secs_f64() > 0.0 {
        (written.saturating_sub(resume_from)) as f64 / duration.as_secs_f64()
    } else {
        0.0
    };
    println!("✔ Downloaded: {}", output_path.display());
    println!("  Size:  {}", humanize_bytes(written));
    println!("  Time:  {:.1}s", duration.as_secs_f64());
    println!("  Speed: {}/s", humanize_bytes(speed as u64));

    Ok(DownloadResult {
        filename,
        size: written,
        duration,
        resumed,
    })
}

fn resolve_filename_from_response(response: &Response, cli: &CliArgs) -> String {
    resolve_filename(
        cli.output.as_deref(),
        response
            .headers()
            .get(CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok()),
        response.url().as_str(),
        response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
    )
}

fn resolve_filename(
    output_override: Option<&str>,
    content_disposition: Option<&str>,
    url: &str,
    content_type: Option<&str>,
) -> String {
    if let Some(path) = output_override {
        return path.to_string();
    }

    if let Some(disposition) = content_disposition {
        if let Some(name) = parse_content_disposition_filename(disposition) {
            return name;
        }
    }

    if let Ok(parsed) = reqwest::Url::parse(url) {
        if let Some(last) = parsed.path_segments().and_then(|mut seg| seg.next_back()) {
            if !last.is_empty() {
                return last.to_string();
            }
        }
    }

    let extension = content_type
        .and_then(|ct| mime_guess::get_mime_extensions_str(ct).and_then(|arr| arr.first().copied()))
        .unwrap_or("bin");
    format!("download.{extension}")
}

fn parse_content_disposition_filename(raw: &str) -> Option<String> {
    for part in raw.split(';') {
        let trimmed = part.trim();
        if let Some(v) = trimmed.strip_prefix("filename=") {
            return Some(v.trim_matches('"').to_string());
        }
    }
    None
}

#[allow(dead_code)]
fn file_exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::{parse_content_disposition_filename, resolve_filename};
    use crate::utils::humanize_bytes;

    #[test]
    fn filename_from_content_disposition() {
        let filename = resolve_filename(
            None,
            Some("attachment; filename=\"report.txt\""),
            "https://example.com/download",
            None,
        );
        assert_eq!(filename, "report.txt");
    }

    #[test]
    fn filename_from_url_path() {
        let filename = resolve_filename(None, None, "https://example.com/files/data.json", None);
        assert_eq!(filename, "data.json");
    }

    #[test]
    fn filename_fallback_download_bin() {
        let filename = resolve_filename(None, None, "https://example.com/", None);
        assert_eq!(filename, "download.bin");
        assert_eq!(parse_content_disposition_filename("inline"), None);
        assert_eq!(humanize_bytes(1536), "1.5 KB");
    }
}
