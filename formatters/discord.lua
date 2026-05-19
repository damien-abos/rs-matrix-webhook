--- Discord formatter — renders a Discord webhook payload as Markdown.
--- Handles top-level username/content and the embeds array (author, title,
--- description, fields, footer).

function format(data, headers)
    local text = ""

    local has_username = data.username ~= nil
    local has_content  = data.content  ~= nil

    if has_username and has_content then
        text = text .. "**" .. data.username .. "**: " .. data.content .. "\n\n"
    elseif has_username then
        text = text .. "**" .. data.username .. "**\n\n"
    elseif has_content then
        text = text .. data.content .. "\n\n"
    end

    for _, embed in ipairs(data.embeds or {}) do
        -- Author
        if embed.author and embed.author.name then
            if embed.author.url then
                text = text .. "[" .. embed.author.name .. "](" .. embed.author.url .. ")\n"
            else
                text = text .. embed.author.name .. "\n"
            end
        end

        -- Title
        if embed.title then
            if embed.url then
                text = text .. "#### [" .. embed.title .. "](" .. embed.url .. ")\n\n"
            else
                text = text .. "#### " .. embed.title .. "\n\n"
            end
        end

        -- Description
        if embed.description then
            text = text .. embed.description .. "\n\n"
        end

        -- Fields
        local has_fields = embed.fields and #embed.fields > 0
        for _, field in ipairs(embed.fields or {}) do
            text = text .. "**" .. field.name .. "**: " .. field.value .. "\n"
        end
        if has_fields then
            text = text .. "\n"
        end

        -- Footer
        if embed.footer and embed.footer.text then
            text = text .. embed.footer.text .. "\n"
        end
    end

    data.body = text
    return data
end
