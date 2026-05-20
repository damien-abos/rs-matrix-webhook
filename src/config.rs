use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, Environment, File};
use serde::Deserialize;

/// CLI arguments — highest priority source, override everything else.
#[derive(Parser, Debug)]
#[command(
    version,
    about = "Matrix webhook server with Lua-extensible formatters"
)]
struct Args {
    /// Listen address
    #[arg(long)]
    host: Option<String>,

    /// Listen port
    #[arg(long, short)]
    port: Option<u16>,

    /// Unix socket path (overrides --host / --port)
    #[arg(long)]
    server_path: Option<String>,

    /// Matrix homeserver URL
    #[arg(long)]
    matrix_url: Option<String>,

    /// Bot Matrix user ID (e.g. @bot:matrix.org)
    #[arg(long)]
    matrix_id: Option<String>,

    /// Bot password (used to obtain an access token)
    #[arg(long)]
    matrix_pw: Option<String>,

    /// Pre-existing Matrix access token (preferred over --matrix-pw)
    #[arg(long)]
    matrix_token: Option<String>,

    /// Shared secret for authenticating webhook requests
    #[arg(long)]
    api_key: Option<String>,

    /// Directory containing custom Lua formatters
    #[arg(long)]
    formatters_dir: Option<String>,

    /// Verbosity level: -v=warn -vv=info -vvv=debug -vvvv=trace -vvvvv=secrets
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to TOML config file (overrides CONFIG_FILE env var)
    #[arg(long, short = 'c')]
    config: Option<String>,
}

/// Application settings loaded from config file and environment variables.
///
/// Priority order (highest first):
///   1. CLI arguments
///   2. Environment variables (e.g. MATRIX_URL, API_KEY, PORT …)
///   3. Config file specified by --config / CONFIG_FILE env var
///   4. Default `config.toml` in the working directory
///   5. Built-in defaults
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
        let args = Args::parse();

        let config_file = args
            .config
            .clone()
            .unwrap_or_else(|| std::env::var("CONFIG_FILE").unwrap_or_default());

        let mut settings = Config::builder()
            .set_default("host", "")?
            .set_default("port", 4785_i64)?
            .set_default("matrix_url", "https://matrix.org")?
            .set_default("verbosity", 2_i64)?
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(&config_file).required(false))
            .add_source(Environment::default())
            .build()
            .context("Failed to build configuration")?
            .try_deserialize::<Settings>()
            .context("Failed to deserialize configuration")?;

        // CLI args take precedence over everything.
        if let Some(v) = args.host {
            settings.host = v;
        }
        if let Some(v) = args.port {
            settings.port = v;
        }
        if args.server_path.is_some() {
            settings.server_path = args.server_path;
        }
        if let Some(v) = args.matrix_url {
            settings.matrix_url = v;
        }
        if let Some(v) = args.matrix_id {
            settings.matrix_id = v;
        }
        if args.matrix_pw.is_some() {
            settings.matrix_pw = args.matrix_pw;
        }
        if args.matrix_token.is_some() {
            settings.matrix_token = args.matrix_token;
        }
        if let Some(v) = args.api_key {
            settings.api_key = v;
        }
        if args.formatters_dir.is_some() {
            settings.formatters_dir = args.formatters_dir;
        }
        if args.verbose > 0 {
            settings.verbosity = args.verbose;
        }

        Ok(settings)
    }
}
