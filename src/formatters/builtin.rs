use std::collections::HashMap;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;

// ---------------------------------------------------------------------------
// GitHub
// ---------------------------------------------------------------------------

pub fn github(mut data: Value, headers: &HashMap<String, String>) -> Result<Value> {
    let event = headers
        .get("x-github-event")
        .map(|s| s.as_str())
        .unwrap_or("");

    if event == "push" {
        let pusher_name = data["pusher"]["name"].as_str().unwrap_or("unknown");
        let refname = data["ref"].as_str().unwrap_or("");
        let after = data["after"].as_str().unwrap_or("");
        let before = data["before"].as_str().unwrap_or("");
        let compare = data["compare"].as_str().unwrap_or("");

        let pusher_link = format!("[@{name}](https://github.com/{name})", name = pusher_name);
        let mut body = format!(
            "{} pushed on {}: [{} → {}]({}):\n\n",
            pusher_link, refname, before, after, compare
        );

        if let Some(commits) = data["commits"].as_array() {
            for commit in commits {
                let msg = commit["message"].as_str().unwrap_or("");
                let url = commit["url"].as_str().unwrap_or("");
                body.push_str(&format!("- [{}]({})\n", msg, url));
            }
        }
        data["body"] = Value::String(body);
    } else {
        data["body"] = Value::String("notification from github".to_string());
    }

    // Authenticate subsequent requests using the hub signature.
    if let Some(sig) = headers.get("x-hub-signature-256") {
        let digest = sig.trim_start_matches("sha256=");
        data["digest"] = Value::String(digest.to_string());
    }

    Ok(data)
}

// ---------------------------------------------------------------------------
// Grafana
// ---------------------------------------------------------------------------

pub fn grafana(mut data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    // Dispatch on payload shape: v9+ has `alerts` and no `ruleName`.
    let body = if data["ruleName"].is_null() && data["alerts"].is_array() {
        grafana_9x(&data)
    } else {
        grafana_v8(&data)
    };

    data["body"] = Value::String(body);
    Ok(data)
}

fn grafana_v8(data: &Value) -> String {
    let mut text = String::new();
    if let Some(title) = data["title"].as_str() {
        text.push_str(&format!("#### {}\n", title));
    }
    if let Some(msg) = data["message"].as_str() {
        text.push_str(msg);
        text.push_str("\n\n");
    }
    if let Some(matches) = data["evalMatches"].as_array() {
        for m in matches {
            let metric = m["metric"].as_str().unwrap_or("");
            // value can be a number or string; render as-is
            let value = match &m["value"] {
                Value::Null => "null".to_string(),
                other => other.to_string(),
            };
            text.push_str(&format!("* {}: {}\n", metric, value));
        }
    }
    text
}

fn grafana_9x(data: &Value) -> String {
    let mut text = String::new();
    if let Some(title) = data["title"].as_str() {
        text.push_str(&format!("#### {}\n", title));
    }
    if let Some(msg) = data["message"].as_str() {
        text.push_str(&msg.replace('\n', "\n\n"));
        text.push_str("\n\n");
    }
    text
}

// ---------------------------------------------------------------------------
// GitLab webhook
// ---------------------------------------------------------------------------

pub fn gitlab_webhook(mut data: Value, headers: &HashMap<String, String>) -> Result<Value> {
    let event_name = data["event_name"].as_str().unwrap_or("unknown");
    let user_name = data["user_name"].as_str().unwrap_or("unknown");
    let project_name = data["project"]["name"].as_str().unwrap_or("unknown");
    let project_url = data["project"]["web_url"].as_str().unwrap_or("");

    data["body"] = Value::String(format!(
        "New {} event on [{}]({}) by {}.",
        event_name, project_name, project_url, user_name
    ));

    // Use the GitLab token as the webhook key when present.
    if let Some(token) = headers.get("x-gitlab-token") {
        data["key"] = Value::String(token.clone());
    }

    Ok(data)
}

// ---------------------------------------------------------------------------
// GitLab → Google Chat  (<url|text> links → Markdown)
// ---------------------------------------------------------------------------

pub fn gitlab_gchat(mut data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    if let Some(body) = data["body"].as_str() {
        // Convert Slack-style <url|label> links to Markdown [label](url).
        let re = Regex::new(r"<(.*?)\|(.*?)>").unwrap();
        let converted = re.replace_all(body, "[$2]($1)").into_owned();
        data["body"] = Value::String(converted);
    }
    Ok(data)
}

// ---------------------------------------------------------------------------
// GitLab → Microsoft Teams  (parse `sections` into Markdown)
// ---------------------------------------------------------------------------

