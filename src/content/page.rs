use std::path::Path;

use chrono::NaiveDate;
use serde::{Serialize, Serializer};

use crate::{content::frontmatter::MangoFrontmatter, error::MangoError};

#[derive(Serialize, Debug)]
pub enum PageType {
    General,
}

#[derive(Serialize, Debug)]
pub struct Page {
    pub title: String,
    pub author: String,
    pub description: String,
    #[serde(serialize_with = "serialize_date")]
    pub date: Option<NaiveDate>,
    pub slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub draft: bool,
    pub kind: PageType,
}

impl Page {
    pub fn new(fm: MangoFrontmatter, content: String, kind: PageType) -> Result<Self, MangoError> {
        let date = fm.date.as_deref().map(parse_date).transpose()?;

        Ok(Page {
            title: fm.title,
            author: fm.author,
            description: fm.description,
            date,
            slug: String::new(),
            tags: fm.tags.unwrap_or_default(),
            content,
            draft: fm.draft,
            kind,
        })
    }

    pub fn generate_slug(&mut self, path: &Path, site: &Path) -> Result<(), MangoError> {
        let slug = path
            .strip_prefix(site)
            .map_err(|_e| MangoError::General("unable to generate slug from path".into()))?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        self.slug = slug;
        Ok(())
    }
}

/// Parses a strict `YYYY-MM-DD` date. chrono's `parse_from_str` accepts
/// unpadded and signed fields, so the shape is checked by hand first.
pub fn parse_date(value: &str) -> Result<NaiveDate, MangoError> {
    let invalid = || {
        MangoError::Frontmatter(format!(
            "invalid date '{value}': expected a valid YYYY-MM-DD date"
        ))
    };

    let bytes = value.as_bytes();
    let well_formed = bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        });
    if !well_formed {
        return Err(invalid());
    }

    let year: i32 = value[0..4].parse().map_err(|_| invalid())?;
    let month: u32 = value[5..7].parse().map_err(|_| invalid())?;
    let day: u32 = value[8..10].parse().map_err(|_| invalid())?;

    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(invalid)
}

/// Serializes a page date for templates: `"YYYY-MM-DD"`, or `""` when unset
/// so `{% if page.date %}` stays falsy.
pub fn serialize_date<S: Serializer>(date: &Option<NaiveDate>, s: S) -> Result<S::Ok, S::Error> {
    match date {
        Some(d) => s.serialize_str(&d.format("%Y-%m-%d").to_string()),
        None => s.serialize_str(""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontmatter() -> MangoFrontmatter {
        MangoFrontmatter {
            title: "Title".into(),
            author: "Author".into(),
            description: "Description".into(),
            date: None,
            tags: None,
            draft: false,
        }
    }

    // AC-1.4
    #[test]
    fn new_defaults_missing_date_and_tags() {
        let page = Page::new(frontmatter(), "content".into(), PageType::General).unwrap();

        assert_eq!(page.date, None);
        assert!(page.tags.is_empty());
        assert_eq!(page.slug, "");
    }

    #[test]
    fn generate_slug_strips_site_prefix_and_extension() {
        let mut page = Page::new(frontmatter(), String::new(), PageType::General).unwrap();
        page.generate_slug(Path::new("site/posts/post_one.md"), Path::new("site"))
            .unwrap();

        assert_eq!(page.slug, "posts/post_one");
    }

    #[test]
    fn generate_slug_fails_when_path_is_outside_site() {
        let mut page = Page::new(frontmatter(), String::new(), PageType::General).unwrap();
        let result = page.generate_slug(Path::new("elsewhere/post.md"), Path::new("site"));

        assert!(matches!(result, Err(MangoError::General(_))));
    }

    // AC-1.1
    #[test]
    fn parse_date_accepts_valid_and_leap_day_dates() {
        assert_eq!(
            parse_date("2026-01-24").unwrap(),
            NaiveDate::from_ymd_opt(2026, 1, 24).unwrap()
        );
        assert_eq!(
            parse_date("2024-02-29").unwrap(),
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
    }

    // AC-1.2
    #[test]
    fn parse_date_rejects_wrong_formats() {
        let bad = [
            "2026/01/24",
            "2026-1-24",
            "26-01-24",
            "2026-01-24T10:00:00",
            "+2026-01-24",
            " 2026-01-24",
            "",
        ];
        for value in bad {
            let err = parse_date(value).expect_err(value);
            assert!(
                matches!(err, MangoError::Frontmatter(_)),
                "{value}: {err:?}"
            );
            assert!(
                err.to_string().contains(&format!("'{value}'")),
                "{value}: {err}"
            );
        }
    }

    // AC-1.3
    #[test]
    fn parse_date_rejects_impossible_dates() {
        for value in ["2026-02-30", "2026-13-01", "2025-02-29", "2026-00-10"] {
            let err = parse_date(value).expect_err(value);
            assert!(
                matches!(err, MangoError::Frontmatter(_)),
                "{value}: {err:?}"
            );
            assert!(err.to_string().contains(value), "{value}: {err}");
        }
    }

    // AC-1.2
    #[test]
    fn new_rejects_invalid_date() {
        let mut fm = frontmatter();
        fm.date = Some("2026-02-30".into());
        let result = Page::new(fm, String::new(), PageType::General);
        assert!(matches!(result, Err(MangoError::Frontmatter(_))));
    }

    // AC-1.5
    #[test]
    fn page_date_serializes_formatted_or_empty() {
        let mut fm = frontmatter();
        fm.date = Some("2026-01-24".into());
        let dated = Page::new(fm, String::new(), PageType::General).unwrap();
        let undated = Page::new(frontmatter(), String::new(), PageType::General).unwrap();

        assert_eq!(serde_json::to_value(&dated).unwrap()["date"], "2026-01-24");
        assert_eq!(serde_json::to_value(&undated).unwrap()["date"], "");
    }
}
