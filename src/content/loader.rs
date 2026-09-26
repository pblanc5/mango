use std::{fs, path::Path};

use crate::{
    content::{
        frontmatter,
        page::{Page, PageType},
    },
    error::MangoError,
};

/// Loads every markdown page under `path` (recursively). Any read or parse
/// error fails the whole load. Draft pages are parsed but not returned.
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

        // `metadata` follows the link, `is_symlink` below does not, so a
        // symlink to a folder is rejected here instead of being descended
        // into — which used to re-walk the whole site under the link,
        // silently publishing duplicate pages when it pointed at an
        // ancestor. On `Err` (a dangling link, an `ELOOP` chain, an entry
        // that vanished after `read_dir`) the entry is skipped: this is where
        // `is_dir()`/`is_file()` used to swallow the error, and turning it
        // into a build error is a deliberate strictness change, tracked as
        // RISK-5 in specs/_system/backlog.md. Skipping stops the recursion
        // just as well as failing would.
        let Ok(target) = fs::metadata(path) else {
            continue;
        };

        if target.is_dir() {
            if path.is_symlink() {
                let msg = format!(
                    "content folder '{}' is a symlink to a directory ('{}'): symlinked folders are not followed; replace it with a real folder",
                    site_relative(site, path),
                    path.display()
                );
                return Err(MangoError::General(msg));
            }

            traverse(site, path, pages)?;
            continue;
        }

        if !target.is_file() || !is_markdown(path) {
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
                let page =
                    Page::new(fm, markdown, PageType::General, path, site).map_err(with_path)?;
                pages.push(page);
            }

            None => {
                let msg = format!(
                    "{}: missing frontmatter: the file must start with a '---' line, a JSON object and a closing '---' line",
                    path.display()
                );
                return Err(MangoError::Frontmatter(msg));
            }
        };
    }

    Ok(())
}

