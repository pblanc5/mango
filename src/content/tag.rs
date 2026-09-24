use std::fmt;

use serde::Serialize;

use crate::{content::slug::Slug, error::MangoError};

/// A frontmatter tag that matches `^[a-z0-9]+(-[a-z0-9]+)*$`. A value only
/// exists after that check (`parse`), so a tag is its own URL segment: its
/// page is at `tags/<tag>`. Ordering is byte-wise on the text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Tag(String);

impl Tag {
    pub fn parse(value: String) -> Result<Tag, MangoError> {
        let valid = !value.is_empty()
            && value.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            });

        if !valid {
            return Err(MangoError::Frontmatter(format!(
                "invalid tag '{value}': expected lowercase ASCII letters and digits separated by single hyphens"
            )));
        }

        Ok(Tag(value))
    }

    /// Parses every value in order, failing on the first invalid one, and
    /// drops duplicates, keeping the first occurrence's position.
    pub fn parse_list(values: Vec<String>) -> Result<Vec<Tag>, MangoError> {
        let mut tags: Vec<Tag> = Vec::with_capacity(values.len());

        for value in values {
            let tag = Tag::parse(value)?;
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }

        Ok(tags)
    }

    /// The tag page's slug: `tags/<tag>`.
    pub fn slug(&self) -> Slug {
        Slug::tag_page(self)
    }

    /// The tag page's URL: `/tags/<tag>/`.
    pub fn url(&self) -> String {
        self.slug().url()
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(tags: &[Tag]) -> Vec<String> {
        tags.iter().map(Tag::to_string).collect()
    }

    fn tag(value: &str) -> Tag {
        Tag::parse(value.to_string()).unwrap()
    }

    // AC-1.1 (batch 4)
    #[test]
    fn validate_tags_accepts_valid_values() {
        let good = ["blog", "static-site", "a1", "2026", "a-b-c"];
        let tags: Vec<String> = good.iter().map(|t| t.to_string()).collect();
        assert_eq!(names(&Tag::parse_list(tags).unwrap()), good);
    }

    // AC-1.2 (batch 4); AC-arch-3.6.2, AC-arch-3.7.4
    #[test]
    fn validate_tags_rejects_invalid_values_naming_value() {
        let bad = [
            "",
            "Rust",
            "static site",
            "c++",
            "-rust",
            "rust-",
            "a--b",
            "café",
            "ünï",
        ];
        for value in bad {
            let err = Tag::parse_list(vec!["ok".into(), value.into()]).expect_err(value);
            assert!(
                matches!(err, MangoError::Frontmatter(_)),
                "{value}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains(&format!("'{value}'")), "{value}: {msg}");
            assert!(
                msg.contains(&format!(
                    "invalid tag '{value}': expected lowercase ASCII letters and digits separated by single hyphens"
                )),
                "{value}: {msg}"
            );

            let err = Tag::parse(value.to_string()).expect_err(value);
            assert_eq!(
                err.to_string(),
                format!(
                    "Mango Frontmatter Error: invalid tag '{value}': expected lowercase ASCII letters and digits separated by single hyphens"
                )
            );
        }
    }

    // AC-1.3 (batch 4)
    #[test]
    fn validate_tags_removes_duplicates_keeping_order() {
        let tags = ["b", "a", "b", "c", "a"].map(String::from).to_vec();
        assert_eq!(names(&Tag::parse_list(tags).unwrap()), ["b", "a", "c"]);
    }

    // AC-arch-3.6.4 (was tag_slug_is_under_tags)
    #[test]
    fn tag_slug_is_under_tags() {
        assert_eq!(tag("blog").slug().to_string(), "tags/blog");
        assert_eq!(tag("blog").url(), "/tags/blog/");
        assert_eq!(tag("blog").slug().parent(), Some(Slug::tag_index()));
    }

    // AC-arch-3.6.5
    #[test]
    fn orders_bytewise_and_serializes_as_plain_string() {
        assert!(tag("a-b") < tag("a1"));
        assert!(tag("2026") < tag("blog"));
        assert_eq!(tag("static-site").to_string(), "static-site");
        assert_eq!(
            serde_json::to_value(tag("static-site")).unwrap(),
            serde_json::json!("static-site")
        );
    }
}