pub fn gitlab_teams(mut data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    let mut body_parts: Vec<String> = Vec::new();

    if let Some(sections) = data["sections"].as_array() {
        for section in sections {
            if let Some(text) = section["text"].as_str() {
                // Split on double newlines, prefix each paragraph with "* ".
                let items: Vec<String> = text.split("\n\n").map(|t| format!("* {}", t)).collect();
                body_parts.push(format!("\n{}", items.join("  \n")));
            } else if let (Some(title), Some(subtitle), Some(activity_text)) = (
                section["activityTitle"].as_str(),
                section["activitySubtitle"].as_str(),
                section["activityText"].as_str(),
            ) {
                body_parts.push(format!("{} {} → {}", title, subtitle, activity_text));
            }
        }
    }

    data["body"] = Value::String(body_parts.join("  \n"));
    Ok(data)
}

// ---------------------------------------------------------------------------
// Discord
// ---------------------------------------------------------------------------

pub fn discord(mut data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    let mut text = String::new();

    let has_username = data["username"].is_string();
    let has_content = data["content"].is_string();

    if has_username && has_content {
        text.push_str(&format!(
            "**{}**: {}\n\n",
            data["username"].as_str().unwrap_or(""),
            data["content"].as_str().unwrap_or(""),
        ));
    } else if has_username {
        text.push_str(&format!(
            "**{}**\n\n",
            data["username"].as_str().unwrap_or("")
        ));
    } else if has_content {
        text.push_str(&format!("{}\n\n", data["content"].as_str().unwrap_or("")));
    }

    if let Some(embeds) = data["embeds"].as_array().map(|a| a.to_owned()) {
        for embed in &embeds {
            if let Some(name) = embed["author"]["name"].as_str() {
                if let Some(url) = embed["author"]["url"].as_str() {
                    text.push_str(&format!("[{}]({})\n", name, url));
                } else {
                    text.push_str(&format!("{}\n", name));
                }
            }

            if let Some(title) = embed["title"].as_str() {
                if let Some(url) = embed["url"].as_str() {
                    text.push_str(&format!("#### [{}]({})\n\n", title, url));
                } else {
                    text.push_str(&format!("#### {}\n\n", title));
                }
            }

            if let Some(desc) = embed["description"].as_str() {
                text.push_str(&format!("{}\n\n", desc));
            }

            if let Some(fields) = embed["fields"].as_array() {
                for field in fields {
                    let name = field["name"].as_str().unwrap_or("");
                    let value = field["value"].as_str().unwrap_or("");
                    text.push_str(&format!("**{}**: {}\n", name, value));
                }
                if !fields.is_empty() {
                    text.push('\n');
                }
            }

            if let Some(footer) = embed["footer"]["text"].as_str() {
                text.push_str(&format!("{}\n", footer));
            }
        }
    }

    data["body"] = Value::String(text);
    Ok(data)
}

// ---------------------------------------------------------------------------
// Identity (pass-through)
// ---------------------------------------------------------------------------

pub fn identity(data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    Ok(data)
}

// ---------------------------------------------------------------------------
// GitHub Release Notifier (grn)
// ---------------------------------------------------------------------------

