use std::collections::HashMap;

use crate::{content::{page::Page, summary::PageSummary}};

pub type SectionSlug = String;

#[derive(Default)]
pub struct SectionIndex {
    pub sections: HashMap<SectionSlug, Vec<PageSummary>>
}

impl SectionIndex {
    fn new() -> Self {
        SectionIndex::default()
    }
}

pub fn build_section_index(pages: &Vec<Page>) -> SectionIndex {
    let mut si = SectionIndex::new();

    let summaries = pages
        .iter()
        .map(|p | PageSummary::from(p))
        .collect::<Vec<_>>();


    for summary in summaries {
        if let Some(parent) = extract_parent_slug(&summary.slug) {
            si.sections.entry(parent)
                .or_default()
                .push(summary);
        }
    }

    si
}

fn extract_parent_slug(slug: &str) -> Option<String> {
    slug.rsplit_once('/')
        .map(|(p, _)| p.to_string())
        
}
