--- GitLab webhook formatter — exact equivalent of the built-in Rust formatter.

function format(data, headers)
    local event_name   = data.event_name or "unknown"
    local user_name    = data.user_name  or "unknown"
    local project      = data.project   or {}
    local project_name = project.name     or "unknown"
    local project_url  = project.web_url  or ""

    data.body = string.format(
        "New %s event on [%s](%s) by %s.",
        event_name, project_name, project_url, user_name)

    -- Use the GitLab token as the webhook authentication key.
    local token = headers["x-gitlab-token"]
    if token then
        data.key = token
    end

    return data
end
