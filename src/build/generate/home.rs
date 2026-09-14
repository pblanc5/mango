use crate::{
    build::index::section::{SectionIndex, compare_summaries},
    config::SiteConfig,
    content::{page::Page, summary::PageSummary},
    render::template::{self, HomeSection, RenderItem},
};

/// The home page item (`dist/index.html`): up to `recent_count` dated pages
/// from the whole site, newest first, and the top-level sections by slug.
pub fn build(pages: &[Page], si: &SectionIndex, config: &SiteConfig) -> RenderItem {
    let mut recent: Vec<PageSummary> = pages
        .iter()
        .filter(|page| page.date.is_some())
        .map(PageSummary::from)
        .collect();
    recent.sort_by(compare_summaries);
    recent.truncate(config.recent_count);

    let sections = si
        .sections
        .iter()
        .filter(|(slug, _)| !slug.contains('/'))
        .map(|(slug, section)| HomeSection::new(slug.clone(), section.pages.len()))
        .collect();

    template::render_home_page(recent, sections, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::index::section::build_section_index,
        content::{frontmatter::MangoFrontmatter, page::PageType},
    };

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

    fn home_for(pages: &[Page], config: &SiteConfig) -> RenderItem {
        build(pages, &build_section_index(pages), config)
    }

    fn recent_slugs(item: &RenderItem) -> Vec<String> {
        item.context.get("home").unwrap()["recent"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["slug"].as_str().unwrap().to_string())
            .collect()
    }

    // AC-2.1
    #[test]
    fn home_item_has_empty_slug_and_source() {
        let item = home_for(&[], &SiteConfig::default());
        assert_eq!(item.slug, "");
        assert_eq!(item.source, "home page");
        assert_eq!(item.template, "home.html");
        assert!(item.context.get("config").is_some());
    }

    // AC-2.2, AC-2.3
    #[test]
    fn recent_is_dated_sorted_and_truncated() {
        let pages = vec![
            page_with("about", "About", Some("2026-01-01")),
            page_with("posts/old", "Old", Some("2020-01-01")),
            page_with("posts/undated", "Undated", None),
            page_with("projects/new", "New", Some("2026-02-07")),
            page_with("posts/b", "Same", Some("2026-01-01")),
            page_with("posts/a", "Same", Some("2026-01-01")),
        ];

        let all = home_for(&pages, &SiteConfig::default());
        assert_eq!(
            recent_slugs(&all),
            ["projects/new", "about", "posts/a", "posts/b", "posts/old"]
        );

        let first = &all.context.get("home").unwrap()["recent"][0];
        assert_eq!(first["title"], "New");
        assert_eq!(first["date"], "2026-02-07");
        assert_eq!(first["url"], "/projects/new/");

        let two = SiteConfig {
            recent_count: 2,
            ..SiteConfig::default()
        };
        assert_eq!(
            recent_slugs(&home_for(&pages, &two)),
            ["projects/new", "about"]
        );

        let zero = SiteConfig {
            recent_count: 0,
            ..SiteConfig::default()
        };
        assert!(recent_slugs(&home_for(&pages, &zero)).is_empty());
    }

    // AC-2.4
    #[test]
    fn sections_are_top_level_sorted_with_counts() {
        let pages = vec![
            page_with("projects/x", "X", None),
            page_with("posts/one", "One", None),
            page_with("posts/two", "Two", None),
            page_with("a/b/c", "C", None),
            page_with("about", "About", None),
        ];

        let item = home_for(&pages, &SiteConfig::default());
        let sections = item.context.get("home").unwrap()["sections"]
            .as_array()
            .unwrap()
            .clone();
        let slugs: Vec<_> = sections.iter().map(|s| s["slug"].clone()).collect();
        assert_eq!(slugs, ["a", "posts", "projects"]);
        assert_eq!(sections[0]["page_count"], 0);
        assert_eq!(sections[1]["page_count"], 2);
        assert_eq!(sections[1]["url"], "/posts/");
        assert_eq!(sections[2]["page_count"], 1);
    }
}
