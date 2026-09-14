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

        if path.is_dir() {
            traverse(site, path, pages)?;
            continue;
        }

        if !path.is_file() || path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }

        let with_path = |e: MangoError| match e {
            MangoError::Frontmatter(msg) => {
                MangoError::Frontmatter(format!("{}: {msg}", path.display()))
            }
            other => other,
        };

        let content = fs::read_to_string(path).map_err(|e| MangoError::io_at(path, e))?;
        let (frontmatter, markdown) = frontmatter::parse(content).map_err(with_path)?;

        match frontmatter {
            Some(fm) => {
                let mut page = Page::new(fm, markdown, PageType::General).map_err(with_path)?;
                page.generate_slug(path, site)?;
                pages.push(page);
            }

            None => {
                let msg = format!("failed to generate frontmatter for page {}", path.display());
                return Err(MangoError::Frontmatter(msg));
            }
        };
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

    fn dated_page(title: &str, date: &str, draft: bool) -> String {
        format!(
            "---\n{{\"title\": \"{title}\", \"author\": \"a\", \"description\": \"d\", \"date\": {date}, \"draft\": {draft}}}\n---\nbody\n"
        )
    }

    // AC-1.1
    #[test]
    fn valid_date_is_parsed() {
        let site = fixture_dir("valid_date_is_parsed");
        write_file(
            &site.join("posts/one.md"),
            &dated_page("One", "\"2026-01-24\"", false),
        );
        write_file(
            &site.join("posts/leap.md"),
            &dated_page("Leap", "\"2024-02-29\"", false),
        );

        let mut pages = load(&site).unwrap();
        pages.sort_by(|a, b| a.slug.cmp(&b.slug));
        assert_eq!(pages[0].date, chrono::NaiveDate::from_ymd_opt(2024, 2, 29));
        assert_eq!(pages[1].date, chrono::NaiveDate::from_ymd_opt(2026, 1, 24));
    }

    fn assert_date_error(test_name: &str, value: &str, draft: bool) {
        let site = fixture_dir(test_name);
        let json_value = serde_json::to_string(value).unwrap();
        write_file(
            &site.join("posts/bad_date.md"),
            &dated_page("Bad", &json_value, draft),
        );

        let err = load(&site).expect_err("invalid date must fail the load");
        assert!(
            matches!(err, MangoError::Frontmatter(_)),
            "{value}: {err:?}"
        );
        let msg = err.to_string();
        assert!(msg.contains("bad_date.md"), "{value}: {msg}");
        assert!(msg.contains(&format!("'{value}'")), "{value}: {msg}");
    }

    // AC-1.2
    #[test]
    fn invalid_date_format_is_frontmatter_error_naming_file_and_value() {
        let bad = [
            "2026/01/24",
            "2026-1-24",
            "26-01-24",
            "2026-01-24T10:00:00",
            "+2026-01-24",
            " 2026-01-24",
            "",
        ];
        for (i, value) in bad.iter().enumerate() {
            assert_date_error(&format!("invalid_date_format_{i}"), value, false);
        }
    }

    // AC-1.3
    #[test]
    fn impossible_date_is_frontmatter_error_naming_file_and_value() {
        for (i, value) in ["2026-02-30", "2026-13-01"].iter().enumerate() {
            assert_date_error(&format!("impossible_date_{i}"), value, false);
        }
    }

    // AC-1.4
    #[test]
    fn missing_or_null_date_is_undated() {
        let site = fixture_dir("missing_or_null_date_is_undated");
        write_file(&site.join("posts/missing.md"), &page("Missing", false));
        write_file(
            &site.join("posts/null.md"),
            &dated_page("Null", "null", false),
        );

        let pages = load(&site).unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages.iter().all(|p| p.date.is_none()));
    }

    // AC-1.6
    #[test]
    fn invalid_date_in_draft_is_still_an_error() {
        assert_date_error("invalid_date_in_draft", "2026-02-30", true);
    }

    // AC-6.1
    #[test]
    fn directory_named_md_is_traversed() {
        let site = fixture_dir("directory_named_md_is_traversed");
        write_file(&site.join("notes.md/inner.md"), &page("Inner", false));

        let pages = load(&site).unwrap();
        let slugs: Vec<_> = pages.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(slugs, vec!["notes.md/inner"]);
    }
}
