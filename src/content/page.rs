use serde::Serialize;

use crate::content::frontmatter::MBFrontmatter;

#[derive(Serialize, Debug)]
pub enum PageType {
    General
}

#[derive(Serialize, Debug)]
pub struct Page {
    pub title: String,
    pub author: String,
    pub date: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub draft: bool,
    pub kind: PageType
}

impl Page {
    pub fn new(fm: MBFrontmatter, content: String, kind: PageType) -> Self {
        Page { 
            title: fm.title, 
            author: fm.author, 
            date: fm.date.unwrap_or_else(|| String::new()), 
            slug: fm.slug.unwrap_or_else(|| String::new()), 
            tags: fm.tags.unwrap_or_else(|| Vec::new()), 
            content: content,
            draft: fm.draft,
            kind: kind
        }
    }
}