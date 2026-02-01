use crate::{build::index::section::SectionIndex, render::template::{self, RenderItem}};

pub fn build(si: SectionIndex) -> Vec<RenderItem> {
    let mut items = Vec::new();
    for (slug, pages) in si.sections {
        let item = template::render_section_page(slug, pages);
        items.push(item);
    }

    items
}