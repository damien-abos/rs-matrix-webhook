use anyhow::{Context, Result};
use config::{Config, Environment, File};
use serde::Deserialize;

/// Application settings loaded from config file and environment variables.
///
/// Priority order (highest first):
///   1. Environment variables (e.g. MATRIX_URL, API_KEY, PORT …)
///   2. Config file specified by CONFIG_FILE env var
///   3. Default `config.toml` in the working directory
///   4. Built-in defaults
#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    /// Listen address (default: "" → all interfaces)
    #[serde(default = "default_host")]
    pub host: String,

    /// Listen port (default: 4785)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Unix socket path (Linux/macOS only, overrides host/port when set)
    pub server_path: Option<String>,

    /// Matrix homeserver URL (default: https://matrix.org)
    #[serde(default = "default_matrix_url")]
    pub matrix_url: String,

    /// Bot Matrix user ID (e.g. @mybot:matrix.org)
    pub matrix_id: String,

    /// Bot password — used to obtain an access token on startup
    pub matrix_pw: Option<String>,

    /// Pre-existing access token (preferred over matrix_pw)
    pub matrix_token: Option<String>,

    /// Shared secret used to authenticate incoming webhook requests
    pub api_key: String,

    /// Directory containing custom Lua formatters (optional)
    pub formatters_dir: Option<String>,

    /// Log verbosity: 0=error 1=warn 2=info 3=debug 4=trace (default: 2)
    #[serde(default = "default_verbosity")]
    pub verbosity: u8,
}

fn default_host() -> String {
    String::new()
}
fn default_port() -> u16 {
    4785
}
fn default_matrix_url() -> String {
    "https://matrix.org".to_string()
}
fn default_verbosity() -> u8 {
    2
}

impl Settings {
    pub fn load() -> Result<Self> {
        let extra_file = std::env::var("CONFIG_FILE").unwrap_or_default();

        let cfg = Config::builder()
            // Built-in defaults
            .set_default("host", "")?
            .set_default("port", 4785_i64)?
            .set_default("matrix_url", "https://matrix.org")?
            .set_default("verbosity", 2_i64)?
            // Optional config files
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(&extra_file).required(false))
            // Environment variables override everything (MATRIX_URL → matrix_url, etc.)
            .add_source(Environment::default())
            .build()
            .context("Failed to build configuration")?;

        cfg.try_deserialize::<Settings>()
            .context("Failed to deserialize configuration")
    }
}
