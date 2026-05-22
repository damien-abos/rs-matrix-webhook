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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert_eq!(to_html(""), "");
    }

    #[test]
    fn plain_text() {
        assert_eq!(to_html("hello"), "<p>hello</p>\n");
    }

    #[test]
    fn heading() {
        let html = to_html("## Title");
        assert!(html.contains("<h2>Title</h2>"));
    }

    #[test]
    fn bold() {
        let html = to_html("**bold**");
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn strikethrough_enabled() {
        let html = to_html("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn tables_enabled() {
        let md = "| a | b |\n|---|---|\n| 1 | 2 |";
        let html = to_html(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>a</th>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn link() {
        let html = to_html("[click](https://example.com)");
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(html.contains(">click</a>"));
    }
}
