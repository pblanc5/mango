use pulldown_cmark::{Options, Parser};

pub fn to_html(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(content, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    // AC-4.1
    #[test]
    fn renders_tables() {
        let html = to_html("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(html.contains("<table>"), "{html}");
    }

    // AC-4.2
    #[test]
    fn renders_footnotes() {
        let html = to_html("text[^1]\n\n[^1]: note\n");
        assert!(
            html.contains(r#"<sup class="footnote-reference">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<div class="footnote-definition""#),
            "{html}"
        );
    }

    // AC-4.3
    #[test]
    fn renders_strikethrough() {
        let html = to_html("~~gone~~");
        assert!(html.contains("<del>gone</del>"), "{html}");
    }

    // AC-4.4
    #[test]
    fn renders_tasklists() {
        let html = to_html("- [ ] todo\n");
        let input = html
            .split("<input")
            .nth(1)
            .unwrap_or_else(|| panic!("no <input in: {html}"));
        let tag = input.split('>').next().unwrap();
        assert!(tag.contains(r#"type="checkbox""#), "{html}");
    }

    // AC-4.5
    #[test]
    fn renders_heading_attributes() {
        let html = to_html("# Title {#my-id}\n");
        assert!(html.contains(r#"<h1 id="my-id">"#), "{html}");
    }
}
