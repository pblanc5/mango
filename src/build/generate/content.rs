use crate::{
    build::output::Output, config::SiteConfig, content::page::Page, error::MangoError,
    render::template,
};

pub fn build(pages: &[Page], config: &SiteConfig) -> Result<Vec<Output>, MangoError> {
    pages
        .iter()
        .map(|page| template::render_page(page, config))
        .collect()
}
