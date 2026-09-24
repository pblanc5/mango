use std::{fmt::Write as _, path::PathBuf};

use chrono::NaiveDate;

use crate::{
    build::{generate::xml::escape, output::GeneratedFile},
    config::{self, SiteConfig},
    render::template::RenderItem,
};

/// `dist/sitemap.xml` (sitemaps.org 0.9): one `<url>` per HTML render item,
/// sorted by `loc`. Only dated content pages get `<lastmod>`. `None` when
/// `base_url` is unset, since sitemaps need absolute URLs.
pub fn build<'a>(
    items: impl IntoIterator<Item = &'a RenderItem>,
    config: &SiteConfig,
) -> Option<GeneratedFile> {
    let base = config::base_url_root(config)?;

    let mut entries: Vec<(String, Option<NaiveDate>)> = items
        .into_iter()
        .map(|item| (format!("{base}{}", item.slug.url()), item.page_date))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for (loc, date) in entries {
        xml.push_str("  <url>\n");
        let _ = writeln!(xml, "    <loc>{}</loc>", escape(&loc));
        if let Some(date) = date {
            let _ = writeln!(xml, "    <lastmod>{}</lastmod>", date.format("%Y-%m-%d"));
        }
        xml.push_str("  </url>\n");
    }
    xml.push_str("</urlset>\n");

    Some(GeneratedFile {
        path: PathBuf::from("sitemap.xml"),
        source: "sitemap".into(),
        contents: xml,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        build::{
            generate::{home, section, tag},
            index::{section::build_section_index, tag::build_tag_index},
        },
        content::{
            frontmatter::MangoFrontmatter,
            page::{Page, PageType},
        },
        render::template,
    };
    use std::path::Path;

    fn page_with(slug: &str, date: Option<&str>, tags: &[&str]) -> Page {
        let fm = MangoFrontmatter {
            title: slug.to_string(),
            author: "author".into(),
            description: "description".into(),
            date: date.map(String::from),
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

    fn with_base(base_url: Option<&str>) -> SiteConfig {
        SiteConfig {
            base_url: base_url.map(String::from),
            ..SiteConfig::default()
        }
    }

    /// Every render item kind the build produces, in pipeline order.
    fn all_items(pages: &[Page], config: &SiteConfig) -> Vec<RenderItem> {
        let mut items: Vec<RenderItem> = pages
            .iter()
            .map(|p| template::render_page(p, config).unwrap())
            .collect();
        let si = build_section_index(pages);
        let home_item = home::build(pages, &si, config);
        items.extend(section::build(si, config));
        items.push(home_item);
        items.extend(tag::build(build_tag_index(pages), config));
        items
    }

    fn locs(xml: &str) -> Vec<&str> {
        xml.match_indices("<loc>")
            .map(|(i, _)| {
                let from = i + "<loc>".len();
                let len = xml[from..].find("</loc>").unwrap();
                &xml[from..from + len]
            })
            .collect()
    }

    // AC-7.1 (batch 4)
    #[test]
    fn no_sitemap_without_base_url() {
        let pages = vec![page_with("posts/one", None, &[])];
        let config = with_base(None);
        assert!(build(&all_items(&pages, &config), &config).is_none());
    }

    // AC-7.1, AC-7.2, AC-7.3 (batch 4)
    #[test]
    fn covers_every_item_kind_sorted() {
        let pages = vec![
            page_with("posts/one", Some("2026-01-24"), &["blog"]),
            page_with("about", None, &[]),
            page_with("a/b/c", None, &["rust"]),
        ];
        let config = with_base(Some("https://example.com"));
        let items = all_items(&pages, &config);

        let sitemap = build(&items, &config).unwrap();
        assert_eq!(sitemap.path, PathBuf::from("sitemap.xml"));
        assert_eq!(sitemap.source, "sitemap");
        assert!(sitemap.contents.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n"
        ));
        assert!(sitemap.contents.ends_with("</urlset>\n"));

        assert_eq!(
            locs(&sitemap.contents),
            [
                "https://example.com/",
                "https://example.com/a/",
                "https://example.com/a/b/",
                "https://example.com/a/b/c/",
                "https://example.com/about/",
                "https://example.com/posts/",
                "https://example.com/posts/one/",
                "https://example.com/tags/",
                "https://example.com/tags/blog/",
                "https://example.com/tags/rust/",
            ]
        );
        assert_eq!(locs(&sitemap.contents).len(), items.len());
    }

    // AC-7.3 (batch 4): plain string order, not slug or item order.
    #[test]
    fn sorted_by_loc_as_plain_strings() {
        let config = with_base(Some("https://example.com"));
        let pages = [
            page_with("b", None, &[]),
            page_with("a-z", None, &[]),
            page_with("a", None, &[]),
        ];
        let items: Vec<_> = pages
            .iter()
            .map(|p| template::render_page(p, &config).unwrap())
            .collect();
        let sitemap = build(&items, &config).unwrap();
        // '-' (0x2D) sorts before '/' (0x2F).
        assert_eq!(
            locs(&sitemap.contents),
            [
                "https://example.com/a-z/",
                "https://example.com/a/",
                "https://example.com/b/",
            ]
        );
    }

    // AC-7.4 (batch 4)
    #[test]
    fn lastmod_only_on_dated_pages() {
        let pages = vec![
            page_with("posts/one", Some("2026-01-24"), &["blog"]),
            page_with("posts/two", None, &["blog"]),
        ];
        let config = with_base(Some("https://example.com"));
        let xml = build(&all_items(&pages, &config), &config)
            .unwrap()
            .contents;

        assert_eq!(xml.matches("<lastmod>").count(), 1, "{xml}");
        assert!(
            xml.contains(
                "  <url>\n    <loc>https://example.com/posts/one/</loc>\n    <lastmod>2026-01-24</lastmod>\n  </url>\n"
            ),
            "{xml}"
        );
        assert!(
            xml.contains("  <url>\n    <loc>https://example.com/posts/two/</loc>\n  </url>\n"),
            "{xml}"
        );
        assert!(
            xml.contains("  <url>\n    <loc>https://example.com/</loc>\n  </url>\n"),
            "{xml}"
        );
    }

    // AC-7.5 (batch 4)
    #[test]
    fn escapes_loc() {
        let config = with_base(Some("https://example.com/?a=1&b='2'"));
        let item = template::render_page(&page_with("x-y", None, &[]), &config).unwrap();
        let xml = build([&item], &config).unwrap().contents;
        assert!(
            xml.contains("<loc>https://example.com/?a=1&amp;b=&apos;2&apos;/x-y/</loc>"),
            "{xml}"
        );
    }

    // AC-7.5 (batch 4)
    #[test]
    fn trailing_slash_base_url() {
        let pages = vec![page_with("posts/one", Some("2026-01-24"), &["blog"])];
        let plain_config = with_base(Some("https://example.com"));
        let slashed_config = with_base(Some("https://example.com/"));

        let plain = build(&all_items(&pages, &plain_config), &plain_config).unwrap();
        let slashed = build(&all_items(&pages, &slashed_config), &slashed_config).unwrap();

        assert_eq!(plain.contents, slashed.contents);
        let without_scheme = slashed
            .contents
            .replace("https://", "")
            .replace("http://", "");
        assert!(!without_scheme.contains("//"), "{}", slashed.contents);
    }
}
