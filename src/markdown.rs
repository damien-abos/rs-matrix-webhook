use pulldown_cmark::{Options, Parser, html};

/// Converts a Markdown string to an HTML string.
///
/// Enables strikethrough and tables in addition to the CommonMark baseline,
/// matching the feature set used by the original Python matrix-webhook.
pub fn to_html(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(markdown, opts);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}
