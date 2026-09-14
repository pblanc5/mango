use crate::{
    config::SiteConfig,
    content::page::Page,
    error::MangoError,
    render::template::{self, RenderItem},
};

pub fn build(pages: &[Page], config: &SiteConfig) -> Result<Vec<RenderItem>, MangoError> {
    pages
        .iter()
        .map(|page| template::render_page(page, config))
        .collect()
}
