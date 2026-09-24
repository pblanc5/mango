use std::collections::BTreeMap;

use crate::{
    config::SiteConfig,
    content::{summary::PageSummary, tag::Tag},
    render::template::{self, RenderItem, TagIndexEntry},
};

/// The tag index item first (always, even with no tags, so `/tags/` is
/// reserved on every site), then one tag page item per tag in name order.
pub fn build(index: BTreeMap<Tag, Vec<PageSummary>>, config: &SiteConfig) -> Vec<RenderItem> {
    let entries = index
        .iter()
        .map(|(name, pages)| TagIndexEntry::new(name.clone(), pages.len()))
        .collect();

    let mut items = vec![template::render_tag_index(entries, config)];
    items.extend(
        index
            .into_iter()
            .map(|(name, pages)| template::render_tag_page(name, pages, config)),
    );
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::index::tag::build_tag_index,
        content::{
            frontmatter::MangoFrontmatter,
            page::{Page, PageType},
            slug::Slug,
        },
    };
    use std::path::Path;

    fn page_with(slug: &str, tags: &[&str]) -> Page {
        let fm = MangoFrontmatter {
            title: slug.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: None,
            tags: Some(tags.iter().map(|t| t.to_string()).collect()),
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

    // AC-2.2, AC-2.3, AC-2.4 (batch 4)
    #[test]
    fn index_first_then_tags_in_name_order() {
        let pages = vec![
            page_with("posts/a", &["rust", "blog"]),
            page_with("posts/b", &["blog"]),
        ];

        let items = build(build_tag_index(&pages), &SiteConfig::default());
        let slugs: Vec<_> = items.iter().map(|i| i.slug.to_string()).collect();
        assert_eq!(slugs, ["tags", "tags/blog", "tags/rust"]);
        let sources: Vec<_> = items.iter().map(|i| i.source.as_str()).collect();
        assert_eq!(sources, ["tag index", "tag page 'blog'", "tag page 'rust'"]);

        let tags = items[0].context.get("tags").unwrap();
        assert_eq!(tags[0]["name"], "blog");
        assert_eq!(tags[0]["page_count"], 2);
        assert_eq!(tags[1]["name"], "rust");
        assert_eq!(tags[1]["url"], "/tags/rust/");
        assert_eq!(tags[1]["page_count"], 1);

        let blog = items[1].context.get("tag").unwrap();
        assert_eq!(blog["pages"].as_array().unwrap().len(), 2);
        assert!(items.iter().all(|i| i.context.get("config").is_some()));
    }

    // AC-2.4 (batch 4)
    #[test]
    fn index_generated_without_tags() {
        let items = build(
            build_tag_index(&[page_with("a", &[])]),
            &SiteConfig::default(),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].slug, Slug::tag_index());
        assert_eq!(items[0].template, "tags.html");
        assert_eq!(
            items[0].context.get("tags").unwrap(),
            &serde_json::json!([])
        );
    }
}
