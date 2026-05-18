--- GitLab → Google Chat formatter — exact equivalent of the built-in Rust formatter.
--- Converts Slack-style <url|label> links to Markdown [label](url).

function format(data, headers)
    if data.body then
        -- Pattern Lua : (.-) est non-greedy, | est littéral
        data.body = data.body:gsub("<(.-)|(.-)>", "[%2](%1)")
    end
    return data
end
