mod builtin;
pub mod lua;

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use crate::config::Settings;

// ---------------------------------------------------------------------------
// Formatter function type
// ---------------------------------------------------------------------------

type BuiltinFn = fn(Value, &HashMap<String, String>) -> Result<Value>;

enum FormatterImpl {
    Builtin(BuiltinFn),
    /// Lua script source code
    Lua(String),
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

pub struct FormatterRegistry {
    formatters: HashMap<String, FormatterImpl>,
}

impl FormatterRegistry {
    /// Builds the registry: registers built-ins first, then loads any `.lua`
    /// files found in `settings.formatters_dir` (Lua files override built-ins
    /// when they share the same name).
    pub fn new(settings: &Settings) -> Result<Self> {
        let mut formatters: HashMap<String, FormatterImpl> = HashMap::new();

        // Built-in Rust formatters
        for (name, f) in [
            ("github", builtin::github as BuiltinFn),
            ("grafana", builtin::grafana as BuiltinFn),
            ("gitlab_webhook", builtin::gitlab_webhook as BuiltinFn),
            ("gitlab_gchat", builtin::gitlab_gchat as BuiltinFn),
            ("gitlab_teams", builtin::gitlab_teams as BuiltinFn),
            ("grn", builtin::grn as BuiltinFn),
            ("identity", builtin::identity as BuiltinFn),
        ] {
            formatters.insert(name.to_string(), FormatterImpl::Builtin(f));
        }

        // Optional Lua formatters directory
        if let Some(ref dir) = settings.formatters_dir {
            let dir_path = Path::new(dir);
            if dir_path.is_dir() {
                for entry in std::fs::read_dir(dir_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().map(|e| e == "lua").unwrap_or(false) {
                        if let Some(name) =
                            path.file_stem().and_then(|s| s.to_str()).map(String::from)
                        {
                            let script = std::fs::read_to_string(&path)?;
                            tracing::info!("Loaded Lua formatter: {}", name);
                            formatters.insert(name, FormatterImpl::Lua(script));
                        }
                    }
                }
            } else {
                tracing::warn!("formatters_dir '{}' is not a directory", dir);
            }
        }

        Ok(Self { formatters })
    }

    /// Applies the named formatter to `data`.
    /// Returns `data` unchanged if the formatter name is unknown (with a warning).
    pub fn apply(
        &self,
        name: &str,
        data: Value,
        headers: &HashMap<String, String>,
    ) -> Result<Value> {
        match self.formatters.get(name) {
            Some(FormatterImpl::Builtin(f)) => f(data, headers),
            Some(FormatterImpl::Lua(script)) => lua::call_formatter(script, data, headers),
            None => {
                tracing::warn!(
                    "Unknown formatter '{}' — passing data through unchanged",
                    name
                );
                Ok(data)
            }
        }
    }

    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.formatters.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }
}
