# rs-matrix-webhook

A webhook receiver that forwards notifications to [Matrix](https://matrix.org) rooms,
written in Rust. Spiritual rewrite of [nim65s/matrix-webhook](https://github.com/nim65s/matrix-webhook)
with Lua-extensible formatters.

## Features

- **HTTP webhook endpoint** — accepts JSON payloads, posts formatted messages to any Matrix room
- **Authentication** — API key (body/query) or HMAC-SHA256 digest
- **Built-in formatters** — GitHub, Grafana (v8 + v9+), GitLab (webhook / Google Chat / Teams), GRN, identity
- **Lua formatters** — drop a `.lua` file in a directory; it overrides or extends the built-ins at runtime without recompiling
- **Markdown → HTML** — message bodies are rendered with CommonMark + tables + strikethrough
- **Configuration** — TOML file and/or environment variables (compatible with the original project's env var names)
- **Unix socket** support (Linux/macOS)

## Quick start

```bash
# Copy and edit the configuration
cp config.example.toml config.toml
$EDITOR config.toml          # set matrix_id, matrix_token, api_key

# Run
cargo run --release
```

Send a message:
```bash
curl -X POST http://localhost:4785/!yourroom:matrix.example.com \
  -H 'Content-Type: application/json' \
  -d '{"key":"your-api-key","body":"Hello from webhook!"}'
```

## Configuration

All options can be set in `config.toml` **or** as environment variables (same name, upper-case).
Environment variables take precedence over the file.

A different config file path can be specified with `CONFIG_FILE=/path/to/file`.

| Key | Env var | Default | Description |
|---|---|---|---|
| `host` | `HOST` | `""` (all interfaces) | Listen address |
| `port` | `PORT` | `4785` | TCP port |
| `server_path` | `SERVER_PATH` | — | Unix socket path (overrides host/port) |
| `matrix_url` | `MATRIX_URL` | `https://matrix.org` | Homeserver URL |
| `matrix_id` | `MATRIX_ID` | **required** | Bot Matrix user ID |
| `matrix_token` | `MATRIX_TOKEN` | — | Access token (preferred) |
| `matrix_pw` | `MATRIX_PW` | — | Password (used to obtain a token on startup) |
| `api_key` | `API_KEY` | **required** | Shared webhook secret |
| `formatters_dir` | `FORMATTERS_DIR` | — | Directory of custom Lua formatters |
| `verbosity` | `VERBOSITY` | `2` | Log level: 0=error 1=warn 2=info 3=debug 4=trace |

## HTTP API

### `GET /health`

Returns `200 OK`. Use for liveness probes.

### `POST /{room_id}` · `POST /`

Posts a message to the given Matrix room.

`room_id` can be provided:
1. In the URL path: `POST /!abc:example.com`
2. As a query parameter: `POST /?room_id=!abc:example.com`
3. In the JSON body: `{"room_id": "!abc:example.com", ...}`

**Authentication** — one of:

| Method | Field | Where |
|---|---|---|
| API key | `key` | JSON body or query string |
| HMAC-SHA256 | `digest` | JSON body — `hex(hmac_sha256(raw_body, api_key))` |

**Optional fields:**

| Field | Description |
|---|---|
| `body` | Message text (Markdown). Required unless a formatter produces it. |
| `msgtype` | Matrix message type (default: `m.text`) |
| `formatter` | Name of the formatter to apply (query string or body) |

The legacy `text` field is accepted as an alias for `body`.

## Built-in formatters

| Name | Source | Description |
|---|---|---|
| `github` | GitHub webhooks | Push, issues, pull requests |
| `grafana` | Grafana alerts | v8 (evalMatches) and v9+ (unified alerting) |
| `gitlab_webhook` | GitLab webhooks | Generic event summary |
| `gitlab_gchat` | GitLab → Google Chat | Converts `<url\|label>` links to Markdown |
| `gitlab_teams` | GitLab → MS Teams | Parses `sections` payload into Markdown |
| `grn` | GitHub Release Notifier | Formatted release announcement |
| `identity` | — | Pass-through, returns data unchanged |

## Custom Lua formatters

Set `formatters_dir` to a directory containing `.lua` files.
Each file must export a `format(data, headers)` function that returns the modified `data` table.
The `body` field must be set before returning.

A Lua formatter with the same name as a built-in **overrides** it.

```lua
-- formatters/my_service.lua
function format(data, headers)
    data.body = string.format("**Alert**: %s", data.message or "no message")
    return data
end
```

Invoke it with:
```bash
curl -X POST "http://localhost:4785/!room:example.com?formatter=my_service" \
  -H 'Content-Type: application/json' \
  -d '{"key":"secret","message":"disk full"}'
```

The `formatters/` directory in this repository contains Lua equivalents of every built-in formatter,
ready to use as starting points.

## Docker

```bash
# Single platform (current machine)
docker build -t rs-matrix-webhook .

# Multi-platform (amd64 + arm64 — requires docker buildx)
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t rs-matrix-webhook:latest \
  --push .

docker run -d \
  -e MATRIX_ID=@bot:example.com \
  -e MATRIX_TOKEN=syt_... \
  -e API_KEY=secret \
  -p 4785:4785 \
  rs-matrix-webhook
```

The build stage always runs natively on the builder's architecture — Zig handles
C cross-compilation (vendored Lua) and linking for both targets without QEMU.
The final image is fully static (musl) on top of `gcr.io/distroless/static-debian12`.

## Building from source

Requires **Rust 1.85+** (for edition 2024 transitive dependencies).

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Debug build
cargo build

# Optimised release build
cargo build --release
# Binary: target/release/matrix-webhook
```

## License

MIT — see [LICENSE](LICENSE).
