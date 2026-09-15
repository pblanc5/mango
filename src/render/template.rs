use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;
use tera::Tera;

use crate::{
    config::SiteConfig,
    content::{
        page::{Page, slug_url, tag_slug},
        summary::PageSummary,
    },
    error::MangoError,
    render::markdown::to_html,
};

#[derive(Serialize, Default)]
pub struct PageTemplate {
    pub title: String,
    pub author: String,
    pub description: String,
    #[serde(serialize_with = "crate::content::page::serialize_date")]
    pub date: Option<NaiveDate>,
    pub tags: Vec<TagLink>,
    pub url: String,
    pub content: String,
}

/// A link to a tag page: `page.tags[]` entries.
#[derive(Serialize, Debug)]
pub struct TagLink {
    pub name: String,
    pub url: String,
}

impl TagLink {
    pub fn new(name: String) -> Self {
        TagLink {
            url: slug_url(&tag_slug(&name)),
            name,
        }
    }
}

/// A tag listed on the tag index: `tags[]` entries.
#[derive(Serialize, Debug)]
pub struct TagIndexEntry {
    pub name: String,
    pub url: String,
    pub page_count: usize,
}

impl TagIndexEntry {
    pub fn new(name: String, page_count: usize) -> Self {
        TagIndexEntry {
            url: slug_url(&tag_slug(&name)),
            name,
            page_count,
        }
    }
}

#[derive(Serialize)]
struct TagTemplate {
    name: String,
    url: String,
    pages: Vec<PageSummary>,
}

impl PageTemplate {
    fn add_content(self, html: String) -> Self {
        PageTemplate {
            title: self.title,
            author: self.author,
            description: self.description,
            date: self.date,
            tags: self.tags,
            url: self.url,
            content: html,
        }
    }
}

impl From<&Page> for PageTemplate {
    fn from(page: &Page) -> Self {
        PageTemplate {
            title: page.title.clone(),
            author: page.author.clone(),
            description: page.description.clone(),
            date: page.date,
            tags: page.tags.iter().cloned().map(TagLink::new).collect(),
            url: slug_url(&page.slug),
            content: String::new(),
        }
    }
}

#[derive(Serialize)]
struct SectionTemplate {
    slug: String,
    url: String,
    pages: Vec<PageSummary>,
    subsections: Vec<SectionLink>,
}

/// A link to a section: `section.subsections[]` entries.
#[derive(Serialize, Debug)]
pub struct SectionLink {
    pub slug: String,
    pub url: String,
}

impl SectionLink {
    pub fn new(slug: String) -> Self {
        SectionLink {
            url: slug_url(&slug),
            slug,
        }
    }
}

/// A top-level section listed on the home page: `home.sections[]` entries.
#[derive(Serialize, Debug)]
pub struct HomeSection {
    pub slug: String,
    pub url: String,
    pub page_count: usize,
}

impl HomeSection {
    pub fn new(slug: String, page_count: usize) -> Self {
        HomeSection {
            url: slug_url(&slug),
            slug,
            page_count,
        }
    }
}

#[derive(Serialize)]
struct HomeTemplate {
    recent: Vec<PageSummary>,
    sections: Vec<HomeSection>,
}

pub struct RenderItem {
    pub slug: String,
    /// Human-readable origin, used in collision errors.
    pub source: String,
    pub template: String,
    pub context: tera::Context,
    /// The page date for content page items (`None` for undated pages and
    /// every other item kind). The sitemap uses it for `<lastmod>`.
    pub page_date: Option<NaiveDate>,
}

pub fn load_templates(templates: &Path) -> Result<Tera, MangoError> {
    if !templates.is_dir() {
        let msg = format!(
            "the templates path '{}' is not a directory",
            templates.display()
        );
        return Err(MangoError::General(msg));
    }

    let template_path = templates.join("**/*.html");
    let template_glob = match template_path.to_str() {
        Some(t) => t,
        None => {
            return Err(MangoError::General("template directory not defined".into()));
        }
    };

    Tera::new(template_glob).map_err(MangoError::Template)
}

pub fn render_page(page: &Page, config: &SiteConfig) -> Result<RenderItem, MangoError> {
    use tera::Context;
    let mut context = Context::new();

    let html = to_html(&page.content);
    let page_template = PageTemplate::from(page).add_content(html);

    context.insert("page", &page_template);
    context.insert("config", config);
    let template = "page.html";
    Ok(RenderItem {
        slug: page.slug.clone(),
        source: format!("page '{}'", page.slug),
        template: template.into(),
        context,
        page_date: page.date,
    })
}

