use serde::Deserialize;

use crate::error::MangoError;

const FRONTMATTER_DELIMITER: &str = "---";
const BYTE_ORDER_MARK: char = '\u{feff}';

#[derive(Deserialize, Debug)]
pub struct MangoFrontmatter {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub draft: bool,
}

/// Splits a leading `---` JSON `---` block from the markdown body. A UTF-8
/// byte order mark (added by some Windows editors) is ignored.
pub fn parse(content: String) -> Result<(Option<MangoFrontmatter>, String), MangoError> {
    let content = match content.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => rest.to_owned(),
        None => content,
    };
    let mut lines = content.lines();

    if lines.next().map(|l| l.trim()) != Some(FRONTMATTER_DELIMITER) {
        return Ok((None, content.to_string()));
    }

    let mut json_lines = Vec::new();

    for line in lines.by_ref() {
        if line.trim() == FRONTMATTER_DELIMITER {
            let json = json_lines.join("\n");
            let body = lines.collect::<Vec<_>>().join("\n");
            let fm = serde_json::from_str::<MangoFrontmatter>(&json)
                .map_err(|e| MangoError::Frontmatter(e.to_string()))?;

            return Ok((Some(fm), body));
        }

        json_lines.push(line);
    }

    Err(MangoError::Frontmatter(
        "unterminated frontmatter block".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JSON: &str = r#"{
        "title": "Post One",
        "author": "tester",
        "description": "my first post",
        "date": "2026-01-24",
        "tags": ["blog"],
        "draft": false
    }"#;

    #[test]
    fn parses_valid_frontmatter_and_returns_body() {
        let content = format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld");
        let (fm, body) = parse(content).unwrap();

        let fm = fm.expect("frontmatter should be present");
        assert_eq!(fm.title, "Post One");
        assert_eq!(fm.author, "tester");
        assert_eq!(fm.date.as_deref(), Some("2026-01-24"));
        assert_eq!(fm.tags, Some(vec!["blog".to_string()]));
        assert!(!fm.draft);
        assert_eq!(body, "# Hello\n\nworld");
    }

    #[test]
    fn byte_order_mark_is_ignored() {
        let content = format!("\u{feff}---\n{VALID_JSON}\n---\nbody");
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter behind a BOM").title, "Post One");
        assert_eq!(body, "body");
    }

    #[test]
    fn optional_fields_may_be_omitted() {
        let json = r#"{"title": "t", "author": "a", "description": "d", "draft": true}"#;
        let content = format!("---\n{json}\n---\nbody");
        let (fm, _) = parse(content).unwrap();

        let fm = fm.unwrap();
        assert_eq!(fm.date, None);
        assert_eq!(fm.tags, None);
    }

    #[test]
    fn returns_none_when_no_frontmatter() {
        let content = "# Just markdown\n\nno frontmatter here".to_string();
        let (fm, body) = parse(content.clone()).unwrap();

        assert!(fm.is_none());
        assert_eq!(body, content);
    }

    #[test]
    fn malformed_json_is_an_error() {
        let content = "---\n{not valid json\n---\nbody".to_string();
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    #[test]
    fn unterminated_block_is_an_error() {
        let content = "---\n{\"title\": \"t\"}\nno closing delimiter".to_string();
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    // The CRLF tests below are characterization tests: they pin what `parse`
    // already does with Windows line endings so a regression is caught. Inputs
    // are built from the LF source text by `crlf`, never written with literal
    // `\r\n` in a multi-line literal, so they hold however this file is checked
    // out (`core.autocrlf`, `.gitattributes`).

    /// Rewrites every line ending to CRLF, whatever it started as.
    fn crlf(text: &str) -> String {
        lf(text).replace('\n', "\r\n")
    }

    /// Rewrites every line ending to LF, whatever it started as.
    fn lf(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    // AC-12.1
    #[test]
    fn parses_crlf_frontmatter_and_normalizes_body() {
        let content = crlf(&format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n"));
        let (fm, body) = parse(content).unwrap();

        let fm = fm.expect("frontmatter should be present");
        assert_eq!(fm.title, "Post One");
        assert_eq!(fm.author, "tester");
        assert_eq!(fm.description, "my first post");
        assert_eq!(fm.date.as_deref(), Some("2026-01-24"));
        assert_eq!(fm.tags, Some(vec!["blog".to_string()]));
        assert!(!fm.draft);
        for field in [&fm.title, &fm.author, &fm.description] {
            assert!(!field.contains('\r'), "carriage return in {field:?}");
        }
        // The body is normalized to LF and its final newline is dropped,
        // exactly as for the LF input in `parses_valid_frontmatter_and_returns_body`.
        assert_eq!(body, "# Hello\n\nworld");
        assert!(!body.contains('\r'), "carriage return in body: {body:?}");
    }

    // AC-12.2
    #[test]
    fn crlf_and_lf_documents_parse_identically() {
        // Both inputs come from the same source text, so they cannot drift apart.
        let source = format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n");
        let (lf_fm, lf_body) = parse(lf(&source)).unwrap();
        let (crlf_fm, crlf_body) = parse(crlf(&source)).unwrap();

        let lf_fm = lf_fm.expect("LF frontmatter");
        let crlf_fm = crlf_fm.expect("CRLF frontmatter");
        assert_eq!(crlf_body, lf_body);
        // Equality alone is only a symmetry check: a change that gave *both*
        // inputs CRLF bodies would keep it green. Pin the shared value too, so
        // this test fails on the direction of the normalization, not just its
        // consistency.
        assert_eq!(crlf_body, "# Hello\n\nworld");
        assert!(!crlf_body.contains('\r'), "carriage return in body");
        assert_eq!(crlf_fm.title, lf_fm.title);
        assert_eq!(crlf_fm.author, lf_fm.author);
        assert_eq!(crlf_fm.description, lf_fm.description);
        assert_eq!(crlf_fm.date, lf_fm.date);
        assert_eq!(crlf_fm.tags, lf_fm.tags);
        assert_eq!(crlf_fm.draft, lf_fm.draft);
    }

    // AC-12.3
    #[test]
    fn byte_order_mark_with_crlf_is_ignored() {
        // The BOM is stripped before the content is split into lines, so the
        // two quirks compose.
        let content = format!(
            "\u{feff}{}",
            crlf(&format!("---\n{VALID_JSON}\n---\n# Hello\n\nworld\n"))
        );
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter behind a BOM").title, "Post One");
        assert_eq!(body, "# Hello\n\nworld");
    }

    // AC-12.4
    #[test]
    fn parses_mixed_lf_and_crlf_line_endings() {
        // Written with explicit escapes: the endings differ line by line.
        let content = concat!(
            "---\r\n",
            "{\"title\": \"t\", \"author\": \"a\",\r\n",
            " \"description\": \"d\", \"draft\": false}\n",
            "---\n",
            "# Hello\r\n",
            "\n",
            "world\r\n",
        )
        .to_string();
        let (fm, body) = parse(content).unwrap();

        assert_eq!(fm.expect("frontmatter should be present").title, "t");
        assert_eq!(body, "# Hello\n\nworld");
        assert!(!body.contains('\r'), "carriage return in body: {body:?}");
    }

    // AC-12.5
    #[test]
    fn unterminated_crlf_block_is_an_error() {
        let content = crlf("---\n{\"title\": \"t\"}\nno closing delimiter\n");
        assert!(matches!(parse(content), Err(MangoError::Frontmatter(_))));
    }

    // AC-12.6
    #[test]
    fn crlf_body_is_returned_verbatim_when_no_frontmatter() {
        // The "no frontmatter" early return hands back the input unchanged, so
        // this is the one path that does not normalize CRLF to LF. It cannot be
        // reached from the CLI today (`loader::load` turns `None` into the
        // `missing frontmatter` build error); the assertion records current
        // behavior rather than a guarantee. ARCH-7 proposes replacing this
        // return with an error, which would retire this test.
        let content = crlf("# Just markdown\n\nno frontmatter here\n");
        let (fm, body) = parse(content.clone()).unwrap();

        assert!(fm.is_none());
        assert_eq!(body, content);
        assert!(body.contains("\r\n"), "CRLF should survive: {body:?}");
    }
}
