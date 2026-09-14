use std::path::Path;

use serde::Serialize;

use crate::{content::frontmatter::MangoFrontmatter, error::MangoError};

#[derive(Serialize, Debug)]
pub enum PageType {
    General,
}

#[derive(Serialize, Debug)]
pub struct Page {
    pub title: String,
    pub author: String,
    pub description: String,
    pub date: String,
    pub slug: String,
    pub tags: Vec<String>,
    pub content: String,
    pub draft: bool,
    pub kind: PageType,
}

impl Page {
    pub fn new(fm: MangoFrontmatter, content: String, kind: PageType) -> Self {
        Page {
            title: fm.title,
            author: fm.author,
            description: fm.description,
            date: fm.date.unwrap_or_default(),
            slug: String::new(),
            tags: fm.tags.unwrap_or_default(),
            content,
            draft: fm.draft,
            kind,
        }
    }

    pub fn generate_slug(&mut self, path: &Path, site: &Path) -> Result<(), MangoError> {
        let slug = path
            .strip_prefix(site)
            .map_err(|_e| MangoError::General("unable to generate slug from path".into()))?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        self.slug = slug;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frontmatter() -> MangoFrontmatter {
        MangoFrontmatter {
            title: "Title".into(),
            author: "Author".into(),
            description: "Description".into(),
            date: None,
            tags: None,
            draft: false,
        }
    }

    #[test]
    fn new_defaults_missing_date_and_tags() {
        let page = Page::new(frontmatter(), "content".into(), PageType::General);

        assert_eq!(page.date, "");
        assert!(page.tags.is_empty());
        assert_eq!(page.slug, "");
    }

    #[test]
    fn generate_slug_strips_site_prefix_and_extension() {
        let mut page = Page::new(frontmatter(), String::new(), PageType::General);
        page.generate_slug(Path::new("site/posts/post_one.md"), Path::new("site"))
            .unwrap();

        assert_eq!(page.slug, "posts/post_one");
    }

    #[test]
    fn generate_slug_fails_when_path_is_outside_site() {
        let mut page = Page::new(frontmatter(), String::new(), PageType::General);
        let result = page.generate_slug(Path::new("elsewhere/post.md"), Path::new("site"));

        assert!(matches!(result, Err(MangoError::General(_))));
    }
}
