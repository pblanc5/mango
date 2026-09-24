use crate::{
    build::{index::section::SectionIndex, output::Output},
    config::SiteConfig,
    render::template::{self, SectionLink},
};

/// Section outputs come out in ascending section-slug order (the index is a
/// `BTreeMap`), ancestor sections included.
pub fn build(si: SectionIndex, config: &SiteConfig) -> Vec<Output> {
    si.sections
        .into_iter()
        .map(|(slug, section)| {
            let subsections = section
                .subsections
                .into_iter()
                .map(SectionLink::new)
                .collect();
            template::render_section_page(slug, section.pages, subsections, config)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::{index::section::build_section_index, output::OutputKind},
        content::{
            frontmatter::MangoFrontmatter,
            page::{Page, PageType},
            slug::Slug,
        },
    };
    use std::path::Path;

    fn page_with_slug(slug: &str) -> Page {
        let fm = MangoFrontmatter {
            title: slug.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: None,
            tags: None,
            draft: false,
        };
        Page::new(
            fm,
            String::new(),
            PageType::General,
            Path::new(&format!("site/{slug}.md")),
            Path::new("site"),
        )
        .unwrap()
    }

    // AC-2.4, AC-2.5; AC-3.7 (batch 3)
    #[test]
    fn render_items_in_section_slug_order() {
        let pages = vec![
            page_with_slug("projects/x"),
            page_with_slug("posts/x"),
            page_with_slug("a/b/c"),
        ];

        let items = build(build_section_index(&pages), &SiteConfig::default());
        let kinds: Vec<_> = items.iter().map(|i| &i.kind).collect();
        let expected: Vec<_> = ["a", "a/b", "posts", "projects"]
            .map(|s| OutputKind::Section(Slug::from_test_text(s)))
            .into_iter()
            .collect();
        assert_eq!(kinds, expected.iter().collect::<Vec<_>>());
        assert_eq!(items[2].kind.to_string(), "section index 'posts'");

        // AC-3.3, AC-1.11
        let a = items[0].context().get("section").unwrap();
        assert_eq!(a["subsections"][0]["url"], "/a/b/");
        assert!(items[0].context().get("config").is_some());
    }
}
