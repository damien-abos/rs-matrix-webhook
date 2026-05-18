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

        let pusher_link = format!(
            "[@{name}](https://github.com/{name})",
            name = pusher_name
        );
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
                let items: Vec<String> = text
                    .split("\n\n")
                    .map(|t| format!("* {}", t))
                    .collect();
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
