use anyhow::{anyhow, Result};
use reqwest::blocking::{RequestBuilder, Response};
use std::collections::HashMap;

pub mod basic;
pub mod bearer;
pub mod digest;

use basic::BasicAuth;
use bearer::BearerAuth;
use digest::DigestAuth;

/// CAUS-PLUGINMGMT-31, CAUS-PLUGINMGMT-32, CAUS-PLUGINMGMT-33, CAUS-PLUGINMGMT-35, CAUS-SESSIONAUT-41:
/// Auth plugin contract used for all authentication application paths.
pub trait AuthPlugin: Send + Sync {
    /// CAUS-PLUGINMGMT-31:
    /// Stable plugin identity for registry and diagnostics.
    fn name(&self) -> &'static str;

    /// CAUS-PLUGINMGMT-31:
    /// Optional auth realm metadata.
    fn realm(&self) -> Option<&str> {
        None
    }

    /// CAUS-PLUGINMGMT-33, CAUS-SESSIONAUT-41:
    /// Applies authentication to an outgoing request builder.
    fn apply(&self, req: RequestBuilder) -> RequestBuilder;

    /// CAUS-SESSIONAUT-42:
    /// Handles a 401 response and optionally returns a retry-ready request builder.
    fn handle_401(&self, _req: RequestBuilder, _response: &Response) -> Option<RequestBuilder> {
        None
    }
}

/// CAUS-PLUGINMGMT-31:
/// Auth plugin registry that owns registered plugin implementations.
pub struct AuthRegistry {
    plugins: HashMap<&'static str, Box<dyn AuthPlugin>>,
}

impl Default for AuthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthRegistry {
    /// CAUS-PLUGINMGMT-31:
    /// Creates an empty auth registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// CAUS-PLUGINMGMT-31:
    /// Creates registry with built-in auth plugin names registered.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(BasicAuth::placeholder()));
        registry.register(Box::new(BearerAuth::placeholder()));
        registry.register(Box::new(DigestAuth::placeholder()));
        registry
    }

    /// CAUS-PLUGINMGMT-31:
    /// Registers an auth plugin by its stable name.
    pub fn register(&mut self, plugin: Box<dyn AuthPlugin>) {
        self.plugins.insert(plugin.name(), plugin);
    }

    /// CAUS-PLUGINMGMT-31:
    /// Gets plugin by name.
    pub fn get(&self, name: &str) -> Result<&dyn AuthPlugin> {
        let key = name.to_ascii_lowercase();
        self.plugins
            .get(key.as_str())
            .map(|p| p.as_ref())
            .ok_or_else(|| anyhow!("unsupported auth type '{name}'"))
    }

    /// CAUS-PLUGINMGMT-31:
    /// Lists registered auth plugin names.
    pub fn list(&self) -> Vec<&'static str> {
        let mut keys = self.plugins.keys().copied().collect::<Vec<_>>();
        keys.sort_unstable();
        keys
    }
}

/// CAUS-PLUGINMGMT-31, CAUS-SESSIONAUT-41, CAUS-SESSIONAUT-45:
/// Builds auth plugin from selected type and credential payload.
pub fn build_auth(auth_type: &str, credentials: &str) -> Result<Box<dyn AuthPlugin>> {
    let normalized = auth_type.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "basic" => Ok(Box::new(BasicAuth::new(credentials)?)),
        "bearer" => Ok(Box::new(BearerAuth::new(credentials)?)),
        "digest" => Ok(Box::new(DigestAuth::new(credentials)?)),
        _ => Err(anyhow!("unsupported auth type '{auth_type}'")),
    }
}
