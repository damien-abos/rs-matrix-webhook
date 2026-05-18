--- GitLab → Microsoft Teams formatter — exact equivalent of the built-in Rust formatter.
--- Parses the Teams payload `sections` into Markdown.

--- Splits a string on a multi-character separator.
local function split(str, sep)
    local parts = {}
    local pattern = "(.-)" .. sep
    local last = 1
    for part, pos in (str .. sep):gmatch("(.-)" .. sep .. "()") do
        table.insert(parts, part)
        last = pos
    end
    return parts
end

function format(data, headers)
    local body_parts = {}

    for _, section in ipairs(data.sections or {}) do
        if section.text ~= nil then
            -- Split on \n\n and prefix each paragraph with "* ".
            local items = {}
            for _, part in ipairs(split(section.text, "\n\n")) do
                table.insert(items, "* " .. part)
            end
            table.insert(body_parts, "\n" .. table.concat(items, "  \n"))
        elseif section.activityTitle and section.activitySubtitle and section.activityText then
            table.insert(body_parts,
                section.activityTitle .. " " .. section.activitySubtitle
                .. " → " .. section.activityText)
        end
    end

    data.body = table.concat(body_parts, "  \n")
    return data
end
