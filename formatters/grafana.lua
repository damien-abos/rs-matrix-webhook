--- Grafana formatter — exact equivalent of the built-in Rust formatter.
--- Supports Grafana v8 (ruleName/evalMatches) and v9+ (alerts/message).

local function grafana_v8(data)
    local text = ""
    if data.title then
        text = text .. "#### " .. data.title .. "\n"
    end
    if data.message then
        text = text .. data.message .. "\n\n"
    end
    for _, m in ipairs(data.evalMatches or {}) do
        local metric = m.metric or ""
        local value  = (m.value ~= nil) and tostring(m.value) or "null"
        text = text .. "* " .. metric .. ": " .. value .. "\n"
    end
    return text
end

local function grafana_9x(data)
    local text = ""
    if data.title then
        text = text .. "#### " .. data.title .. "\n"
    end
    if data.message then
        -- Each newline becomes a double newline (Markdown paragraph break).
        text = text .. data.message:gsub("\n", "\n\n") .. "\n\n"
    end
    return text
end

function format(data, headers)
    -- v9+ : pas de ruleName mais un tableau alerts
    if data.ruleName == nil and type(data.alerts) == "table" then
        data.body = grafana_9x(data)
    else
        data.body = grafana_v8(data)
    end
    return data
end