pub fn grn(mut data: Value, _headers: &HashMap<String, String>) -> Result<Value> {
    let version = data["version"].as_str().unwrap_or("");
    let title = data["title"].as_str().unwrap_or("");
    let author = data["author"].as_str().unwrap_or("");
    let package = data["package_name"].as_str().unwrap_or("");

    data["body"] = Value::String(format!(
        "### {package} - {version}\n\n{title}\n\n\
         [{author} released new version **{version}** for **{package}**]\
         (https://github.com/{package}/releases/tag/{version}).\n\n",
    ));

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn no_headers() -> HashMap<String, String> {
        HashMap::new()
    }

    // ── identity ──────────────────────────────────────────────────────────────

    #[test]
    fn identity_passes_through() {
        let data = json!({"body": "hello", "extra": 42});
        let result = identity(data.clone(), &no_headers()).unwrap();
        assert_eq!(result, data);
    }

    // ── github ────────────────────────────────────────────────────────────────

    #[test]
    fn github_push_formats_body() {
        let data = json!({
            "pusher": {"name": "alice"},
            "ref": "refs/heads/main",
            "before": "aaa",
            "after": "bbb",
            "compare": "https://github.com/org/repo/compare/aaa...bbb",
            "commits": [
                {"message": "fix bug", "url": "https://github.com/org/repo/commit/bbb"}
            ]
        });
        let mut headers = HashMap::new();
        headers.insert("x-github-event".to_string(), "push".to_string());

        let result = github(data, &headers).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("[@alice](https://github.com/alice)"));
        assert!(body.contains("refs/heads/main"));
        assert!(body.contains("aaa"));
        assert!(body.contains("bbb"));
        assert!(body.contains("[fix bug](https://github.com/org/repo/commit/bbb)"));
    }

    #[test]
    fn github_non_push_gives_generic_body() {
        let data = json!({});
        let mut headers = HashMap::new();
        headers.insert("x-github-event".to_string(), "ping".to_string());

        let result = github(data, &headers).unwrap();
        assert_eq!(result["body"].as_str().unwrap(), "notification from github");
    }

    #[test]
    fn github_hub_signature_sets_digest() {
        let data = json!({});
        let mut headers = HashMap::new();
        headers.insert("x-github-event".to_string(), "push".to_string());
        headers.insert(
            "x-hub-signature-256".to_string(),
            "sha256=abcdef1234".to_string(),
        );

        let result = github(data, &headers).unwrap();
        assert_eq!(result["digest"].as_str().unwrap(), "abcdef1234");
    }

    // ── grafana ───────────────────────────────────────────────────────────────

    #[test]
    fn grafana_v8_formats_title_and_metrics() {
        let data = json!({
            "title": "Disk Alert",
            "message": "Disk is full",
            "ruleName": "disk-rule",
            "evalMatches": [
                {"metric": "disk_used", "value": 95}
            ]
        });
        let result = grafana(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("Disk Alert"));
        assert!(body.contains("Disk is full"));
        assert!(body.contains("disk_used"));
        assert!(body.contains("95"));
    }

    #[test]
    fn grafana_9x_dispatches_on_alerts_array() {
        let data = json!({
            "title": "CPU Alert",
            "message": "CPU high",
            "alerts": [{"status": "firing"}]
        });
        let result = grafana(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("CPU Alert"));
        assert!(body.contains("CPU high"));
    }

    // ── gitlab_webhook ────────────────────────────────────────────────────────

    #[test]
    fn gitlab_webhook_formats_event_summary() {
        let data = json!({
            "event_name": "push",
            "user_name": "bob",
            "project": {"name": "myrepo", "web_url": "https://gitlab.com/org/myrepo"}
        });
        let result = gitlab_webhook(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("push"));
        assert!(body.contains("myrepo"));
        assert!(body.contains("bob"));
    }

    #[test]
    fn gitlab_webhook_token_sets_key() {
        let data = json!({"event_name": "push", "user_name": "x", "project": {}});
        let mut headers = HashMap::new();
        headers.insert("x-gitlab-token".to_string(), "tok123".to_string());

        let result = gitlab_webhook(data, &headers).unwrap();
        assert_eq!(result["key"].as_str().unwrap(), "tok123");
    }

    // ── gitlab_gchat ──────────────────────────────────────────────────────────

    #[test]
    fn gitlab_gchat_converts_links() {
        let data = json!({"body": "see <https://example.com|here> for details"});
        let result = gitlab_gchat(data, &no_headers()).unwrap();
        assert_eq!(
            result["body"].as_str().unwrap(),
            "see [here](https://example.com) for details"
        );
    }

    #[test]
    fn gitlab_gchat_no_body_is_noop() {
        let data = json!({"other": "field"});
        let result = gitlab_gchat(data.clone(), &no_headers()).unwrap();
        assert_eq!(result, data);
    }

    // ── gitlab_teams ──────────────────────────────────────────────────────────

    #[test]
    fn gitlab_teams_text_sections_joined() {
        let data = json!({
            "sections": [
                {"text": "first paragraph\n\nsecond paragraph"},
                {"text": "another section"}
            ]
        });
        let result = gitlab_teams(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("* first paragraph"));
        assert!(body.contains("* another section"));
    }

    #[test]
    fn gitlab_teams_activity_sections() {
        let data = json!({
            "sections": [{
                "activityTitle": "Alice",
                "activitySubtitle": "pushed",
                "activityText": "3 commits"
            }]
        });
        let result = gitlab_teams(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("Alice"));
        assert!(body.contains("pushed"));
        assert!(body.contains("3 commits"));
    }

    // ── discord ───────────────────────────────────────────────────────────────

    #[test]
    fn discord_username_and_content() {
        let data = json!({"username": "bot", "content": "hello world"});
        let result = discord(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("**bot**"));
        assert!(body.contains("hello world"));
    }

    #[test]
    fn discord_embed_with_title_and_description() {
        let data = json!({
            "embeds": [{
                "title": "Alert",
                "url": "https://example.com",
                "description": "Something happened",
                "fields": [{"name": "severity", "value": "high"}]
            }]
        });
        let result = discord(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("[Alert](https://example.com)"));
        assert!(body.contains("Something happened"));
        assert!(body.contains("**severity**: high"));
    }

    // ── grn ───────────────────────────────────────────────────────────────────

    #[test]
    fn grn_formats_release_announcement() {
        let data = json!({
            "version": "v1.2.3",
            "title": "Bug fixes",
            "author": "alice",
            "package_name": "org/myapp"
        });
        let result = grn(data, &no_headers()).unwrap();
        let body = result["body"].as_str().unwrap();
        assert!(body.contains("### org/myapp - v1.2.3"));
        assert!(body.contains("Bug fixes"));
        assert!(body.contains("alice"));
        assert!(body.contains("https://github.com/org/myapp/releases/tag/v1.2.3"));
    }
}
