use std::path::Path;

use serde::Serialize;

use crate::{content::frontmatter::MangoFrontmatter, error::MangoError};

#[derive(Serialize, Debug)]
pub enum PageType {
    General
}

#[derive(Serialize, Debug)]
pub struct Page {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub draft: bool,
    pub kind: PageType
}

impl Page {
    pub fn new(fm: MangoFrontmatter, content: String, kind: PageType) -> Self {
        Page { 
            title: fm.title, 
            author: fm.author,
            description: fm.description,
            date: fm.date.unwrap_or_default(),
            slug: String::new(),
            tags: fm.tags.unwrap_or_default(), 
            content,
            draft: fm.draft,
            kind
        }
    }

    pub fn generate_slug(&mut self, path: &Path, site: &Path) -> Result<(), MangoError> {
        let slug = path.strip_prefix(site)
            .map_err(|_e| MangoError::General("unable to generate slug from path".into()))?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        self.slug = slug;
        Ok(())
    }
}

