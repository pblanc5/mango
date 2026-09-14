use crate::{
    build::index::section::SectionIndex,
    render::template::{self, RenderItem},
};

pub fn build(si: SectionIndex) -> Vec<RenderItem> {
    let mut items = Vec::new();

    si.sections.into_iter().for_each(|(s, p)| {
        let item = template::render_section_page(s, p);
        items.push(item);
    });

    items
}
