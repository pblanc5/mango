use std::{fs, path::Path};

use crate::{
    content::{
        frontmatter,
        page::{Page, PageType},
    },
    error::MangoError,
};

/// Loads every `.md` page under `path` (recursively). Any read or parse error
/// fails the whole load. Draft pages are parsed but not returned.
pub fn load(path: &Path) -> Result<Vec<Page>, MangoError> {
    if !path.is_dir() {
        let msg = format!("{} is not a directory", path.display());
        return Err(MangoError::General(msg));
    }

    let mut pages = Vec::new();
    traverse(path, path, &mut pages)?;

    Ok(pages.into_iter().filter(|page| !page.draft).collect())
}

fn traverse(site: &Path, dir: &Path, pages: &mut Vec<Page>) -> Result<(), MangoError> {
    for result in fs::read_dir(dir).map_err(|e| MangoError::io_at(dir, e))? {
        let entry = result.map_err(|e| MangoError::io_at(dir, e))?;
        let path = &entry.path();

        if let Some(ext) = path.extension()
            && ext == "md"
        {
            let content = fs::read_to_string(path).map_err(|e| MangoError::io_at(path, e))?;
            let (frontmatter, markdown) = frontmatter::parse(content).map_err(|e| match e {
                MangoError::Frontmatter(msg) => {
                    MangoError::Frontmatter(format!("{}: {msg}", path.display()))
                }
                other => other,
            })?;

            match frontmatter {
                Some(fm) => {
                    let mut page = Page::new(fm, markdown, PageType::General);
                    page.generate_slug(path, site)?;
                    pages.push(page);
                }

                None => {
                    let msg = format!(
                        "failed to generate frontmatter for page {}",
                        path.to_str().unwrap_or_default()
                    );
                    return Err(MangoError::Frontmatter(msg));
                }
            };
        }

        if path.is_dir() {
            traverse(site, path, pages)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/loader")
            .join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn page(title: &str, draft: bool) -> String {
        format!(
            "---\n{{\"title\": \"{title}\", \"author\": \"a\", \"description\": \"d\", \"draft\": {draft}}}\n---\nbody\n"
        )
    }

    // AC-1.1
    #[test]
    fn subdirectory_page_without_frontmatter_is_an_error_naming_the_file() {
        let site = fixture_dir("subdirectory_page_without_frontmatter");
        write_file(&site.join("posts/bad.md"), "# no frontmatter\n");
        write_file(&site.join("posts/good.md"), &page("Good", false));

        let err = load(&site).expect_err("missing frontmatter must fail the load");
        assert!(err.to_string().contains("bad.md"), "{err}");
    }

    // AC-1.2
    #[test]
    fn subdirectory_malformed_frontmatter_is_an_error_naming_the_file() {
        let site = fixture_dir("subdirectory_malformed_frontmatter");
        write_file(
            &site.join("posts/bad.md"),
            "---\n{not valid json\n---\nbody\n",
        );

        let serde_msg = serde_json::from_str::<serde_json::Value>("{not valid json")
            .unwrap_err()
            .to_string();

        let err = load(&site).expect_err("malformed frontmatter must fail the load");
        assert!(matches!(err, MangoError::Frontmatter(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("bad.md"), "{msg}");
        assert!(msg.contains(&serde_msg), "expected '{serde_msg}' in: {msg}");
    }

    // AC-1.3
    #[test]
    fn loads_pages_nested_two_levels_deep() {
        let site = fixture_dir("nested_two_levels");
        write_file(&site.join("a/b/page.md"), &page("Deep", false));

        let pages = load(&site).unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "a/b/page");
    }

    // AC-1.4
    #[test]
    fn non_directory_site_is_a_general_error() {
        let dir = fixture_dir("non_directory_site");
        let file = dir.join("not_a_dir.md");
        write_file(&file, &page("File", false));

        let err = load(&file).expect_err("a file is not a site directory");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert!(err.to_string().contains("not_a_dir.md"), "{err}");
    }

    // AC-3.1
    #[test]
    fn draft_pages_are_excluded() {
        let site = fixture_dir("draft_pages_are_excluded");
        write_file(&site.join("posts/published.md"), &page("Published", false));
        write_file(&site.join("posts/secret.md"), &page("Secret", true));

        let pages = load(&site).unwrap();
        let slugs: Vec<_> = pages.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(slugs, vec!["posts/published"]);
    }

    // AC-3.3
    #[test]
    fn malformed_draft_is_still_an_error() {
        let site = fixture_dir("malformed_draft");
        write_file(
            &site.join("posts/draft.md"),
            "---\n{\"title\": \"t\", \"draft\": true,\n---\nbody\n",
        );

        let err = load(&site).expect_err("malformed draft must still fail the load");
        assert!(matches!(err, MangoError::Frontmatter(_)), "{err:?}");
        assert!(err.to_string().contains("draft.md"), "{err}");
    }
}
