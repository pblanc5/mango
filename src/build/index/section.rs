use std::collections::HashMap;

use crate::content::{page::Page, summary::PageSummary};

pub type SectionSlug = String;

#[derive(Default)]
pub struct SectionIndex {
    pub sections: HashMap<SectionSlug, Vec<PageSummary>>,
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

    si
}

fn extract_parent_slug(slug: &str) -> Option<String> {
    slug.rsplit_once('/').map(|(p, _)| p.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

    fn page_with_slug(slug: &str) -> Page {
        let fm = MangoFrontmatter {
            title: slug.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: None,
            tags: None,
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General);
        page.slug = slug.to_string();
        page
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
}
