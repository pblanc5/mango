use std::collections::BTreeMap;

use crate::{
    build::index::section::compare_summaries,
    content::{page::Page, summary::PageSummary},
};

/// Pages grouped by tag, ordered by tag name. Each tag's pages are sorted
/// with `compare_summaries` (newest first, undated last). `pages` must
/// already be draft-filtered.
pub fn build_tag_index(pages: &[Page]) -> BTreeMap<String, Vec<PageSummary>> {
    let mut index: BTreeMap<String, Vec<PageSummary>> = BTreeMap::new();

    for page in pages {
        for tag in &page.tags {
            index
                .entry(tag.clone())
                .or_default()
                .push(PageSummary::from(page));
        }
    }

    for summaries in index.values_mut() {
        summaries.sort_by(compare_summaries);
    }

    index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

    fn page_with(slug: &str, title: &str, date: Option<&str>, tags: &[&str]) -> Page {
        let fm = MangoFrontmatter {
            title: title.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: date.map(String::from),
            tags: Some(tags.iter().map(|t| t.to_string()).collect()),
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = slug.to_string();
        page
    }

    fn slugs(summaries: &[PageSummary]) -> Vec<&str> {
        summaries.iter().map(|s| s.slug.as_str()).collect()
    }

    // AC-2.1 (batch 4)
    #[test]
    fn groups_pages_by_tag_sorted() {
        let pages = vec![
            page_with("posts/old", "Old", Some("2020-01-01"), &["rust", "blog"]),
            page_with("posts/new", "New", Some("2026-01-01"), &["blog"]),
            page_with("about", "About", None, &[]),
            page_with("projects/b", "Same", Some("2025-01-01"), &["rust"]),
            page_with("projects/a", "Same", Some("2025-01-01"), &["rust"]),
        ];

        let index = build_tag_index(&pages);
        let keys: Vec<_> = index.keys().map(String::as_str).collect();
        assert_eq!(keys, ["blog", "rust"]);
        assert_eq!(slugs(&index["blog"]), ["posts/new", "posts/old"]);
        assert_eq!(
            slugs(&index["rust"]),
            ["projects/a", "projects/b", "posts/old"]
        );
    }

    // AC-2.1 (batch 4)
    #[test]
    fn undated_pages_sort_last() {
        let pages = vec![
            page_with("posts/undated", "A", None, &["blog"]),
            page_with("posts/old", "Z", Some("2001-01-01"), &["blog"]),
            page_with("posts/new", "Y", Some("2026-01-01"), &["blog"]),
        ];

        let index = build_tag_index(&pages);
        assert_eq!(
            slugs(&index["blog"]),
            ["posts/new", "posts/old", "posts/undated"]
        );
    }

    #[test]
    fn no_tags_gives_empty_index() {
        let index = build_tag_index(&[page_with("a", "A", None, &[])]);
        assert!(index.is_empty());
    }

    // AC-3.2 (batch 4)
    #[test]
    fn summaries_have_no_tags_key() {
        let index = build_tag_index(&[page_with("a/b", "A", None, &["blog"])]);
        let json = serde_json::to_value(&index["blog"]).unwrap();
        assert!(json[0].get("tags").is_none(), "{json}");
        assert_eq!(json[0]["url"], "/a/b/");
    }
}
