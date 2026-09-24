use std::{
    collections::HashMap,
    fmt,
    path::{Path, PathBuf},
};

use chrono::NaiveDate;
use tera::Tera;

use crate::{
    content::{slug::Slug, tag::Tag},
    error::MangoError,
};

/// One thing a build puts in the output folder: what it is, and how its bytes
/// are produced. Its location, collision label and sitemap entry are all
/// derived from `kind`, so they cannot disagree with each other.
#[derive(Debug)]
pub struct Output {
    pub kind: OutputKind,
    pub body: Body,
}

/// What an output is. Each variant carries exactly what identifies an output
/// of that kind; `Display` gives the label used in collision errors.
#[derive(Debug, PartialEq, Eq)]
pub enum OutputKind {
    /// A content page and its frontmatter date (the sitemap's `<lastmod>`).
    Page {
        slug: Slug,
        date: Option<NaiveDate>,
    },
    /// A section index.
    Section(Slug),
    Home,
    TagIndex,
    Tag(Tag),
    /// `feed.xml`.
    Feed,
    /// `sitemap.xml`.
    Sitemap,
    /// An asset copy: `folder` is the assets folder's name (the folder under
    /// the output folder it is copied into), `rel` the path relative to it.
    Asset {
        folder: PathBuf,
        rel: PathBuf,
    },
}

/// How an output's bytes are produced.
#[derive(Debug)]
pub enum Body {
    /// Rendered through a Tera template during planning.
    Template {
        name: &'static str,
        context: tera::Context,
    },
    /// Generated in memory (the feed and sitemap XML).
    Text(String),
    /// Copied from this source file during commit.
    Copy(PathBuf),
}

/// An output after rendering: its full path and what `write` puts there.
#[derive(Debug)]
pub struct RenderedOutput {
    pub path: PathBuf,
    pub contents: Contents,
}

/// A rendered output's contents: text to write, or a file to copy. Templates
/// are already rendered, so no template can fail after planning.
#[derive(Debug, PartialEq, Eq)]
pub enum Contents {
    Text(String),
    Copy(PathBuf),
}

impl Output {
    /// The file this output is written to under `dist`.
    pub fn path(&self, dist: &Path) -> PathBuf {
        match &self.kind {
            OutputKind::Page { slug, .. } | OutputKind::Section(slug) => slug.output_path(dist),
            OutputKind::Home => Slug::home().output_path(dist),
            OutputKind::TagIndex => Slug::tag_index().output_path(dist),
            OutputKind::Tag(tag) => tag.slug().output_path(dist),
            OutputKind::Feed => dist.join("feed.xml"),
            OutputKind::Sitemap => dist.join("sitemap.xml"),
            OutputKind::Asset { folder, rel } => dist.join(folder).join(rel),
        }
    }
}

/// Test-only accessors for the body. Each panics on a different body.
#[cfg(test)]
impl Output {
    pub(crate) fn context(&self) -> &tera::Context {
        match &self.body {
            Body::Template { context, .. } => context,
            other => panic!("not a template output: {other:?}"),
        }
    }

    pub(crate) fn template_name(&self) -> &str {
        match &self.body {
            Body::Template { name, .. } => name,
            other => panic!("not a template output: {other:?}"),
        }
    }

    pub(crate) fn text(&self) -> &str {
        match &self.body {
            Body::Text(text) => text,
            other => panic!("not a text output: {other:?}"),
        }
    }
}

impl fmt::Display for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputKind::Page { slug, .. } => write!(f, "page '{slug}'"),
            OutputKind::Section(slug) => write!(f, "section index '{slug}'"),
            OutputKind::Home => f.write_str("home page"),
            OutputKind::TagIndex => f.write_str("tag index"),
            OutputKind::Tag(tag) => write!(f, "tag page '{tag}'"),
            OutputKind::Feed => f.write_str("RSS feed"),
            OutputKind::Sitemap => f.write_str("sitemap"),
            OutputKind::Asset { rel, .. } => {
                write!(f, "asset '{}'", rel.to_string_lossy().replace('\\', "/"))
            }
        }
    }
}