pub fn render_section_page(
    slug: String,
    pages: Vec<PageSummary>,
    subsections: Vec<SectionLink>,
    config: &SiteConfig,
) -> RenderItem {
    use tera::Context;
    let mut context = Context::new();
    let template = SectionTemplate {
        url: slug_url(&slug),
        slug,
        pages,
        subsections,
    };
    context.insert("section", &template);
    context.insert("config", config);

    RenderItem {
        source: format!("section index '{}'", template.slug),
        slug: template.slug,
        template: "section.html".into(),
        context,
        page_date: None,
    }
}

pub fn render_home_page(
    recent: Vec<PageSummary>,
    sections: Vec<HomeSection>,
    config: &SiteConfig,
) -> RenderItem {
    use tera::Context;
    let mut context = Context::new();
    context.insert("home", &HomeTemplate { recent, sections });
    context.insert("config", config);

    RenderItem {
        slug: String::new(),
        source: "home page".into(),
        template: "home.html".into(),
        context,
        page_date: None,
    }
}

/// The tag index item (`dist/tags/index.html`). `entries` should already be
/// sorted by name.
pub fn render_tag_index(entries: Vec<TagIndexEntry>, config: &SiteConfig) -> RenderItem {
    use tera::Context;
    let mut context = Context::new();
    context.insert("tags", &entries);
    context.insert("config", config);

    RenderItem {
        slug: "tags".into(),
        source: "tag index".into(),
        template: "tags.html".into(),
        context,
        page_date: None,
    }
}

