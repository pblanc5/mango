use std::{fmt::Write as _, path::PathBuf};

use crate::{
    build::{
        generate::{home::recent_pages, xml::escape},
        output::GeneratedFile,
    },
    config::{self, SiteConfig},
    content::page::{Page, slug_url},
};

/// `dist/feed.xml`: an RSS 2.0 feed of the home page's recent list. `None`
/// when `base_url` is unset, since RSS needs absolute links.
pub fn build(pages: &[Page], config: &SiteConfig) -> Option<GeneratedFile> {
    let base = config::base_url_root(config)?;

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<rss version=\"2.0\">\n");
    xml.push_str("  <channel>\n");
    let _ = writeln!(
        xml,
        "    <title>{}</title>",
        escape(config.title.as_deref().unwrap_or(""))
    );
    let _ = writeln!(xml, "    <link>{}</link>", escape(&format!("{base}/")));
    let _ = writeln!(
        xml,
        "    <description>{}</description>",
        escape(config.description.as_deref().unwrap_or(""))
    );

    for page in recent_pages(pages, config.recent_count) {
        // recent_pages only returns dated pages.
        let Some(date) = page.date else { continue };
        let link = escape(&format!("{base}{}", slug_url(&page.slug)));

        xml.push_str("    <item>\n");
        let _ = writeln!(xml, "      <title>{}</title>", escape(&page.title));
        let _ = writeln!(xml, "      <link>{link}</link>");
        let _ = writeln!(xml, "      <guid isPermaLink=\"true\">{link}</guid>");
        let _ = writeln!(
            xml,
            "      <description>{}</description>",
            escape(&page.description)
        );
        let _ = writeln!(
            xml,
            "      <pubDate>{}</pubDate>",
            date.format("%a, %d %b %Y 00:00:00 +0000")
        );
        xml.push_str("    </item>\n");
    }

    xml.push_str("  </channel>\n");
    xml.push_str("</rss>\n");

    Some(GeneratedFile {
        path: PathBuf::from("feed.xml"),
        source: "RSS feed".into(),
        contents: xml,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

    fn page_with(slug: &str, title: &str, description: &str, date: Option<&str>) -> Page {
        let fm = MangoFrontmatter {
            title: title.to_string(),
            author: "author".into(),
            description: description.to_string(),
            date: date.map(String::from),
            tags: None,
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = slug.to_string();
        page
    }

    fn config_with(base_url: Option<&str>) -> SiteConfig {
        SiteConfig {
            title: Some("Mango Test Site".into()),
            description: Some("A test site".into()),
            base_url: base_url.map(String::from),
            ..SiteConfig::default()
        }
    }

    fn fixture_pages() -> Vec<Page> {
        vec![
            page_with(
                "posts/post_one",
                "Post One",
                "my first post",
                Some("2026-01-24"),
            ),
            page_with("about", "About", "undated page", None),
            page_with(
                "projects/mango",
                "Mango Task Tracker",
                "tracks progress",
                Some("2026-02-07"),
            ),
        ]
    }

    fn item_count(feed: &GeneratedFile) -> usize {
        feed.contents.matches("<item>").count()
    }

    // AC-6.1 (batch 4)
    #[test]
    fn no_feed_without_base_url() {
        assert!(build(&fixture_pages(), &config_with(None)).is_none());
    }

    // AC-6.2, AC-6.3, AC-6.8 (batch 4)
    #[test]
    fn golden_feed() {
        let feed = build(&fixture_pages(), &config_with(Some("https://example.com"))).unwrap();

        assert_eq!(feed.path, PathBuf::from("feed.xml"));
        assert_eq!(feed.source, "RSS feed");
        let expected = "\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<rss version=\"2.0\">
  <channel>
    <title>Mango Test Site</title>
    <link>https://example.com/</link>
    <description>A test site</description>
    <item>
      <title>Mango Task Tracker</title>
      <link>https://example.com/projects/mango/</link>
      <guid isPermaLink=\"true\">https://example.com/projects/mango/</guid>
      <description>tracks progress</description>
      <pubDate>Sat, 07 Feb 2026 00:00:00 +0000</pubDate>
    </item>
    <item>
      <title>Post One</title>
      <link>https://example.com/posts/post_one/</link>
      <guid isPermaLink=\"true\">https://example.com/posts/post_one/</guid>
      <description>my first post</description>
      <pubDate>Sat, 24 Jan 2026 00:00:00 +0000</pubDate>
    </item>
  </channel>
</rss>
";
        assert_eq!(feed.contents, expected);
    }

    // AC-6.2 (batch 4)
    #[test]
    fn unset_title_and_description_are_empty() {
        let config = SiteConfig {
            base_url: Some("https://example.com".into()),
            ..SiteConfig::default()
        };
        let feed = build(&[], &config).unwrap();
        assert!(
            feed.contents.contains("    <title></title>\n"),
            "{}",
            feed.contents
        );
        assert!(
            feed.contents.contains("    <description></description>\n"),
            "{}",
            feed.contents
        );
        assert_eq!(item_count(&feed), 0);
    }

    // AC-6.4 (batch 4)
    #[test]
    fn trailing_slash_base_url_has_no_double_slash() {
        let pages = fixture_pages();
        let plain = build(&pages, &config_with(Some("https://example.com"))).unwrap();
        let slashed = build(&pages, &config_with(Some("https://example.com/"))).unwrap();
        let many = build(&pages, &config_with(Some("https://example.com///"))).unwrap();

        assert_eq!(plain.contents, slashed.contents);
        assert_eq!(plain.contents, many.contents);
        let without_scheme = slashed.contents.replace("https://", "");
        assert!(!without_scheme.contains("//"), "{}", slashed.contents);
    }

    // AC-6.5 (batch 4)
    #[test]
    fn values_are_escaped() {
        let pages = vec![page_with(
            "posts/a&b",
            "Tom & \"Jerry\" <3",
            "it's <b>bold</b>",
            Some("2026-01-24"),
        )];
        let config = SiteConfig {
            title: Some("A & B".into()),
            description: Some("<site>".into()),
            base_url: Some("https://example.com/?x='1'".into()),
            ..SiteConfig::default()
        };

        let xml = build(&pages, &config).unwrap().contents;
        assert!(xml.contains("<title>A &amp; B</title>"), "{xml}");
        assert!(
            xml.contains("<description>&lt;site&gt;</description>"),
            "{xml}"
        );
        assert!(
            xml.contains("<link>https://example.com/?x=&apos;1&apos;/</link>"),
            "{xml}"
        );
        assert!(
            xml.contains("<title>Tom &amp; &quot;Jerry&quot; &lt;3</title>"),
            "{xml}"
        );
        assert!(
            xml.contains("<link>https://example.com/?x=&apos;1&apos;/posts/a&amp;b/</link>"),
            "{xml}"
        );
        assert!(
            xml.contains("<description>it&apos;s &lt;b&gt;bold&lt;/b&gt;</description>"),
            "{xml}"
        );
    }

    // AC-6.6 (batch 4)
    #[test]
    fn undated_pages_excluded() {
        let feed = build(&fixture_pages(), &config_with(Some("https://example.com"))).unwrap();
        assert_eq!(item_count(&feed), 2);
        assert!(!feed.contents.contains("About"), "{}", feed.contents);
        assert!(!feed.contents.contains("/about/"), "{}", feed.contents);
    }

    // AC-6.6, AC-6.7 (batch 4)
    #[test]
    fn item_count_follows_recent_count() {
        let pages = fixture_pages();
        for (count, expected) in [(0, 0), (1, 1), (2, 2), (10, 2)] {
            let config = SiteConfig {
                recent_count: count,
                ..config_with(Some("https://example.com"))
            };
            let feed = build(&pages, &config).unwrap();
            assert_eq!(item_count(&feed), expected, "recent_count {count}");
        }

        let one = SiteConfig {
            recent_count: 1,
            ..config_with(Some("https://example.com"))
        };
        let feed = build(&pages, &one).unwrap();
        assert!(
            feed.contents.contains("Mango Task Tracker"),
            "{}",
            feed.contents
        );
    }
}
