# Test coverage

## Running the tests

```bash
cargo test --locked
```

## Coverage

Coverage is measured with [`cargo-llvm-cov`](https://github.com/taiki-e/cargo-llvm-cov).

```bash
# Install (once)
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview

# Run
cargo llvm-cov --locked
```

### Current results

| File | Lines | Functions | Notes |
|---|---|---|---|
| `auth.rs` | **100%** | **100%** | |
| `markdown.rs` | **100%** | **100%** | |
| `formatters/builtin.rs` | **94.6%** | **100%** | 18 lines missed — rare branches (e.g. discord content-only, grafana null values) |
| `formatters/lua.rs` | **80.9%** | 92.3% | Float-keyed and integer-keyed Lua tables not exercised; `lua_to_json` catch-all branch |
| `formatters/mod.rs` | **82.2%** | 80.0% | Lua formatter loading from disk not covered by unit tests |
| `config.rs` | 0% | 0% | CLI argument parsing — not unit-testable without process spawn |
| `main.rs` | 0% | 0% | HTTP route handlers — require integration tests |
| `matrix.rs` | 0% | 0% | Matrix client — requires a live homeserver |
| **Total** | **59.9%** | **64.0%** | |

The three 0% modules are expected: they all depend on external services (a Matrix homeserver,
a running HTTP server, or CLI argument parsing) that are out of scope for unit tests.
Coverage of the pure business logic (auth, markdown, all formatters) is 89–100%.

### CI

Coverage runs automatically on every push to `main` or a tag (not on pull requests).
Results are uploaded to [Codecov](https://codecov.io) when the `CODECOV_TOKEN` secret is set.

## Test suite breakdown

### `auth.rs` — 5 tests

| Test | What it checks |
|---|---|
| `correct_digest_accepted` | Valid HMAC-SHA256 digest passes |
| `wrong_digest_rejected` | Bogus hex string is rejected |
| `wrong_key_rejected` | Correct digest, wrong key → rejected |
| `tampered_body_rejected` | Digest computed on different body → rejected |
| `empty_body_accepted` | HMAC of an empty payload is accepted |

### `markdown.rs` — 7 tests

| Test | What it checks |
|---|---|
| `empty_input` | Empty string produces empty output |
| `plain_text` | Wrapped in `<p>` |
| `heading` | `## Title` → `<h2>` |
| `bold` | `**bold**` → `<strong>` |
| `strikethrough_enabled` | `~~text~~` → `<del>` (opt-in extension) |
| `tables_enabled` | Pipe tables → `<table>` (opt-in extension) |
| `link` | `[label](url)` → `<a href="...">` |

### `formatters/builtin.rs` — 15 tests

| Test | Formatter | What it checks |
|---|---|---|
| `identity_passes_through` | `identity` | Data returned unchanged |
| `github_push_formats_body` | `github` | Pusher link, ref, before/after, commit list |
| `github_non_push_gives_generic_body` | `github` | Non-push events produce a generic message |
| `github_hub_signature_sets_digest` | `github` | `x-hub-signature-256` header → `digest` field (strips `sha256=` prefix) |
| `grafana_v8_formats_title_and_metrics` | `grafana` | v8 payload dispatched by `ruleName` presence |
| `grafana_9x_dispatches_on_alerts_array` | `grafana` | v9+ payload dispatched by `alerts` array |
| `gitlab_webhook_formats_event_summary` | `gitlab_webhook` | Event name, project, author in body |
| `gitlab_webhook_token_sets_key` | `gitlab_webhook` | `x-gitlab-token` header → `key` field |
| `gitlab_gchat_converts_links` | `gitlab_gchat` | `<url\|label>` → `[label](url)` |
| `gitlab_gchat_no_body_is_noop` | `gitlab_gchat` | Missing `body` field leaves data unchanged |
| `gitlab_teams_text_sections_joined` | `gitlab_teams` | `text` sections split and prefixed with `* ` |
| `gitlab_teams_activity_sections` | `gitlab_teams` | `activityTitle/Subtitle/Text` fields merged |
| `discord_username_and_content` | `discord` | `**username**: content` in body |
| `discord_embed_with_title_and_description` | `discord` | Embed title link, description, fields |
| `grn_formats_release_announcement` | `grn` | Package, version, author, GitHub release URL |

### `formatters/lua.rs` — 6 tests

| Test | What it checks |
|---|---|
| `sets_body_field` | Formatter can write to `data.body` |
| `reads_data_fields` | Formatter can read arbitrary fields from `data` |
| `reads_headers` | Formatter can read HTTP headers |
| `returns_array_as_json_array` | Lua sequential table → JSON array |
| `missing_format_function_is_error` | Script without `format()` → error |
| `syntax_error_is_error` | Malformed Lua → error |

### `formatters/mod.rs` — 3 tests

| Test | What it checks |
|---|---|
| `names_are_sorted_and_contain_builtins` | Registry exposes all built-ins in alphabetical order |
| `apply_known_formatter` | `apply("identity", …)` returns data unchanged |
| `apply_unknown_formatter_returns_data_unchanged` | Unknown name → pass-through (no panic) |
