use std::path::Path;

use serde::Serialize;
use tera::Tera;

use crate::{content::page::Page, error::MangoError, render::markdown::to_html};

#[derive(Serialize)]
pub struct PageTemplate {
    pub title: String,
    pub author: String,
    pub date: String,
    pub tags: Vec<String>,
    pub content: String,
}

pub fn render_page(page: &Page, templates: &Path) -> Result<String, MangoError> {
    let template_path = templates.join("**/*.html");
    let template_glob = match template_path.to_str() {
        Some(t) => t,
        None => {
            return Err(MangoError::General("template directory not defined".into()));
        }
    };

    let tera = Tera::new(template_glob)
        .map_err(MangoError::Template)?;

    use tera::Context;
    let mut context = Context::new();

    let page_template = PageTemplate {
        title: page.title.clone(),
        author: page.author.clone(),
        date: page.date.clone(),
        tags: page.tags.clone(),
        content: to_html(page.content.clone())
    };

    context.insert("page", &page_template);
    let template = "page.html";
    let html = tera.render(template, &context)
        .map_err(MangoError::Template)?;

    Ok(html)
}