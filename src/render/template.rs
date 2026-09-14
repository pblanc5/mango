use std::path::Path;

use serde::Serialize;
use tera::Tera;

use crate::{
    content::{page::Page, summary::PageSummary},
    error::MangoError,
    render::markdown::to_html,
};

#[derive(Serialize, Default)]
pub struct PageTemplate {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: String,
    pub tags: Vec<String>,
    pub content: String,
}

impl PageTemplate {
    fn add_content(self, html: String) -> Self {
        PageTemplate {
            title: self.title,
            author: self.author,
            description: self.description,
            date: self.date,
            tags: self.tags,
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
            date: page.date.clone(),
            tags: page.tags.clone(),
            content: String::new(),
        }
    }
}

#[derive(Serialize)]
struct SectionTemplate {
    slug: String,
    pages: Vec<PageSummary>,
}

impl SectionTemplate {
    fn new(slug: String, pages: Vec<PageSummary>) -> Self {
        SectionTemplate { slug, pages }
    }
}

pub struct RenderItem {
    pub slug: String,
    pub template: String,
    pub context: tera::Context,
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

pub fn render_page(page: &Page) -> Result<RenderItem, MangoError> {
    use tera::Context;
    let mut context = Context::new();

    let html = to_html(&page.content);
    let page_template = PageTemplate::from(page).add_content(html);

    context.insert("page", &page_template);
    let template = "page.html";
    Ok(RenderItem {
        slug: page.slug.clone(),
        template: template.into(),
        context,
    })
}

pub fn render_section_page(slug: String, summaries: Vec<PageSummary>) -> RenderItem {
    use tera::Context;
    let mut context = Context::new();
    let template = SectionTemplate::new(slug, summaries);
    context.insert("section", &template);

    RenderItem {
        slug: template.slug,
        template: "section.html".into(),
        context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{frontmatter::MangoFrontmatter, page::PageType};

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
        let page = Page::new(fm, "# Hi".into(), PageType::General);

        let item = render_page(&page).unwrap();
        let ctx = item.context.get("page").expect("page context missing");
        assert_eq!(ctx["description"], "my first post");
    }
}
