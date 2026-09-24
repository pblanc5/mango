use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tera::Tera;

use crate::{build::generate::assets::AssetFile, error::MangoError, render::template::RenderItem};

/// A fully rendered output file, held in memory until it is written.
/// `html` may also hold non-HTML contents (the feed and sitemap XML).
#[derive(Debug)]
pub struct RenderedFile {
    pub path: PathBuf,
    pub html: String,
}

/// A non-template output file with an explicit path, generated in Rust
/// (`feed.xml`, `sitemap.xml`).
pub struct GeneratedFile {
    /// Output path relative to the output folder.
    pub path: PathBuf,
    /// Human-readable origin, used in collision errors.
    pub source: String,
    pub contents: String,
}

/// Fails if two outputs would write the same file, or if one output would be
/// a file where another needs a directory (`dist/feed.xml` next to
/// `dist/feed.xml/index.html`). Render items are checked first, in the given
/// order, then generated files, then asset files, so the first source seen is
/// reported first. Runs before cleaning, so a conflict never loses the
/// previous output.
pub fn check_collisions<'a>(
    dist: &Path,
    items: impl IntoIterator<Item = &'a RenderItem>,
    files: &'a [GeneratedFile],
    assets: &'a [AssetFile],
) -> Result<(), MangoError> {
    let mut seen: HashMap<PathBuf, &str> = HashMap::new();
    let mut order: Vec<(PathBuf, &str)> = Vec::new();

    let outputs = items
        .into_iter()
        .map(|item| (item.slug.output_path(dist), item.source.as_str()))
        .chain(
            files
                .iter()
                .map(|file| (dist.join(&file.path), file.source.as_str())),
        )
        .chain(
            assets
                .iter()
                .map(|asset| (asset.dest.clone(), asset.label.as_str())),
        );

    for (path, source) in outputs {
        if let Some(first) = seen.get(&path) {
            let msg = format!(
                "output path '{}' would be written by both {} and {}",
                path.display(),
                first,
                source
            );
            return Err(MangoError::General(msg));
        }
        seen.insert(path.clone(), source);
        order.push((path, source));
    }

    for (path, source) in &order {
        for ancestor in path.ancestors().skip(1) {
            if ancestor == dist {
                break;
            }
            if let Some(file_source) = seen.get(ancestor) {
                let msg = format!(
                    "output path '{}' would be written as a file by {}, but {} needs it to be a directory for '{}'",
                    ancestor.display(),
                    file_source,
                    source,
                    path.display()
                );
                return Err(MangoError::General(msg));
            }
        }
    }

    Ok(())
}

