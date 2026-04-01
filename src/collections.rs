use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::cli::CliArgs;
use crate::config::config_root_dir;

/// Persisted request collection entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionEntry {
    pub alias: String,
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub created: String,
}

/// Saves a named request collection from current CLI arguments.
pub fn save_request(alias: &str, cli: &CliArgs) -> Result<()> {
    let entry = CollectionEntry {
        alias: alias.to_string(),
        method: cli.method.clone(),
        url: cli.url.clone(),
        items: cli.request_items.clone(),
        headers: HashMap::new(),
        created: Utc::now().to_rfc3339(),
    };
    let path = collection_path(alias)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create collections dir: {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&entry).context("failed to encode collection JSON")?;
    std::fs::write(&path, json).with_context(|| format!("failed to save {}", path.display()))?;
    Ok(())
}

/// Loads and prints summary for a named request collection run.
pub fn run_request(alias: &str, profile: Option<&str>) -> Result<()> {
    let entry = load_request(alias)?;
    if let Some(profile) = profile {
        eprintln!(
            "Running saved request '{}' with env profile '{}'",
            entry.alias, profile
        );
    } else {
        eprintln!("Running saved request '{}'", entry.alias);
    }
    Ok(())
}

/// Lists all saved request collections.
pub fn list_requests() -> Result<Vec<CollectionEntry>> {
    let dir = collections_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read collections dir: {}", dir.display()))?
    {
        let entry = entry.context("failed to read collection directory entry")?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read collection: {}", path.display()))?;
        let parsed: CollectionEntry = serde_json::from_str(&data)
            .with_context(|| format!("failed to parse collection: {}", path.display()))?;
        out.push(parsed);
    }
    out.sort_by(|a, b| a.alias.cmp(&b.alias));
    Ok(out)
}

/// Deletes a saved request collection by alias.
pub fn delete_request(alias: &str) -> Result<()> {
    let path = collection_path(alias)?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("failed to delete collection: {}", path.display()))?;
    }
    Ok(())
}

/// Loads a collection entry by alias.
pub fn load_request(alias: &str) -> Result<CollectionEntry> {
    let path = collection_path(alias)?;
    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read collection: {}", path.display()))?;
    let entry: CollectionEntry = serde_json::from_str(&data)
        .with_context(|| format!("failed to parse collection: {}", path.display()))?;
    Ok(entry)
}

fn collections_dir() -> Result<PathBuf> {
    let root = config_root_dir()?;
    Ok(root.join("collections"))
}

fn collection_path(alias: &str) -> Result<PathBuf> {
    Ok(collections_dir()?.join(format!("{alias}.json")))
}
