use serde::Serialize;

use crate::content::page::Page;

#[derive(Serialize, Debug)]
pub struct PageSummary {
    pub title: String,
    pub date: String,
    pub slug: String,
}

impl From<&Page> for PageSummary {
    fn from(page: &Page) -> Self {
        PageSummary {
            title: page.title.clone(),
            date: page.date.clone(),
            slug: page.slug.clone(),
        }
    }
}
