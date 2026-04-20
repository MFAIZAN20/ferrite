use anyhow::{Context, Result};

use crate::config::Config;
use crate::plugins::{list_plugins, plugins_dir, print_plugins};

/// Prints community plugin installation guidance.
pub fn install_plugin(name: &str) -> Result<()> {
    println!("To install a ferrite plugin:");
    println!("  1. cargo install ferrite-plugin-{name}");
    println!("  2. Place the .toml manifest in ~/.config/ferrite/plugins/");
    println!("  See https://github.com/MFAIZAN20/ferrite/wiki/plugins");
    Ok(())
}

/// Removes plugin manifest from plugins directory.
pub fn uninstall_plugin(name: &str, config: &Config) -> Result<()> {
    let dir = plugins_dir(config);
    let manifest = dir.join(format!("{name}.toml"));
    if manifest.exists() {
        std::fs::remove_file(&manifest)
            .with_context(|| format!("failed to remove plugin manifest: {}", manifest.display()))?;
        println!("Removed plugin manifest: {}", manifest.display());
    } else {
        println!("Plugin manifest not found: {}", manifest.display());
    }
    Ok(())
}

/// Lists and prints plugins.
pub fn print_plugin_list(config: &Config) -> Result<()> {
    let plugins = list_plugins(config);
    print_plugins(&plugins);
    Ok(())
}