/// One tag page item (`dist/tags/<name>/index.html`).
pub fn render_tag_page(name: String, pages: Vec<PageSummary>, config: &SiteConfig) -> RenderItem {
    use tera::Context;
    let mut context = Context::new();
    let slug = tag_slug(&name);
    let template = TagTemplate {
        url: slug_url(&slug),
        name,
        pages,
    };
    context.insert("tag", &template);
    context.insert("config", config);

    RenderItem {
        source: format!("tag page '{}'", template.name),
        slug,
        template: "tag.html".into(),
        context,
        page_date: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

    fn page_with(slug: &str, date: Option<&str>) -> Page {
        let fm = MangoFrontmatter {
            title: "Title".into(),
            author: "Author".into(),
            description: "d".into(),
            date: date.map(String::from),
            tags: None,
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = slug.into();
        page
    }

    // AC-5.1
    #[test]
    fn page_context_includes_description() {
        let fm = MangoFrontmatter {
            title: "Title".into(),
            author: "Author".into(),
            description: "my first post".into(),
            date: None,
            tags: None,
            draft: false,
        };
        let page = Page::new(fm, "# Hi".into(), PageType::General).unwrap();

        let item = render_page(&page, &SiteConfig::default()).unwrap();
        let ctx = item.context.get("page").expect("page context missing");
        assert_eq!(ctx["description"], "my first post");
    }

    // AC-1.5; AC-6.1 (batch 3): a real slug, so the source label is meaningful
    #[test]
    fn page_context_date_is_formatted_or_empty() {
        let config = SiteConfig::default();
        let dated = page_with("posts/dated", Some("2026-01-24"));
        let undated = page_with("posts/undated", None);

        let item = render_page(&dated, &config).unwrap();
        assert_eq!(item.context.get("page").unwrap()["date"], "2026-01-24");
        assert_eq!(item.source, "page 'posts/dated'");

        let item = render_page(&undated, &config).unwrap();
        assert_eq!(item.context.get("page").unwrap()["date"], "");
    }

    // AC-5.2
    #[test]
    fn page_context_includes_url() {
        let item = render_page(&page_with("posts/post_one", None), &SiteConfig::default()).unwrap();
        assert_eq!(item.context.get("page").unwrap()["url"], "/posts/post_one/");
    }

    // AC-1.10, AC-1.11
    #[test]
    fn page_context_includes_config() {
        let config = SiteConfig {
            title: Some("Site".into()),
            ..SiteConfig::default()
        };
        let item = render_page(&page_with("a", None), &config).unwrap();
        let ctx = item.context.get("config").expect("config context missing");
        assert_eq!(ctx["title"], "Site");
        assert!(ctx["author"].is_null(), "{ctx}");
        assert!(ctx["description"].is_null(), "{ctx}");
        assert!(ctx["base_url"].is_null(), "{ctx}");
        assert_eq!(ctx["recent_count"], 10);
    }

    // AC-1.11, AC-3.3, AC-3.4, AC-5.3
    #[test]
    fn section_context_includes_config_url_and_subsections() {
        let item = render_section_page(
            "a".into(),
            Vec::new(),
            vec![SectionLink::new("a/b".into())],
            &SiteConfig::default(),
        );
        assert_eq!(item.slug, "a");
        assert_eq!(item.source, "section index 'a'");
        let section = item.context.get("section").unwrap();
        assert_eq!(section["url"], "/a/");
        assert_eq!(section["pages"].as_array().unwrap().len(), 0);
        assert_eq!(section["subsections"][0]["slug"], "a/b");
        assert_eq!(section["subsections"][0]["url"], "/a/b/");
        assert_eq!(item.context.get("config").unwrap()["recent_count"], 10);
    }

    // AC-2.1, AC-1.11
    #[test]
    fn home_item_fields_and_context() {
        let item = render_home_page(
            Vec::new(),
            vec![HomeSection::new("posts".into(), 3)],
            &SiteConfig::default(),
        );
        assert_eq!(item.slug, "");
        assert_eq!(item.source, "home page");
        assert_eq!(item.template, "home.html");
        let home = item.context.get("home").unwrap();
        assert_eq!(home["recent"].as_array().unwrap().len(), 0);
        assert_eq!(home["sections"][0]["slug"], "posts");
        assert_eq!(home["sections"][0]["url"], "/posts/");
        assert_eq!(home["sections"][0]["page_count"], 3);
        assert!(item.context.get("config").is_some());
    }

    // AC-3.1 (batch 4)
    #[test]
    fn page_context_tags_are_links() {
        let fm = MangoFrontmatter {
            title: "Title".into(),
            author: "Author".into(),
            description: "d".into(),
            date: Some("2026-01-24".into()),
            tags: Some(vec![
                "static-site".into(),
                "blog".into(),
                "static-site".into(),
            ]),
            draft: false,
        };
        let mut page = Page::new(fm, String::new(), PageType::General).unwrap();
        page.slug = "posts/one".into();

        let item = render_page(&page, &SiteConfig::default()).unwrap();
        let tags = &item.context.get("page").unwrap()["tags"];
        assert_eq!(
            *tags,
            serde_json::json!([
                { "name": "static-site", "url": "/tags/static-site/" },
                { "name": "blog", "url": "/tags/blog/" },
            ])
        );
        assert_eq!(item.page_date, page.date);
    }

    // AC-2.2 (batch 4)
    #[test]
    fn tag_index_context_and_source() {
        let config = SiteConfig::default();
        let item = render_tag_index(
            vec![
                TagIndexEntry::new("blog".into(), 2),
                TagIndexEntry::new("rust".into(), 1),
            ],
            &config,
        );
        assert_eq!(item.slug, "tags");
        assert_eq!(item.source, "tag index");
        assert_eq!(item.template, "tags.html");
        assert_eq!(item.page_date, None);
        assert_eq!(
            *item.context.get("tags").unwrap(),
            serde_json::json!([
                { "name": "blog", "url": "/tags/blog/", "page_count": 2 },
                { "name": "rust", "url": "/tags/rust/", "page_count": 1 },
            ])
        );
        assert_eq!(item.context.get("config").unwrap()["recent_count"], 10);
    }

    // AC-2.3, AC-3.2 (batch 4)
    #[test]
    fn tag_page_context_and_source() {
        let page = page_with("posts/one", Some("2026-01-24"));
        let item = render_tag_page(
            "blog".into(),
            vec![PageSummary::from(&page)],
            &SiteConfig::default(),
        );
        assert_eq!(item.slug, "tags/blog");
        assert_eq!(item.source, "tag page 'blog'");
        assert_eq!(item.template, "tag.html");
        assert_eq!(item.page_date, None);
        let tag = item.context.get("tag").unwrap();
        assert_eq!(tag["name"], "blog");
        assert_eq!(tag["url"], "/tags/blog/");
        assert_eq!(
            tag["pages"],
            serde_json::json!([{
                "title": "Title",
                "date": "2026-01-24",
                "slug": "posts/one",
                "url": "/posts/one/",
            }])
        );
        assert!(tag["pages"][0].get("tags").is_none());
        assert!(item.context.get("config").is_some());
    }

    // AC-1.10
    #[test]
    fn null_config_title_uses_default_filter() {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "t.html",
            r#"{{ config.title | default(value="My Site") }}|{{ config.base_url }}|{% if config.author %}yes{% else %}no{% endif %}"#,
        )
        .unwrap();
        let item = render_home_page(Vec::new(), Vec::new(), &SiteConfig::default());

        let html = tera.render("t.html", &item.context).unwrap();
        assert_eq!(html, "My Site||no");
    }
}
