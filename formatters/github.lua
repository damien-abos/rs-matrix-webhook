--- GitHub formatter — exact equivalent of the built-in Rust formatter.

function format(data, headers)
    local event = headers["x-github-event"] or ""

    if event == "push" then
        local pusher_name = (data.pusher and data.pusher.name) or "unknown"
        local ref     = data["ref"] or ""
        local after   = data.after   or ""
        local before  = data.before  or ""
        local compare = data.compare or ""

        local pusher_link = string.format(
            "[@%s](https://github.com/%s)", pusher_name, pusher_name)
        local body = string.format(
            "%s pushed on %s: [%s → %s](%s):\n\n",
            pusher_link, ref, before, after, compare)

        for _, commit in ipairs(data.commits or {}) do
            body = body .. string.format("- [%s](%s)\n",
                commit.message or "", commit.url or "")
        end
        data.body = body
    else
        data.body = "notification from github"
    end

    -- Forward the hub signature as digest for request authentication.
    local sig = headers["x-hub-signature-256"]
    if sig then
        data.digest = sig:gsub("^sha256=", "")
    end

    return data
end
