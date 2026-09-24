use std::{
    fmt,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{content::tag::Tag, error::MangoError};

/// Where a page, section index, tag page, tag index or the home page lives:
/// the `/`-joined path under the site root, `""` for the home page.
///
/// A value only exists after validation (`from_content_path`) or as a
/// derivation from one that is already valid (`parent`, `tag_page`, the
/// fixed `home` and `tag_index` locations). It owns every slug rule: URL,
/// output path, parent section, top-level test and segments. Ordering is the
/// byte-wise order of the joined text, so `a-c` sorts before `a/b`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Slug(String);

impl Slug {
    /// The slug of a content file: its path relative to `site`, without the
    /// extension, with `/` separators. Every segment may only use ASCII
    /// letters, digits, `-`, `_` and `.`, and may not consist only of dots, so
    /// URLs, the feed and the sitemap need no encoding and no output lands
    /// outside its own folder.
    pub fn from_content_path(path: &Path, site: &Path) -> Result<Slug, MangoError> {
        let text = path
            .strip_prefix(site)
            .map_err(|_e| MangoError::General("unable to generate slug from path".into()))?
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");

        Slug::parse(text, path)
    }

    /// Validates `/`-joined slug text; `file` names the source in errors.
    fn parse(text: String, file: &Path) -> Result<Slug, MangoError> {
        for segment in text.split('/') {
            let allowed = !segment.is_empty()
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'));
            if !allowed {
                return Err(MangoError::General(format!(
                    "{}: invalid file name '{segment}': use only ASCII letters, digits, '-', '_' and '.'",
                    file.display()
                )));
            }

            if segment.bytes().all(|b| b == b'.') {
                return Err(MangoError::General(format!(
                    "{}: invalid file name '{segment}': a segment cannot consist only of dots",
                    file.display()
                )));
            }
        }

        Ok(Slug(text))
    }

    /// The home page: `dist/index.html`, URL `/`.
    pub fn home() -> Slug {
        Slug(String::new())
    }

    /// The tag index: `tags`.
    pub fn tag_index() -> Slug {
        Slug("tags".into())
    }

    /// A tag's page: `tags/<tag>`. Use `Tag::slug`.
    pub fn tag_page(tag: &Tag) -> Slug {
        Slug(format!("tags/{tag}"))
    }

    /// The enclosing section: everything before the last `/`. `None` for a
    /// top-level slug and for the home slug.
    pub fn parent(&self) -> Option<Slug> {
        self.0
            .rsplit_once('/')
            .map(|(parent, _)| Slug(parent.to_string()))
    }

    /// A slug directly under the site root (not the home slug itself).
    pub fn is_top_level(&self) -> bool {
        !self.0.is_empty() && !self.0.contains('/')
    }

    /// The `/`-separated segments; none for the home slug.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|_| !self.0.is_empty())
    }

    /// Root-relative URL: `/<slug>/`, or `/` for the home slug.
    pub fn url(&self) -> String {
        if self.0.is_empty() {
            "/".to_string()
        } else {
            format!("/{}/", self.0)
        }
    }

    /// The output file: `<dist>/<segments>/index.html`.
    pub fn output_path(&self, dist: &Path) -> PathBuf {
        let mut path = dist.to_path_buf();
        path.extend(self.segments());
        path.push("index.html");
        path
    }

    /// Test-only: a slug from `/`-joined text, validated like a content file
    /// path. Panics on invalid text.
    #[cfg(test)]
    pub(crate) fn from_test_text(text: &str) -> Slug {
        Slug::parse(text.to_string(), Path::new(text))
            .unwrap_or_else(|e| panic!("invalid test slug '{text}': {e}"))
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slug(path: &str) -> Result<Slug, MangoError> {
        Slug::from_content_path(Path::new(path), Path::new("site"))
    }

    const CHARSET_MESSAGE: &str = "use only ASCII letters, digits, '-', '_' and '.'";
    const DOTS_MESSAGE: &str = "a segment cannot consist only of dots";

    // AC-arch-3.1.1
    #[test]
    fn generate_slug_strips_site_prefix_and_extension() {
        assert_eq!(
            slug("site/posts/post_one.md").unwrap().to_string(),
            "posts/post_one"
        );
    }

    #[test]
    fn generate_slug_fails_when_path_is_outside_site() {
        let result = slug("elsewhere/post.md");
        assert!(matches!(result, Err(MangoError::General(_))));
    }

    // AC-arch-3.1.7
    #[test]
    fn generate_slug_outside_site_message() {
        let err = slug("elsewhere/post.md").expect_err("outside the site");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert_eq!(
            err.to_string(),
            "Mango Error: unable to generate slug from path"
        );
    }

    #[test]
    fn generate_slug_accepts_safe_names_and_rejects_others() {
        for ok in ["site/posts/post_one.md", "site/A-b_c/v1.2.md"] {
            slug(ok).unwrap_or_else(|e| panic!("{ok}: {e}"));
        }

        for (bad, segment) in [
            ("site/my posts/one.md", "my posts"),
            ("site/héllo.md", "héllo"),
            ("site/a+b.md", "a+b"),
        ] {
            let err = slug(bad).expect_err(bad);
            assert!(matches!(err, MangoError::General(_)), "{bad}: {err:?}");
            let msg = err.to_string();
            assert!(msg.contains(bad), "{msg}");
            assert!(
                msg.contains(&format!("invalid file name '{segment}'")),
                "{msg}"
            );
        }
    }

    // AC-arch-3.1.2
    #[test]
    fn generate_slug_error_message_is_exact() {
        let err = slug("site/a+b.md").expect_err("'+' is not allowed");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        assert_eq!(
            err.to_string(),
            "Mango Error: site/a+b.md: invalid file name 'a+b': use only ASCII letters, digits, '-', '_' and '.'"
        );
    }

    // AC-arch-3.7.7
    #[test]
    fn backslash_in_stem_becomes_separator() {
        assert_eq!(slug("site/a\\b.md").unwrap().to_string(), "a/b");
    }

    // AC-arch-3.1.6
    #[cfg(unix)]
    #[test]
    fn non_utf8_file_name_is_rejected_lossily() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let path = Path::new(OsStr::from_bytes(b"site/\xff.md"));
        let err = Slug::from_content_path(path, Path::new("site"))
            .expect_err("invalid UTF-8 is replaced and rejected");
        assert!(matches!(err, MangoError::General(_)), "{err:?}");
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("invalid file name '\u{FFFD}': {CHARSET_MESSAGE}")),
            "{msg}"
        );
    }

    // AC-arch-3.5.2, AC-arch-3.7.4, AC-arch-3.8.2: an empty segment gets the
    // character-set message, never the dot-only one.
    #[test]
    fn empty_and_bad_segments_get_the_charset_message() {
        for (text, segment) in [
            ("a//b", ""),
            ("", ""),
            ("/a", ""),
            ("a/", ""),
            ("a b", "a b"),
        ] {
            let err = Slug::parse(text.to_string(), Path::new("site/x.md")).expect_err(text);
            assert!(matches!(err, MangoError::General(_)), "{text}: {err:?}");
            let msg = err.to_string();
            assert_eq!(
                msg,
                format!("Mango Error: site/x.md: invalid file name '{segment}': {CHARSET_MESSAGE}"),
                "{text}"
            );
            assert!(!msg.contains(DOTS_MESSAGE), "{text}: {msg}");
        }
    }

    // AC-arch-3.5.2, AC-arch-3.7.4, AC-arch-3.8.1, AC-arch-3.8.2
    #[test]
    fn dot_only_segments_get_their_own_message() {
        for (text, segment) in [
            (".", "."),
            ("..", ".."),
            ("...", "..."),
            ("posts/.", "."),
            ("posts/..", ".."),
            ("a/./b", "."),
        ] {
            let err = Slug::parse(text.to_string(), Path::new("site/x.md")).expect_err(text);
            assert!(matches!(err, MangoError::General(_)), "{text}: {err:?}");
            assert_eq!(
                err.to_string(),
                format!("Mango Error: site/x.md: invalid file name '{segment}': {DOTS_MESSAGE}"),
                "{text}"
            );
        }

        // Dots next to other characters are still fine.
        for ok in [".a", "a.", "v1.2", "a/.b/c"] {
            Slug::parse(ok.to_string(), Path::new("site/x.md"))
                .unwrap_or_else(|e| panic!("{ok}: {e}"));
        }
    }

    // AC-arch-3.8.1: through a real content path, top-level and nested.
    #[test]
    fn dot_only_file_names_are_rejected() {
        for (path, segment) in [
            ("site/...md", ".."),
            // `with_extension("")` turns `..md` into `..`, not `.`.
            ("site/..md", ".."),
            ("site/posts/...md", ".."),
        ] {
            let err = slug(path).expect_err(path);
            assert_eq!(
                err.to_string(),
                format!("Mango Error: {path}: invalid file name '{segment}': {DOTS_MESSAGE}")
            );
        }
    }

    // AC-5.1, AC-5.3, AC-5.5; AC-arch-3.5.3
    #[test]
    fn slug_url_is_root_relative_with_trailing_slash() {
        assert_eq!(
            Slug::from_test_text("posts/post_one").url(),
            "/posts/post_one/"
        );
        assert_eq!(Slug::from_test_text("posts").url(), "/posts/");
        assert_eq!(Slug::home().url(), "/");
        assert_eq!(Slug::tag_index().url(), "/tags/");
    }

    // AC-arch-3.5.3
    #[test]
    fn output_path_is_index_html_under_segments() {
        let dist = Path::new("dist");
        assert_eq!(Slug::home().output_path(dist), dist.join("index.html"));
        assert_eq!(
            Slug::from_test_text("a/b/c").output_path(dist),
            dist.join("a").join("b").join("c").join("index.html")
        );
        assert_eq!(
            Slug::tag_index().output_path(dist),
            dist.join("tags").join("index.html")
        );
    }

    // AC-arch-3.5.3 (was extract_parent_slug_uses_last_separator)
    #[test]
    fn parent_uses_last_separator() {
        let parent = |s: &str| Slug::from_test_text(s).parent().map(|p| p.to_string());
        assert_eq!(parent("a/b/c"), Some("a/b".to_string()));
        assert_eq!(parent("posts/one"), Some("posts".to_string()));
        assert_eq!(parent("about"), None);
        assert_eq!(Slug::home().parent(), None);
    }

    // AC-arch-3.5.3
    #[test]
    fn top_level_and_segments() {
        assert!(Slug::from_test_text("posts").is_top_level());
        assert!(!Slug::from_test_text("posts/one").is_top_level());
        assert!(!Slug::home().is_top_level());

        let segments = |s: &Slug| s.segments().map(String::from).collect::<Vec<_>>();
        assert_eq!(segments(&Slug::from_test_text("a/b/c")), ["a", "b", "c"]);
        assert_eq!(segments(&Slug::from_test_text("about")), ["about"]);
        assert!(segments(&Slug::home()).is_empty());
    }

    // AC-arch-3.5.4
    #[test]
    fn orders_bytewise_on_joined_text() {
        assert!(Slug::from_test_text("a-c") < Slug::from_test_text("a/b"));
        assert!(Slug::from_test_text("a") < Slug::from_test_text("a/b"));
        assert!(Slug::home() < Slug::from_test_text("a"));
    }

    // AC-arch-3.5.5
    #[test]
    fn displays_and_serializes_as_plain_string() {
        let s = Slug::from_test_text("posts/one");
        assert_eq!(s.to_string(), "posts/one");
        assert_eq!(
            serde_json::to_value(&s).unwrap(),
            serde_json::json!("posts/one")
        );
        assert_eq!(
            serde_json::to_value(Slug::home()).unwrap(),
            serde_json::json!("")
        );
    }
}