/// Labels a path for error messages: relative to the site folder, with `/`
/// separators, falling back to the full path if it is not under `site`.
fn site_relative(site: &Path, path: &Path) -> String {
    path.strip_prefix(site)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// `.md` and `.markdown` files, in any letter case, are pages.
fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
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
        let msg = err.to_string();
        assert!(msg.contains("bad.md"), "{msg}");
        assert!(msg.contains("missing frontmatter"), "{msg}");
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
        assert_eq!(pages[0].slug.to_string(), "a/b/page");
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
        let slugs: Vec<_> = pages.iter().map(|p| p.slug.to_string()).collect();
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
        let slugs: Vec<_> = pages.iter().map(|p| p.slug.to_string()).collect();
        assert_eq!(slugs, vec!["notes.md/inner"]);
    }

    #[test]
    fn markdown_extensions_in_any_case_are_pages() {
        let site = fixture_dir("markdown_extensions_in_any_case");
        for name in ["a.md", "b.MD", "c.markdown", "d.MarkDown"] {
            write_file(&site.join(name), &page(name, false));
        }
        write_file(&site.join("e.txt"), "not a page");
        write_file(&site.join("f.mdx"), "not a page");

        let mut slugs: Vec<_> = load(&site)
            .unwrap()
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        slugs.sort();
        assert_eq!(slugs, ["a", "b", "c", "d"]);
    }

    #[test]
    fn invalid_file_name_is_an_error_naming_the_file_even_for_drafts() {
        let site = fixture_dir("invalid_file_name");
        write_file(&site.join("my posts/draft.md"), &page("Draft", true));

        let err = load(&site).expect_err("a space in a folder name must fail the load");
        let msg = err.to_string();
        assert!(msg.contains("draft.md"), "{msg}");
        assert!(msg.contains("invalid file name 'my posts'"), "{msg}");
    }

    // AC-arch-3.1.3
    #[test]
    fn frontmatter_error_wins_over_invalid_file_name() {
        for (i, extra) in ["\"tags\": [\"Rust\"]", "\"date\": \"2026-02-30\""]
            .iter()
            .enumerate()
        {
            let site = fixture_dir(&format!(
                "frontmatter_error_wins_over_invalid_file_name_{i}"
            ));
            write_file(
                &site.join("my posts/x.md"),
                &format!(
                    "---\n{{\"title\": \"t\", \"author\": \"a\", \"description\": \"d\", \"draft\": false, {extra}}}\n---\nbody\n"
                ),
            );

            let err = load(&site).expect_err("both errors present");
            assert!(
                matches!(err, MangoError::Frontmatter(_)),
                "{extra}: {err:?}"
            );
            let msg = err.to_string();
            let value = if i == 0 { "'Rust'" } else { "'2026-02-30'" };
            assert!(msg.contains(value), "{msg}");
            assert!(!msg.contains("invalid file name"), "{msg}");
        }
    }

    // AC-risk-4.1.2 [baseline] A missing required key fails the load with a
    // `Frontmatter` error that starts with the file path.
    #[test]
    fn missing_required_key_is_a_frontmatter_error_naming_the_file() {
        let site = fixture_dir("missing_required_key");
        let file = site.join("posts").join("no_title.md");
        write_file(
            &file,
            "---\n{\"author\": \"a\", \"description\": \"d\", \"draft\": false}\n---\nbody\n",
        );

        let err = load(&site).expect_err("a missing required key must fail the load");
        assert!(matches!(err, MangoError::Frontmatter(_)), "{err:?}");
        let msg = err.to_string();
        let prefix = format!("Mango Frontmatter Error: {}: ", file.display());
        assert!(msg.starts_with(&prefix), "{msg}");
    }

    // AC-risk-4.5.3, AC-risk-4.4.2
    #[test]
    fn key_errors_win_over_date_tag_and_file_name_errors() {
        let site = fixture_dir("key_errors_win_over_date_tag_and_file_name_errors");
        let file = site.join("my posts").join("x.md");
        write_file(
            &file,
            "---\n{\"title\": \"t\", \"author\": \"a\", \"description\": \"d\", \"draft\": false, \"date\": \"2026-02-30\", \"tags\": [\"Rust\"], \"tag\": []}\n---\nbody\n",
        );

        let err = load(&site).expect_err("key errors must fail the load");
        assert!(matches!(err, MangoError::Frontmatter(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains(&file.display().to_string()), "{msg}");
        assert!(msg.contains("unknown 'tag'"), "{msg}");
        assert!(!msg.contains("invalid date"), "{msg}");
        assert!(!msg.contains("'Rust'"), "{msg}");
        assert!(!msg.contains("invalid file name"), "{msg}");
    }

    // AC-risk-4.3.3
    #[test]
    fn unknown_key_in_draft_is_still_an_error() {
        let site = fixture_dir("unknown_key_in_draft");
        write_file(&site.join("posts/published.md"), &page("Published", false));
        write_file(
            &site.join("posts/draft.md"),
            "---\n{\"title\": \"t\", \"author\": \"a\", \"description\": \"d\", \"draft\": true, \"dates\": \"2026-01-24\"}\n---\nbody\n",
        );

        let err = load(&site).expect_err("an unknown key in a draft must fail the load");
        assert!(matches!(err, MangoError::Frontmatter(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("draft.md"), "{msg}");
        assert!(msg.contains("unknown 'dates'"), "{msg}");
    }

    // AC-arch-3.8.1
    #[test]
    fn dot_only_file_name_is_rejected_even_for_drafts() {
        let site = fixture_dir("dot_only_file_name_is_rejected_even_for_drafts");
        write_file(&site.join("posts/...md"), &page("Draft", true));

        let err = load(&site).expect_err("a dot-only file name must fail the load");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("...md"), "{msg}");
        assert!(
            msg.contains("invalid file name '..': a segment cannot consist only of dots"),
            "{msg}"
        );
    }

    // AC-risk-6.3.4
    #[cfg(unix)]
    #[test]
    fn backslash_in_folder_name_is_rejected_even_for_drafts() {
        let site = fixture_dir("backslash_in_folder_name_is_rejected_even_for_drafts");
        let file = site.join("x\\y").join("p.md");
        write_file(&file, &page("Draft", true));

        let err = load(&site).expect_err("a backslash in a folder name must fail the load");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains(&file.display().to_string()), "{msg}");
        assert!(msg.contains("invalid file name 'x\\y'"), "{msg}");
    }

    // AC-risk-6.3.5
    #[cfg(unix)]
    #[test]
    fn frontmatter_error_wins_over_backslash_in_name() {
        for (label, extra, expected) in [
            ("tag", "\"tags\": [\"Rust\"]", "'Rust'"),
            ("date", "\"date\": \"2026-02-30\"", "'2026-02-30'"),
        ] {
            let site = fixture_dir(&format!(
                "frontmatter_error_wins_over_backslash_in_name_{label}"
            ));
            write_file(
                &site.join("a\\b.md"),
                &format!(
                    "---\n{{\"title\": \"t\", \"author\": \"a\", \"description\": \"d\", \"draft\": false, {extra}}}\n---\nbody\n"
                ),
            );

            let err = load(&site).expect_err("both errors present");
            assert!(
                matches!(err, MangoError::Frontmatter(_)),
                "{extra}: {err:?}"
            );
            let msg = err.to_string();
            assert!(msg.contains(expected), "{msg}");
            assert!(!msg.contains("invalid file name"), "{msg}");
        }
    }

    // AC-risk-6.3.7: a `\` in the site folder's own path is not checked, and
    // the filesystem accepts it in a folder name.
    #[cfg(unix)]
    #[test]
    fn load_accepts_backslash_in_site_folder_path() {
        let site = fixture_dir("load_accepts_backslash_in_site_folder_path").join("my\\site");
        write_file(&site.join("posts").join("one.md"), &page("One", false));

        let slugs: Vec<_> = load(&site)
            .unwrap()
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        assert_eq!(slugs, ["posts/one"]);
    }

    // AC-11.1, AC-11.3
    #[cfg(unix)]
    #[test]
    fn load_rejects_symlinked_directory() {
        use std::os::unix::fs::symlink;

        for (test_name, link_rel) in [
            ("load_rejects_symlinked_directory_root", "vendor"),
            ("load_rejects_symlinked_directory_nested", "posts/vendor"),
        ] {
            let site = fixture_dir(test_name);
            write_file(&site.join("posts/real.md"), &page("Real", false));
            write_file(&site.join("shared/linked.md"), &page("Linked", false));
            let link = site.join(link_rel);
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            symlink(site.join("shared"), &link).unwrap();

            let err = load(&site).expect_err("a symlinked content folder must fail the load");
            assert!(matches!(err, MangoError::General(_)), "{err:?}");
            let msg = err.to_string();
            assert!(
                msg.contains(&format!("content folder '{link_rel}'")),
                "{msg}"
            );
            assert!(msg.contains("symlink to a directory"), "{msg}");
            // The link's own path as walked, not its target.
            assert!(msg.contains(&link.display().to_string()), "{msg}");
            assert!(msg.ends_with("replace it with a real folder"), "{msg}");
        }
    }

    // AC-11.2
    #[cfg(unix)]
    #[test]
    fn load_rejects_symlink_loop_to_ancestor() {
        use std::os::unix::fs::symlink;

        let site = fixture_dir("load_rejects_symlink_loop_to_ancestor");
        write_file(&site.join("posts/real.md"), &page("Real", false));
        // Pointing at the site folder itself: this used to be followed,
        // yielding ~40 duplicate copies of the site with no error at all.
        // The call must return an error instead.
        symlink(&site, site.join("loop")).unwrap();

        let err = load(&site).expect_err("a symlink to an ancestor must fail the load");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(msg.contains("content folder 'loop'"), "{msg}");
        assert!(msg.contains("symlink to a directory"), "{msg}");
    }

    // AC-11.4
    #[cfg(unix)]
    #[test]
    fn load_follows_symlinked_markdown_file() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("load_follows_symlinked_markdown_file");
        let site = dir.join("site");
        write_file(&site.join("posts/real.md"), &page("Real", false));
        write_file(&dir.join("shared/post.md"), &page("Shared", false));
        symlink(dir.join("shared/post.md"), site.join("posts/linked.md")).unwrap();

        let mut slugs: Vec<_> = load(&site)
            .unwrap()
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        slugs.sort();
        // The slug comes from the link path, not from the link target.
        assert_eq!(slugs, ["posts/linked", "posts/real"]);
    }

    // AC-11.5
    #[cfg(unix)]
    #[test]
    fn load_ignores_symlinked_non_markdown_file() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("load_ignores_symlinked_non_markdown_file");
        let site = dir.join("site");
        write_file(&site.join("posts/real.md"), &page("Real", false));
        write_file(&dir.join("shared/notes.txt"), "not a page");
        symlink(dir.join("shared/notes.txt"), site.join("posts/notes.txt")).unwrap();

        let slugs: Vec<_> = load(&site)
            .unwrap()
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        assert_eq!(slugs, ["posts/real"]);
    }

    // AC-11.8
    #[cfg(unix)]
    #[test]
    fn load_accepts_symlinked_site_root() {
        use std::os::unix::fs::symlink;

        let dir = fixture_dir("load_accepts_symlinked_site_root");
        let real = dir.join("shared-content");
        write_file(&real.join("posts/one.md"), &page("One", false));
        let link = dir.join("site");
        symlink(&real, &link).unwrap();

        let slugs: Vec<_> = load(&link)
            .unwrap()
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        assert_eq!(slugs, ["posts/one"]);
    }

    // AC-11.6 [baseline] An unresolvable symlink is ignored, not an error.
    #[cfg(unix)]
    #[test]
    fn load_ignores_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let site = fixture_dir("load_ignores_dangling_symlink");
        write_file(&site.join("posts/real.md"), &page("Real", false));
        let missing = site.join("posts/nowhere");
        symlink(&missing, site.join("posts/broken.md")).unwrap();
        symlink(&missing, site.join("posts/broken.txt")).unwrap();

        let mut slugs: Vec<_> = load(&site)
            .expect("a dangling symlink must not fail the load")
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        slugs.sort();
        assert_eq!(slugs, ["posts/real"]);
    }

    // AC-11.7 [baseline] A looping link chain is ignored, not an error.
    #[cfg(unix)]
    #[test]
    fn load_ignores_symlink_loop_chain() {
        use std::os::unix::fs::symlink;

        let site = fixture_dir("load_ignores_symlink_loop_chain");
        write_file(&site.join("real.md"), &page("Real", false));
        symlink("b.md", site.join("a.md")).unwrap();
        symlink("a.md", site.join("b.md")).unwrap();

        let mut slugs: Vec<_> = load(&site)
            .expect("a symlink loop chain must not fail the load")
            .into_iter()
            .map(|p| p.slug.to_string())
            .collect();
        slugs.sort();
        assert_eq!(slugs, ["real"]);
    }
}