/// Maps generated files to their final paths under `dist` without touching
/// the filesystem.
pub fn render_generated(dist: &Path, files: &[GeneratedFile]) -> Vec<RenderedFile> {
    files
        .iter()
        .map(|file| RenderedFile {
            path: dist.join(&file.path),
            html: file.contents.clone(),
        })
        .collect()
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
                path: item.slug.output_path(dist),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::slug::Slug;
    use std::fs;

    fn item(slug: &str, source: &str, template: &str) -> RenderItem {
        item_at(Slug::from_test_text(slug), source, template)
    }

    fn item_at(slug: Slug, source: &str, template: &str) -> RenderItem {
        let mut context = tera::Context::new();
        context.insert("slug", &slug);
        RenderItem {
            slug,
            source: source.into(),
            template: template.into(),
            context,
            page_date: None,
        }
    }

    fn generated(path: &str, source: &str) -> GeneratedFile {
        GeneratedFile {
            path: PathBuf::from(path),
            source: source.into(),
            contents: format!("<{source}/>"),
        }
    }

    fn asset(rel: &str) -> AssetFile {
        AssetFile {
            source: Path::new("assets-src").join(rel),
            dest: Path::new("dist").join("assets").join(rel),
            label: format!("asset '{rel}'"),
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

        let err = check_collisions(dist, pages.iter().chain(sections.iter()), &[], &[])
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
        check_collisions(
            Path::new("dist"),
            pages.iter().chain(sections.iter()),
            &[],
            &[],
        )
        .unwrap();
    }

    // AC-3.4
    #[test]
    fn distinct_paths_pass() {
        let pages = [page("a/one"), page("a/two"), page("b")];
        let sections = [section("a")];
        check_collisions(
            Path::new("dist"),
            pages.iter().chain(sections.iter()),
            &[],
            &[],
        )
        .unwrap();
    }

    // AC-2.1, AC-2.5 (batch 3)
    #[test]
    fn home_slug_maps_to_root_index() {
        let dist = Path::new("dist");
        let home = item_at(Slug::home(), "home page", "t.html");
        assert_eq!(home.slug.output_path(dist), dist.join("index.html"));

        let pages = [page("posts/one")];
        let sections = [section("posts")];
        check_collisions(
            dist,
            pages.iter().chain(sections.iter()).chain([&home]),
            &[],
            &[],
        )
        .unwrap();

        let other = item_at(Slug::home(), "other root", "t.html");
        let err =
            check_collisions(dist, [&home, &other], &[], &[]).expect_err("two root items collide");
        let msg = err.to_string();
        assert!(
            msg.contains(&dist.join("index.html").display().to_string()),
            "{msg}"
        );
        assert!(msg.contains("home page"), "{msg}");
    }

    // AC-5.1 (batch 4)
    #[test]
    fn generated_file_collides_with_render_item() {
        let dist = Path::new("dist");
        let items = [page("posts/one"), item("tags", "tag index", "tags.html")];

        // Distinct paths pass.
        let files = [
            generated("feed.xml", "RSS feed"),
            generated("sitemap.xml", "sitemap"),
        ];
        check_collisions(dist, &items, &files, &[]).unwrap();

        // A generated file at a render item's path: item named first.
        let clash = [generated("posts/one/index.html", "RSS feed")];
        let err = check_collisions(dist, &items, &clash, &[]).expect_err("file vs item");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        let path = dist.join("posts").join("one").join("index.html");
        assert_eq!(
            msg,
            format!(
                "Mango Error: output path '{}' would be written by both page 'posts/one' and RSS feed",
                path.display()
            )
        );

        // Two generated files at the same path.
        let twice = [
            generated("feed.xml", "RSS feed"),
            generated("feed.xml", "sitemap"),
        ];
        let err = check_collisions(dist, &items, &twice, &[]).expect_err("file vs file");
        let msg = err.to_string();
        assert!(
            msg.contains(&dist.join("feed.xml").display().to_string()),
            "{msg}"
        );
        assert!(msg.contains("both RSS feed and sitemap"), "{msg}");
    }

    #[test]
    fn file_vs_directory_conflict_is_detected() {
        let dist = Path::new("dist");
        let items = [page("feed.xml")];
        let files = [generated("feed.xml", "RSS feed")];

        let err = check_collisions(dist, &items, &files, &[])
            .expect_err("feed.xml is needed as both a file and a directory");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("'{}'", dist.join("feed.xml").display())),
            "{msg}"
        );
        assert!(
            msg.contains("written as a file by RSS feed, but page 'feed.xml'"),
            "{msg}"
        );
    }

    #[test]
    fn asset_conflicts_are_detected() {
        let dist = Path::new("dist");
        let assets = [asset("index.html"), asset("minimal/main.css")];

        // A section index next to an asset is fine.
        check_collisions(dist, &[section("assets/minimal")], &[], &assets).unwrap();

        // A page nested under an asset file.
        let err = check_collisions(dist, &[page("assets/minimal/main.css")], &[], &assets)
            .expect_err("page nested under an asset file");
        let msg = err.to_string();
        assert!(msg.contains("asset 'minimal/main.css'"), "{msg}");
        assert!(msg.contains("page 'assets/minimal/main.css'"), "{msg}");

        // A page written to the same path as an asset.
        let err = check_collisions(dist, &[page("assets")], &[], &assets)
            .expect_err("page on an asset path");
        assert!(
            err.to_string()
                .contains("both page 'assets' and asset 'index.html'"),
            "{err}"
        );
    }

    // AC-5.2 (batch 4)
    #[test]
    fn render_generated_maps_paths_without_touching_fs() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/render_generated_does_not_touch_fs");
        let _ = fs::remove_dir_all(&dist);

        let files = [
            generated("feed.xml", "RSS feed"),
            generated("sitemap.xml", "sitemap"),
        ];
        let rendered = render_generated(&dist, &files);

        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].path, dist.join("feed.xml"));
        assert_eq!(rendered[0].html, "<RSS feed/>");
        assert_eq!(rendered[1].path, dist.join("sitemap.xml"));
        assert_eq!(rendered[1].html, "<sitemap/>");
        assert!(!dist.exists(), "render_generated must not create files");
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
