use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tera::Tera;

use crate::{error::MangoError, render::template::RenderItem};

/// A fully rendered output file, held in memory until it is written.
pub struct RenderedFile {
    pub path: PathBuf,
    pub html: String,
}

/// Fails if two render items would write the same output file. Items are
/// checked in the given order, so the first source seen is reported first.
pub fn check_collisions<'a>(
    dist: &Path,
    items: impl IntoIterator<Item = &'a RenderItem>,
) -> Result<(), MangoError> {
    let mut seen: HashMap<PathBuf, &str> = HashMap::new();

    for item in items {
        let path = get_final_path(dist, &item.slug);
        if let Some(first) = seen.get(&path) {
            let msg = format!(
                "output path '{}' would be written by both {} and {}",
                path.display(),
                first,
                item.source
            );
            return Err(MangoError::General(msg));
        }
        seen.insert(path, &item.source);
    }

    Ok(())
}

/// Renders every item to a string without touching the filesystem. Any
/// render error fails the whole batch.
pub fn render(
    tera: &Tera,
    dist: &Path,
    items: &[RenderItem],
) -> Result<Vec<RenderedFile>, MangoError> {
    items
        .iter()
        .map(|item| {
            Ok(RenderedFile {
                path: get_final_path(dist, &item.slug),
                html: tera.render(&item.template, &item.context)?,
            })
        })
        .collect()
}

pub fn write(files: &[RenderedFile]) -> Result<(), MangoError> {
    for file in files {
        if let Some(parent) = file.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| MangoError::io_at(parent, e))?;
        }

        std::fs::write(&file.path, &file.html).map_err(|e| MangoError::io_at(&file.path, e))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn item(slug: &str, source: &str, template: &str) -> RenderItem {
        let mut context = tera::Context::new();
        context.insert("slug", slug);
        RenderItem {
            slug: slug.into(),
            source: source.into(),
            template: template.into(),
            context,
        }
    }

    fn page(slug: &str) -> RenderItem {
        item(slug, &format!("page '{slug}'"), "t.html")
    }

    fn section(slug: &str) -> RenderItem {
        item(slug, &format!("section index '{slug}'"), "t.html")
    }

    // AC-3.1
    #[test]
    fn collision_names_path_and_both_sources() {
        let dist = Path::new("dist");
        let pages = [page("posts")];
        let sections = [section("posts")];

        let err = check_collisions(dist, pages.iter().chain(sections.iter()))
            .expect_err("page and section index share an output path");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        let expected_path = dist.join("posts").join("index.html");
        assert!(msg.contains(&expected_path.display().to_string()), "{msg}");
        assert!(msg.contains("page 'posts'"), "{msg}");
        assert!(msg.contains("section index 'posts'"), "{msg}");
    }

    // AC-3.3
    #[test]
    fn index_md_is_not_a_collision() {
        let pages = [page("posts/index"), page("posts/one")];
        let sections = [section("posts")];
        check_collisions(Path::new("dist"), pages.iter().chain(sections.iter())).unwrap();
    }

    // AC-3.4
    #[test]
    fn distinct_paths_pass() {
        let pages = [page("a/one"), page("a/two"), page("b")];
        let sections = [section("a")];
        check_collisions(Path::new("dist"), pages.iter().chain(sections.iter())).unwrap();
    }

    // AC-2.1, AC-2.5 (batch 3)
    #[test]
    fn home_slug_maps_to_root_index() {
        let dist = Path::new("dist");
        let home = item("", "home page", "t.html");
        assert_eq!(get_final_path(dist, &home.slug), dist.join("index.html"));

        let pages = [page("posts/one")];
        let sections = [section("posts")];
        check_collisions(dist, pages.iter().chain(sections.iter()).chain([&home])).unwrap();

        let other = item("", "other root", "t.html");
        let err = check_collisions(dist, [&home, &other]).expect_err("two root items collide");
        let msg = err.to_string();
        assert!(
            msg.contains(&dist.join("index.html").display().to_string()),
            "{msg}"
        );
        assert!(msg.contains("home page"), "{msg}");
    }

    // AC-9.1
    #[test]
    fn render_returns_paths_and_html_without_touching_fs() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/render_does_not_touch_fs");
        let _ = fs::remove_dir_all(&dist);

        let mut tera = Tera::default();
        // Autoescape would turn '/' in slugs into '&#x2F;'.
        tera.autoescape_on(vec![]);
        tera.add_raw_template("t.html", "<p>{{ slug }}</p>")
            .unwrap();

        let files = render(&tera, &dist, &[page("posts/one"), section("posts")]).unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(
            files[0].path,
            dist.join("posts").join("one").join("index.html")
        );
        assert_eq!(files[0].html, "<p>posts/one</p>");
        assert_eq!(files[1].path, dist.join("posts").join("index.html"));
        assert_eq!(files[1].html, "<p>posts</p>");
        assert!(!dist.exists(), "render must not create files");
    }

    // AC-9.1
    #[test]
    fn render_error_returns_template_error() {
        let mut tera = Tera::default();
        tera.add_raw_template("t.html", "<p>{{ slug }}</p>")
            .unwrap();
        tera.add_raw_template("bad.html", "{{ missing.value }}")
            .unwrap();

        let items = [page("ok"), item("broken", "page 'broken'", "bad.html")];
        let result = render(&tera, Path::new("dist"), &items);
        assert!(matches!(result, Err(MangoError::Template(_))));
    }

    #[test]
    fn write_creates_parent_dirs_and_files() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/write_round_trip");
        let _ = fs::remove_dir_all(&dist);

        let files = [RenderedFile {
            path: dist.join("a/b/index.html"),
            html: "<p>hi</p>".into(),
        }];
        write(&files).unwrap();

        assert_eq!(
            fs::read_to_string(dist.join("a/b/index.html")).unwrap(),
            "<p>hi</p>"
        );
    }
}
