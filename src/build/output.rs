use std::path::{Path, PathBuf};

use crate::{error::MangoError};

pub fn write(dist:  &Path, slug: String, content: String) -> Result<(), MangoError> {
    let path = get_final_path(dist, slug);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(MangoError::Io)?;
    }

    std::fs::write(path, content)
        .map_err(MangoError::Io)?;

    Ok(())
}

fn get_final_path(dist: &Path, slug: String) -> PathBuf {
    let mut path = dist.to_path_buf();

    if !slug.is_empty() {
        for segment in slug.split('/') {
            path.push(segment);
        }
    }

    path.push("index.html");
    path
}