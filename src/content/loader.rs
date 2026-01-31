use std::{fs, path::Path};

use crate::{content::{frontmatter, page::{Page, PageType}}, error::MBError};

pub fn load(path: &Path) -> Result<Vec<Page>, MBError> {
    let mut pages = Vec::new();

    match site_traversal(path, &mut pages) {
        Ok(_) => (),
        Err(e) => {
            return Err(e)
        }
    };
    
    Ok(pages)
}

fn site_traversal(path: &Path, pages: &mut Vec<Page>) -> Result<(), MBError> {
    if !path.is_dir() {
        let msg = format!("{} is not a directory", path.to_str().unwrap_or_else(|| ""));
        return Err(MBError::User(msg));
    }
    
    for result in fs::read_dir(path)? {
        let entry = result?;
        let path = &entry.path();

         if let Some(ext) = path.extension() && ext == "md" {
            let content = fs::read_to_string(path)?;
            let (frontmatter, markdown) = frontmatter::parse(content)?;

             match frontmatter {
                Some(fm) => {
                    let page= Page::new(fm, markdown, PageType::General);
                    pages.push(page);
                },
                None => {
                    return Err(MBError::Frontmatter("no frontmatter present".into()));
                }
            };                 
        }

        if path.is_dir() {
           let _ = site_traversal(path, pages);
        }
    }

    Ok(())
}