/// Fails if two outputs would write the same file, or if one output would be
/// a file where another needs a directory (`dist/feed.xml` next to
/// `dist/feed.xml/index.html`). Outputs are checked in the given order, so the
/// earlier output is reported first. Runs before cleaning, so a conflict never
/// loses the previous output.
pub fn check_collisions(dist: &Path, outputs: &[Output]) -> Result<(), MangoError> {
    let mut seen: HashMap<PathBuf, &OutputKind> = HashMap::new();
    let mut order: Vec<(PathBuf, &OutputKind)> = Vec::new();

    for output in outputs {
        let path = output.path(dist);
        let source = &output.kind;
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

/// Renders every template output to a string and resolves every path under
/// `dist`, without touching the filesystem. The first template error in list
/// order fails the whole batch.
pub fn render(
    tera: &Tera,
    dist: &Path,
    outputs: Vec<Output>,
) -> Result<Vec<RenderedOutput>, MangoError> {
    outputs
        .into_iter()
        .map(|output| {
            let path = output.path(dist);
            let contents = match output.body {
                Body::Template { name, context } => Contents::Text(tera.render(name, &context)?),
                Body::Text(text) => Contents::Text(text),
                Body::Copy(source) => Contents::Copy(source),
            };
            Ok(RenderedOutput { path, contents })
        })
        .collect()
}

/// Writes text outputs and copies asset files, in order, creating parent
/// folders. Any failure fails the build.
pub fn write(outputs: &[RenderedOutput]) -> Result<(), MangoError> {
    for output in outputs {
        if let Some(parent) = output.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| MangoError::io_at(parent, e))?;
        }

        match &output.contents {
            Contents::Text(text) => {
                std::fs::write(&output.path, text)
                    .map_err(|e| MangoError::io_at(&output.path, e))?;
            }
            // Names the source even when the destination is the problem
            // (RISK-7).
            Contents::Copy(source) => {
                std::fs::copy(source, &output.path).map_err(|e| MangoError::io_at(source, e))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn template_output(kind: OutputKind, name: &'static str, slug: &str) -> Output {
        let mut context = tera::Context::new();
        context.insert("slug", slug);
        Output {
            kind,
            body: Body::Template { name, context },
        }
    }

    fn page(slug: &str) -> Output {
        let kind = OutputKind::Page {
            slug: Slug::from_test_text(slug),
            date: None,
        };
        template_output(kind, "t.html", slug)
    }

    fn section(slug: &str) -> Output {
        template_output(
            OutputKind::Section(Slug::from_test_text(slug)),
            "t.html",
            slug,
        )
    }

    fn home() -> Output {
        template_output(OutputKind::Home, "t.html", "")
    }

    fn tag_index() -> Output {
        template_output(OutputKind::TagIndex, "tags.html", "tags")
    }

    fn feed() -> Output {
        Output {
            kind: OutputKind::Feed,
            body: Body::Text("<RSS feed/>".into()),
        }
    }

    fn sitemap() -> Output {
        Output {
            kind: OutputKind::Sitemap,
            body: Body::Text("<sitemap/>".into()),
        }
    }

    fn asset(rel: &str) -> Output {
        Output {
            kind: OutputKind::Asset {
                folder: PathBuf::from("assets"),
                rel: PathBuf::from(rel),
            },
            body: Body::Copy(Path::new("assets-src").join(rel)),
        }
    }

    // AC-3.1
    #[test]
    fn collision_names_path_and_both_sources() {
        let dist = Path::new("dist");

        let err = check_collisions(dist, &[page("posts"), section("posts")])
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
        let outputs = [page("posts/index"), page("posts/one"), section("posts")];
        check_collisions(Path::new("dist"), &outputs).unwrap();
    }

    // AC-3.4
    #[test]
    fn distinct_paths_pass() {
        let outputs = [page("a/one"), page("a/two"), page("b"), section("a")];
        check_collisions(Path::new("dist"), &outputs).unwrap();
    }

    // AC-2.1, AC-2.5 (batch 3)
    #[test]
    fn home_slug_maps_to_root_index() {
        let dist = Path::new("dist");
        assert_eq!(home().path(dist), dist.join("index.html"));

        check_collisions(dist, &[page("posts/one"), section("posts"), home()]).unwrap();

        let err = check_collisions(dist, &[home(), home()]).expect_err("two root items collide");
        let msg = err.to_string();
        assert!(
            msg.contains(&dist.join("index.html").display().to_string()),
            "{msg}"
        );
        assert!(msg.contains("home page"), "{msg}");
    }

    // AC-5.1 (batch 4): an HTML output and a feed or sitemap can no longer
    // share a path (locations derive from the kind), so the cross-kind exact
    // case is an HTML output against an asset.
    #[test]
    fn generated_file_collides_with_render_item() {
        let dist = Path::new("dist");

        // Distinct paths pass.
        check_collisions(dist, &[page("posts/one"), tag_index(), feed(), sitemap()]).unwrap();

        // A non-template output at an HTML output's path: HTML output named first.
        let err = check_collisions(dist, &[page("assets"), asset("index.html")])
            .expect_err("page vs asset");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert_eq!(
            err.to_string(),
            format!(
                "Mango Error: output path '{}' would be written by both page 'assets' and asset 'index.html'",
                dist.join("assets").join("index.html").display()
            )
        );

        // The same generated output twice.
        let err =
            check_collisions(dist, &[page("posts/one"), feed(), feed()]).expect_err("file vs file");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert_eq!(
            err.to_string(),
            format!(
                "Mango Error: output path '{}' would be written by both RSS feed and RSS feed",
                dist.join("feed.xml").display()
            )
        );
    }

    #[test]
    fn file_vs_directory_conflict_is_detected() {
        let dist = Path::new("dist");

        let err = check_collisions(dist, &[page("feed.xml"), feed()])
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

        // A section index next to an asset is fine.
        check_collisions(
            dist,
            &[
                section("assets/minimal"),
                asset("index.html"),
                asset("minimal/main.css"),
            ],
        )
        .unwrap();

        // A page nested under an asset file.
        let err = check_collisions(
            dist,
            &[
                page("assets/minimal/main.css"),
                asset("index.html"),
                asset("minimal/main.css"),
            ],
        )
        .expect_err("page nested under an asset file");
        let msg = err.to_string();
        assert!(msg.contains("asset 'minimal/main.css'"), "{msg}");
        assert!(msg.contains("page 'assets/minimal/main.css'"), "{msg}");

        // A page written to the same path as an asset.
        let err = check_collisions(
            dist,
            &[
                page("assets"),
                asset("index.html"),
                asset("minimal/main.css"),
            ],
        )
        .expect_err("page on an asset path");
        assert!(
            err.to_string()
                .contains("both page 'assets' and asset 'index.html'"),
            "{err}"
        );
    }

    // AC-arch-2.7.3: every kind's collision label, derived from the kind alone.
    #[test]
    fn labels_derive_from_kind() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 24);
        let cases = [
            (
                OutputKind::Page {
                    slug: Slug::from_test_text("posts/one"),
                    date: None,
                },
                "page 'posts/one'",
            ),
            (
                OutputKind::Page {
                    slug: Slug::from_test_text("posts/one"),
                    date,
                },
                "page 'posts/one'",
            ),
            (
                OutputKind::Section(Slug::from_test_text("posts")),
                "section index 'posts'",
            ),
            (OutputKind::Home, "home page"),
            (OutputKind::TagIndex, "tag index"),
            (
                OutputKind::Tag(Tag::parse("rust".into()).unwrap()),
                "tag page 'rust'",
            ),
            (OutputKind::Feed, "RSS feed"),
            (OutputKind::Sitemap, "sitemap"),
            (
                OutputKind::Asset {
                    folder: PathBuf::from("assets"),
                    rel: Path::new("css").join("main.css"),
                },
                "asset 'css/main.css'",
            ),
        ];
        for (kind, label) in cases {
            assert_eq!(kind.to_string(), label, "{kind:?}");
        }
    }

    // AC-arch-2.6.1: every kind's output location, derived from the kind alone.
    #[test]
    fn paths_derive_from_kind() {
        let dist = Path::new("dist");
        let at = |kind: OutputKind| {
            Output {
                kind,
                body: Body::Text(String::new()),
            }
            .path(dist)
        };

        assert_eq!(
            at(OutputKind::Page {
                slug: Slug::from_test_text("posts/one"),
                date: NaiveDate::from_ymd_opt(2026, 1, 24),
            }),
            dist.join("posts").join("one").join("index.html")
        );
        assert_eq!(
            at(OutputKind::Section(Slug::from_test_text("posts"))),
            dist.join("posts").join("index.html")
        );
        assert_eq!(at(OutputKind::Home), dist.join("index.html"));
        assert_eq!(
            at(OutputKind::TagIndex),
            dist.join("tags").join("index.html")
        );
        assert_eq!(
            at(OutputKind::Tag(Tag::parse("rust".into()).unwrap())),
            dist.join("tags").join("rust").join("index.html")
        );
        assert_eq!(at(OutputKind::Feed), dist.join("feed.xml"));
        assert_eq!(at(OutputKind::Sitemap), dist.join("sitemap.xml"));
        assert_eq!(
            at(OutputKind::Asset {
                folder: PathBuf::from("static"),
                rel: Path::new("css").join("main.css"),
            }),
            dist.join("static").join("css").join("main.css")
        );
    }

    // AC-5.2 (batch 4)
    #[test]
    fn render_generated_maps_paths_without_touching_fs() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/render_generated_does_not_touch_fs");
        let _ = fs::remove_dir_all(&dist);

        let rendered = render(&Tera::default(), &dist, vec![feed(), sitemap()]).unwrap();

        assert_eq!(rendered.len(), 2);
        assert_eq!(rendered[0].path, dist.join("feed.xml"));
        assert_eq!(rendered[0].contents, Contents::Text("<RSS feed/>".into()));
        assert_eq!(rendered[1].path, dist.join("sitemap.xml"));
        assert_eq!(rendered[1].contents, Contents::Text("<sitemap/>".into()));
        assert!(!dist.exists(), "render must not create files");
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

        let files = render(&tera, &dist, vec![page("posts/one"), section("posts")]).unwrap();

        assert_eq!(files.len(), 2);
        assert_eq!(
            files[0].path,
            dist.join("posts").join("one").join("index.html")
        );
        assert_eq!(files[0].contents, Contents::Text("<p>posts/one</p>".into()));
        assert_eq!(files[1].path, dist.join("posts").join("index.html"));
        assert_eq!(files[1].contents, Contents::Text("<p>posts</p>".into()));
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

        let broken = template_output(
            OutputKind::Page {
                slug: Slug::from_test_text("broken"),
                date: None,
            },
            "bad.html",
            "broken",
        );
        let result = render(&tera, Path::new("dist"), vec![page("ok"), broken]);
        assert!(matches!(result, Err(MangoError::Template(_))));
    }

    #[test]
    fn write_creates_parent_dirs_and_files() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/write_round_trip");
        let _ = fs::remove_dir_all(&dist);

        let files = [RenderedOutput {
            path: dist.join("a/b/index.html"),
            contents: Contents::Text("<p>hi</p>".into()),
        }];
        write(&files).unwrap();

        assert_eq!(
            fs::read_to_string(dist.join("a/b/index.html")).unwrap(),
            "<p>hi</p>"
        );
    }

    // AC-arch-2.5.3: a write failure names the file it could not write, or
    // the parent folder it could not create.
    #[test]
    fn write_failure_names_the_path() {
        let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/unit-fixtures/output/write_failure_names_the_path");
        let _ = fs::remove_dir_all(&dist);

        // (a) The target path is an existing directory.
        let target = dist.join("taken");
        fs::create_dir_all(&target).unwrap();
        let files = [RenderedOutput {
            path: target.clone(),
            contents: Contents::Text("x".into()),
        }];
        match write(&files) {
            Err(MangoError::IoPath { path, .. }) => assert_eq!(path, target),
            other => panic!("expected IoPath, got {other:?}"),
        }

        // (b) The parent folder is an existing file.
        let parent = dist.join("file");
        fs::write(&parent, "x").unwrap();
        let files = [RenderedOutput {
            path: parent.join("index.html"),
            contents: Contents::Text("x".into()),
        }];
        match write(&files) {
            Err(MangoError::IoPath { path, .. }) => assert_eq!(path, parent),
            other => panic!("expected IoPath, got {other:?}"),
        }
    }
}
