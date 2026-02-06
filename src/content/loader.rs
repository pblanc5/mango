use std::{fs, path::Path};

use crate::{content::{frontmatter, page::{Page, PageType}}, error::MangoError};

pub fn load(path: &Path) -> Result<Vec<Page>, MangoError> {
    let mut pages = Vec::new();

    match site_traversal(path, path, &mut pages) {
        Ok(_) => (),
        Err(e) => {
            return Err(e)
        }
    };
    
    Ok(pages)
}

fn site_traversal(site: &Path, parent: &Path, pages: &mut Vec<Page>) -> Result<(), MangoError> {
    if !site.is_dir() {
        let msg = format!("{} is not a directory", site.to_str().unwrap_or(""));
        return Err(MangoError::General(msg));
    }
    
    for result in fs::read_dir(parent)? {
        let entry = result?;
        let path = &entry.path();

         if let Some(ext) = path.extension() && ext == "md" {
            let content = fs::read_to_string(path)?;
            let (frontmatter, markdown) = frontmatter::parse(content)?;

             match frontmatter {
                Some(fm) => {
                    let mut page= Page::new(fm, markdown, PageType::General);
                    page.generate_slug(path, site)?;
                    pages.push(page);
                },

                None => {
                    let msg = format!("failed to generate frontmatter for page {}", path.to_str().unwrap_or_default());
                    return Err(MangoError::Frontmatter(msg));
                }
            };                 
        }

        if path.is_dir() {
           let _ = site_traversal(site,path, pages);
        }
    }

    Ok(())
}