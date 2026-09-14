use std::{cmp::Ordering, collections::BTreeMap};

use crate::content::{page::Page, summary::PageSummary};

pub type SectionSlug = String;

#[derive(Default)]
pub struct SectionIndex {
    /// Ordered by section slug so output is generated in a fixed order.
    pub sections: BTreeMap<SectionSlug, Vec<PageSummary>>,
}

impl SectionIndex {
    fn new() -> Self {
        SectionIndex::default()
    }
}

pub fn build_section_index(pages: &[Page]) -> SectionIndex {
    let mut si = SectionIndex::new();

    let summaries = pages.iter().map(PageSummary::from).collect::<Vec<_>>();

    for summary in summaries {
        if let Some(parent) = extract_parent_slug(&summary.slug) {
            si.sections.entry(parent).or_default().push(summary);
        }
    }

    for summaries in si.sections.values_mut() {
        summaries.sort_by(compare_summaries);
    }

    si
}

/// Newest date first, undated pages last, then title, then slug.
fn compare_summaries(a: &PageSummary, b: &PageSummary) -> Ordering {
    let by_date = match (a.date, b.date) {
        (Some(x), Some(y)) => y.cmp(&x),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };

    by_date
        .then_with(|| a.title.cmp(&b.title))
        .then_with(|| a.slug.cmp(&b.slug))
}

fn extract_parent_slug(slug: &str) -> Option<String> {
    slug.rsplit_once('/').map(|(p, _)| p.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

    fn page_with(slug: &str, title: &str, date: Option<&str>) -> Page {
        let fm = MangoFrontmatter {
            title: title.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: date.map(String::from),
            tags: None,
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = slug.to_string();
        page
    }

    fn page_with_slug(slug: &str) -> Page {
        page_with(slug, slug, None)
    }

    fn slugs(si: &SectionIndex, section: &str) -> Vec<String> {
        si.sections[section]
            .iter()
            .map(|s| s.slug.clone())
            .collect()
    }

    #[test]
    fn groups_pages_under_their_parent_slug() {
        let pages = vec![
            page_with_slug("posts/one"),
            page_with_slug("posts/two"),
            page_with_slug("projects/mango"),
        ];

        let si = build_section_index(&pages);

        assert_eq!(si.sections.len(), 2);
        assert_eq!(si.sections["posts"].len(), 2);
        assert_eq!(si.sections["projects"].len(), 1);
        assert_eq!(si.sections["projects"][0].slug, "projects/mango");
    }

    #[test]
    fn top_level_pages_are_not_indexed() {
        let si = build_section_index(&[page_with_slug("about")]);
        assert!(si.sections.is_empty());
    }

    #[test]
    fn extract_parent_slug_uses_last_separator() {
        assert_eq!(extract_parent_slug("a/b/c"), Some("a/b".to_string()));
        assert_eq!(extract_parent_slug("posts/one"), Some("posts".to_string()));
        assert_eq!(extract_parent_slug("about"), None);
    }

    // AC-2.1, AC-2.5
    #[test]
    fn pages_sorted_newest_first() {
        let pages = vec![
            page_with("posts/old", "A", Some("2020-05-01")),
            page_with("posts/mid", "B", Some("2025-12-31")),
            page_with("posts/new", "C", Some("2026-01-01")),
        ];

        let si = build_section_index(&pages);
        assert_eq!(slugs(&si, "posts"), ["posts/new", "posts/mid", "posts/old"]);
    }

    // AC-2.2, AC-2.5
    #[test]
    fn undated_pages_come_last() {
        let pages = vec![
            page_with("posts/undated", "A", None),
            page_with("posts/old", "Z", Some("2001-01-01")),
            page_with("posts/new", "Y", Some("2026-01-01")),
        ];

        let si = build_section_index(&pages);
        assert_eq!(
            slugs(&si, "posts"),
            ["posts/new", "posts/old", "posts/undated"]
        );
    }

    // AC-2.3, AC-2.5
    #[test]
    fn ties_broken_by_title_then_slug() {
        let pages = vec![
            page_with("posts/z", "Same", Some("2026-01-24")),
            page_with("posts/b", "Beta", Some("2026-01-24")),
            page_with("posts/a", "Same", Some("2026-01-24")),
            page_with("posts/u2", "Undated", None),
            page_with("posts/u1", "Undated", None),
            page_with("posts/u3", "Alpha", None),
        ];

        let si = build_section_index(&pages);
        assert_eq!(
            slugs(&si, "posts"),
            [
                "posts/b", "posts/a", "posts/z", "posts/u3", "posts/u1", "posts/u2"
            ]
        );
    }

    // AC-2.4, AC-2.5
    #[test]
    fn sections_iterate_in_slug_order() {
        let pages = vec![
            page_with_slug("projects/x"),
            page_with_slug("a/b/c"),
            page_with_slug("posts/x"),
            page_with_slug("a/x"),
        ];

        let si = build_section_index(&pages);
        let keys: Vec<_> = si.sections.keys().map(String::as_str).collect();
        assert_eq!(keys, ["a", "a/b", "posts", "projects"]);
    }

    // AC-1.5
    #[test]
    fn summary_date_serializes_formatted_or_empty() {
        let pages = vec![
            page_with("posts/dated", "A", Some("2026-01-24")),
            page_with("posts/undated", "B", None),
        ];

        let si = build_section_index(&pages);
        let json = serde_json::to_value(&si.sections["posts"]).unwrap();
        assert_eq!(json[0]["date"], "2026-01-24");
        assert_eq!(json[1]["date"], "");
    }
}
