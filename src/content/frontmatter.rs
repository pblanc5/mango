use serde::Deserialize;

use crate::error::MangoError;

const FRONTMATTER_DELIMITER: &str = "---";

#[derive(Deserialize, Debug)]
pub struct MangoFrontmatter {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub draft: bool,
}

pub fn parse(content: String) -> Result<(Option<MangoFrontmatter>, String), MangoError> {
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
}
