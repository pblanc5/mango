use std::path::{Path, PathBuf};
use tera::Tera;

use crate::{error::MangoError, render::template::RenderItem};

pub fn write(tera: &Tera, dist:  &Path, render_items: Vec<RenderItem>) -> Result<(), MangoError> {

    for item in render_items {
        let path = get_final_path(dist, &item.slug);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(MangoError::Io)?;
        }

        let content= tera.render(&item.template, &item.context)?;

        std::fs::write(path, content)
            .map_err(MangoError::Io)?;
    }

    Ok(())
}

fn get_final_path(dist: &Path, slug: &str) -> PathBuf {
    let mut path = dist.to_path_buf();

    if !slug.is_empty() {
        for segment in slug.split('/') {
            path.push(segment);
        }
    }

    path.push("index.html");
    path
}

