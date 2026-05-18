--- GitHub Release Notifier (grn) formatter — exact equivalent of the built-in Rust formatter.

function format(data, headers)
    local version = data.version      or ""
    local title   = data.title        or ""
    local author  = data.author       or ""
    local package = data.package_name or ""

    data.body = string.format(
        "### %s - %s\n\n%s\n\n[%s released new version **%s** for **%s**]"
        .. "(https://github.com/%s/releases/tag/%s).\n\n",
        package, version,
        title,
        author, version, package,
        package, version)

    return data
end
