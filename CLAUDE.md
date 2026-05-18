# rs-matrix-webhook — Claude Code context

## What this project does

HTTP webhook server that receives JSON payloads and posts formatted messages to Matrix rooms.
Rust rewrite of [nim65s/matrix-webhook](https://github.com/nim65s/matrix-webhook), adding
Lua-extensible formatters and configuration via both TOML file and environment variables.

## Build & run

```bash
cargo build               # debug
cargo build --release     # optimised
cargo check               # type-check only (fast)
```

The binary is `target/debug/matrix-webhook` or `target/release/matrix-webhook`.
Configuration is read from `config.toml` in the working directory (see `config.example.toml`).

## Project layout

```
src/
  main.rs              Entry point, AppState, HTTP route handlers
  config.rs            Settings struct — TOML + env var loading (config crate)
  matrix.rs            Matrix client-server API (reqwest, no matrix-sdk)
  auth.rs              API key check + HMAC-SHA256 verification
  markdown.rs          Markdown → HTML (pulldown-cmark)
  formatters/
    mod.rs             FormatterRegistry: loads built-ins then scans formatters_dir
    builtin.rs         Built-in Rust formatters
    lua.rs             Lua execution engine + JSON ↔ Lua value conversion
formatters/            Example/reference Lua formatters (one per built-in + identity)
config.example.toml    Annotated configuration template
```

## Key design decisions

- **No matrix-sdk** — Matrix client-server API called directly via `reqwest` to keep
  the dependency tree small. The only operations needed are login, join room, and send event.
- **Lua per call** — a fresh `mlua::Lua` state is created for each formatter invocation.
  This keeps formatters stateless and avoids `Send + Sync` issues on `mlua::Error`.
- **mlua::Result internally** — `json_to_lua` / `lua_to_json` return `mlua::Result` to avoid
  the `Send + Sync` constraint that `anyhow::Error` imposes; errors are converted to
  `anyhow::Error` only at the public `call_formatter` boundary.
- **Lua overrides built-ins** — when `formatters_dir` is set, a `.lua` file whose stem
  matches a built-in name replaces it. Built-ins are registered first; Lua files are
  inserted afterwards, overwriting the map entry.
- **TLS** — `reqwest` is built with `rustls` + `rustls-native-certs` so the system
  certificate store is used. This is required for self-hosted homeservers whose CA is
  not in the Mozilla bundle.
- **Config priority** — env vars > `CONFIG_FILE` path > `config.toml` > built-in defaults.
  Env var names match the original Python project (no prefix).

## Adding a formatter

**Rust built-in** — add a `pub fn name(data: Value, headers: &HashMap<String, String>) -> Result<Value>`
in `src/formatters/builtin.rs`, then register it in the array in `src/formatters/mod.rs`.

**Lua** — create `formatters/<name>.lua` exporting `function format(data, headers)`.
Set `formatters_dir = "./formatters"` in `config.toml`.

## Dependencies worth knowing

| Crate | Role |
|---|---|
| `axum` 0.8 | HTTP server |
| `reqwest` 0.13 | Matrix HTTP client |
| `mlua` 0.11 + lua54 vendored | Lua 5.4 scripting |
| `config` 0.15 | TOML + env var configuration |
| `pulldown-cmark` 0.13 | Markdown → HTML |
| `hmac` / `sha2` / `hex` | HMAC-SHA256 authentication |
