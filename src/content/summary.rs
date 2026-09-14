use chrono::NaiveDate;
use serde::Serialize;

use crate::content::page::{Page, serialize_date};

#[derive(Serialize, Debug)]
pub struct PageSummary {
    pub title: String,
    #[serde(serialize_with = "serialize_date")]
    pub date: Option<NaiveDate>,
    pub slug: String,
}

impl From<&Page> for PageSummary {
    fn from(page: &Page) -> Self {
        PageSummary {
            title: page.title.clone(),
            date: page.date,
            slug: page.slug.clone(),
        }
    }
}
