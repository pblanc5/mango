use crate::{
    content::page::Page,
    error::MangoError,
    render::template::{self, RenderItem},
};

pub fn build(pages: &[Page]) -> Result<Vec<RenderItem>, MangoError> {
    pages.iter().map(template::render_page).collect()
}
