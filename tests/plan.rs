//! In-process pipeline tests: they call the library (`mango::plan`,
//! `mango::commit`, `mango::clean`) directly instead of spawning the binary,
//! so a plan can be inspected before — or without — being written out. The
//! binary's contract (exit status, stderr, files on disk) stays in
//! `tests/build.rs`.
//!
//! Using only the public API is deliberate: it is the same surface the future
//! dev server (FEAT-1) consumes, so these tests double as the check that the
//! re-exports in `src/lib.rs` are sufficient.
//!
//! No test changes the current directory: they run in parallel and each one
//! works inside its own folder under cargo's integration tmpdir.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use mango::{BuildOptions, BuildPlan, MangoError, PlannedOutput, clean, commit, plan};

/// Fresh, per-test scratch directory under cargo's integration-test tmpdir.
fn temp_dir(test_name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(test_name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_file(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A page's JSON frontmatter plus a one-line body.
fn frontmatter(title: &str, date: Option<&str>, tags_json: Option<&str>, draft: bool) -> String {
    let mut fields = format!(
        "\"title\": \"{title}\", \"author\": \"tester\", \"description\": \"desc\", \"draft\": {draft}"
    );
    if let Some(date) = date {
        fields.push_str(&format!(", \"date\": \"{date}\""));
    }
    if let Some(tags) = tags_json {
        fields.push_str(&format!(", \"tags\": {tags}"));
    }
    format!("---\n{{{fields}}}\n---\n# {title}\n")
}

struct Project {
    dir: PathBuf,
    site: PathBuf,
    templates: PathBuf,
    assets: PathBuf,
    out: PathBuf,
}

/// A project with a minimal five-template theme whose output is one line per
/// listed item, so listing order can be asserted as an exact string instead of
/// scraped out of the fixture theme.
fn project(test_name: &str) -> Project {
    let dir = temp_dir(test_name);
    let project = Project {
        site: dir.join("site"),
        templates: dir.join("templates"),
        assets: dir.join("assets"),
        out: dir.join("dist"),
        dir,
    };

    write_file(&project.templates.join("page.html"), "{{ page.title }}");
    write_file(
        &project.templates.join("section.html"),
        "{% for p in section.pages %}{{ p.title }} {{ p.slug | safe }}\n{% endfor %}",
    );
    write_file(
        &project.templates.join("home.html"),
        "{% for p in home.recent %}{{ p.title }}\n{% endfor %}",
    );
    write_file(
        &project.templates.join("tags.html"),
        "{% for t in tags %}{{ t.name }}\n{% endfor %}",
    );
    write_file(
        &project.templates.join("tag.html"),
        "{% for p in tag.pages %}{{ p.title }}\n{% endfor %}",
    );
    write_file(&project.assets.join("style.css"), "body { margin: 0 }\n");

    project
}

/// `config: None` resolves to `mango.json` in the current directory, which the
/// repo root does not have, so the defaults apply.
fn options(project: &Project, config: Option<&Path>) -> BuildOptions {
    BuildOptions {
        site: project.site.clone(),
        templates: project.templates.clone(),
        assets: project.assets.clone(),
        output: project.out.clone(),
        config: config.map(Path::to_path_buf),
    }
}

/// Every path the plan would write, relative to the output folder, in order.
fn paths(plan: &BuildPlan) -> Vec<PathBuf> {
    plan.outputs()
        .map(|output| match output {
            PlannedOutput::File { path, .. } | PlannedOutput::Copy { path, .. } => {
                path.to_path_buf()
            }
        })
        .collect()
}

/// The planned contents of one rendered or generated file.
fn file<'a>(plan: &'a BuildPlan, rel: &str) -> &'a str {
    plan.outputs()
        .find_map(|output| match output {
            PlannedOutput::File { path, contents } if path == Path::new(rel) => Some(contents),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{rel} is not in the plan, got: {:?}", paths(plan)))
}

/// Relative path -> bytes for every file under `dir`, for before/after
/// comparisons. A missing folder is an empty tree.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, files);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                files.insert(rel.replace('\\', "/"), fs::read(&path).unwrap());
            }
        }
    }

    let mut files = BTreeMap::new();
    walk(dir, dir, &mut files);
    files
}

// AC-arch-1.7.1; AC-arch-1.5.4; AC-arch-1.4.4: a top-level page and a section
// index that map to the same file fail planning, and the message names the
// full output path the plan is bound to plus both sources.
#[test]
fn page_and_section_index_collision_fails_planning() {
    let p = project("page_and_section_index_collision_fails_planning");
    write_file(
        &p.site.join("posts.md"),
        &frontmatter("Posts", None, None, false),
    );
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", None, None, false),
    );

    let err = plan(&options(&p, None)).expect_err("page and section index collide");

    assert!(matches!(err, MangoError::General(_)), "{err:?}");
    assert_eq!(
        err.to_string(),
        format!(
            "Mango Error: output path '{}' would be written by both page 'posts' and section index 'posts'",
            p.out.join("posts/index.html").display()
        )
    );
}

// AC-arch-1.7.6: one output is a file where another needs a directory
// (`feed.xml.md` renders to `<dist>/feed.xml/index.html`, next to the feed
// itself), and the message names both outputs.
#[test]
fn file_vs_folder_conflict_fails_planning() {
    let p = project("file_vs_folder_conflict_fails_planning");
    let config = p.dir.join("mango.json");
    write_file(&config, r#"{"base_url": "https://example.com"}"#);
    write_file(
        &p.site.join("feed.xml.md"),
        &frontmatter("Feed", None, None, false),
    );

    let err = plan(&options(&p, Some(&config))).expect_err("file vs folder conflict");

    assert_eq!(
        err.to_string(),
        format!(
            "Mango Error: output path '{}' would be written as a file by RSS feed, but page 'feed.xml' needs it to be a directory for '{}'",
            p.out.join("feed.xml").display(),
            p.out.join("feed.xml/index.html").display()
        )
    );
}

// AC-arch-1.4.4: planning reports the first failure in pipeline order, so a
// malformed config is reported and the missing templates folder is not.
#[test]
fn first_failure_is_reported_in_pipeline_order() {
    let p = project("first_failure_is_reported_in_pipeline_order");
    let config = p.dir.join("bad.json");
    write_file(&config, "{not valid json");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", None, None, false),
    );
    let mut opts = options(&p, Some(&config));
    opts.templates = p.dir.join("no-templates");

    let err = plan(&opts).expect_err("malformed config");

    assert!(matches!(err, MangoError::Config(_)), "{err:?}");
    let msg = err.to_string();
    assert!(msg.contains("bad.json"), "{msg}");
    assert!(!msg.contains("no-templates"), "{msg}");
}

// AC-arch-1.4.2; AC-arch-1.7.2: planning creates, modifies and deletes
// nothing, whether it fails or succeeds.
#[test]
fn planning_creates_modifies_and_deletes_nothing() {
    let p = project("planning_creates_modifies_and_deletes_nothing");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", Some("2026-01-01"), Some(r#"["rust"]"#), false),
    );
    write_file(&p.out.join("stale/index.html"), "previous output");

    let snapshot = |label: &str| {
        (
            label.to_string(),
            [
                tree(&p.site),
                tree(&p.templates),
                tree(&p.assets),
                tree(&p.out),
            ],
        )
    };

    // A plan that fails on bad input.
    write_file(
        &p.site.join("posts/bad.md"),
        &frontmatter("Bad", None, Some(r#"["Rust"]"#), false),
    );
    let (_, before) = snapshot("failing");
    plan(&options(&p, None)).expect_err("invalid tag must fail planning");
    let (_, after) = snapshot("failing");
    assert_eq!(after, before, "a failed plan touched the filesystem");

    // A plan that succeeds.
    fs::remove_file(p.site.join("posts/bad.md")).unwrap();
    let (_, before) = snapshot("succeeding");
    plan(&options(&p, None)).expect("planning must succeed");
    let (_, after) = snapshot("succeeding");
    assert_eq!(after, before, "a successful plan touched the filesystem");
}

// AC-arch-1.4.3; AC-arch-1.5.1; AC-arch-1.1.7: a successful plan already holds
// every output, fully rendered, in memory.
#[test]
fn plan_holds_every_output_fully_rendered() {
    let p = project("plan_holds_every_output_fully_rendered");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", Some("2026-01-01"), Some(r#"["rust"]"#), false),
    );

    let without_base_url = plan(&options(&p, None)).expect("planning must succeed");
    assert_eq!(
        paths(&without_base_url),
        [
            "posts/one/index.html",
            "posts/index.html",
            "index.html",
            "tags/index.html",
            "tags/rust/index.html",
            "assets/style.css",
        ]
        .map(PathBuf::from)
    );

    // Every rendered file is the template's exact output.
    assert_eq!(file(&without_base_url, "posts/one/index.html"), "One");
    assert_eq!(
        file(&without_base_url, "posts/index.html"),
        "One posts/one\n"
    );
    assert_eq!(file(&without_base_url, "index.html"), "One\n");
    assert_eq!(file(&without_base_url, "tags/index.html"), "rust\n");
    assert_eq!(file(&without_base_url, "tags/rust/index.html"), "One\n");

    // The asset is a copy from its source, not rendered content.
    assert_eq!(
        without_base_url.outputs().last().unwrap(),
        PlannedOutput::Copy {
            path: Path::new("assets/style.css"),
            source: &p.assets.join("style.css"),
        }
    );

    // The plan names the folder it is bound to.
    assert_eq!(without_base_url.output_dir(), p.out);

    // With `base_url` the feed and sitemap join the plan, already generated.
    let config = p.dir.join("mango.json");
    write_file(&config, r#"{"base_url": "https://example.com"}"#);
    let with_base_url = plan(&options(&p, Some(&config))).expect("planning must succeed");
    assert_eq!(
        paths(&with_base_url),
        [
            "posts/one/index.html",
            "posts/index.html",
            "index.html",
            "tags/index.html",
            "tags/rust/index.html",
            "feed.xml",
            "sitemap.xml",
            "assets/style.css",
        ]
        .map(PathBuf::from)
    );
    assert!(
        file(&with_base_url, "feed.xml").contains("<link>https://example.com/posts/one/</link>"),
        "{}",
        file(&with_base_url, "feed.xml")
    );
    assert!(
        file(&with_base_url, "sitemap.xml").contains("<lastmod>2026-01-01</lastmod>"),
        "{}",
        file(&with_base_url, "sitemap.xml")
    );
}

// AC-arch-1.5.2: planning the same inputs twice gives the same outputs, in the
// same order.
#[test]
fn planning_twice_gives_identical_outputs() {
    let p = project("planning_twice_gives_identical_outputs");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", Some("2026-01-01"), Some(r#"["rust"]"#), false),
    );
    write_file(
        &p.site.join("notes/two.md"),
        &frontmatter("Two", Some("2025-01-01"), Some(r#"["notes"]"#), false),
    );
    write_file(&p.assets.join("img/logo.svg"), "<svg/>\n");

    let first = plan(&options(&p, None)).expect("planning must succeed");
    let second = plan(&options(&p, None)).expect("planning must succeed");

    let first: Vec<PlannedOutput<'_>> = first.outputs().collect();
    let second: Vec<PlannedOutput<'_>> = second.outputs().collect();
    assert_eq!(first, second);
    assert!(!first.is_empty(), "nothing was planned");
}

// AC-arch-1.5.3; AC-arch-1.4.6: committing empties the output folder (keeping
// it) and leaves behind exactly the plan's enumerated outputs.
#[test]
fn commit_writes_exactly_the_enumerated_outputs() {
    let p = project("commit_writes_exactly_the_enumerated_outputs");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", Some("2026-01-01"), Some(r#"["rust"]"#), false),
    );
    write_file(&p.out.join("stray.txt"), "gone");
    write_file(&p.out.join("stale/deep/index.html"), "gone too");

    let plan = plan(&options(&p, None)).expect("planning must succeed");
    let expected: BTreeMap<String, Vec<u8>> = plan
        .outputs()
        .map(|output| match output {
            PlannedOutput::File { path, contents } => (
                path.to_string_lossy().replace('\\', "/"),
                contents.as_bytes().to_vec(),
            ),
            PlannedOutput::Copy { path, source } => (
                path.to_string_lossy().replace('\\', "/"),
                fs::read(source).unwrap(),
            ),
        })
        .collect();

    commit(plan).expect("committing must succeed");

    assert!(p.out.is_dir(), "the output folder itself must be kept");
    assert_eq!(tree(&p.out), expected);
}

// AC-arch-1.4.7; AC-arch-1.4.8: the safety check runs at commit time against
// the inputs the plan was made with, and a refusal deletes and writes nothing.
#[test]
fn commit_refuses_output_containing_an_input_and_touches_nothing() {
    let p = project("commit_refuses_output_containing_an_input_and_touches_nothing");
    write_file(
        &p.site.join("posts/one.md"),
        &frontmatter("One", None, None, false),
    );
    let mut opts = options(&p, None);
    // The output folder would contain the site, templates and assets folders.
    opts.output = p.dir.clone();

    let plan = plan(&opts).expect("planning does not check the output folder");
    let before = tree(&p.dir);

    let err = commit(plan).expect_err("an output folder containing an input must be refused");

    assert_eq!(
        err.to_string(),
        format!(
            "Mango Error: refusing to clean output path '{}': it is or contains the input path '{}'",
            p.dir.display(),
            p.site.display()
        )
    );
    assert_eq!(tree(&p.dir), before, "a refused commit changed files");
}

// AC-arch-1.7.7: within a section the plan lists pages newest date first,
// undated last, ties broken by title and then by slug.
#[test]
fn section_lists_newest_first_undated_last_then_title_then_slug() {
    let p = project("section_lists_newest_first_undated_last_then_title_then_slug");
    for (name, title, date) in [
        ("b-new.md", "Newer", Some("2026-01-02")),
        ("a-old.md", "Older", Some("2020-01-01")),
        ("same-b.md", "Bravo", Some("2025-01-01")),
        ("same-a.md", "Alpha", Some("2025-01-01")),
        ("dup-2.md", "Same Title", Some("2024-01-01")),
        ("dup-1.md", "Same Title", Some("2024-01-01")),
        ("undated.md", "Undated", None),
    ] {
        write_file(
            &p.site.join("posts").join(name),
            &frontmatter(title, date, None, false),
        );
    }

    let plan = plan(&options(&p, None)).expect("planning must succeed");

    assert_eq!(
        file(&plan, "posts/index.html"),
        concat!(
            "Newer posts/b-new\n",
            "Alpha posts/same-a\n",
            "Bravo posts/same-b\n",
            "Same Title posts/dup-1\n",
            "Same Title posts/dup-2\n",
            "Older posts/a-old\n",
            "Undated posts/undated\n",
        )
    );
}

// AC-2.2; AC-arch-1.7.8: the home page lists at most `recent_count` dated
// pages, newest first, and drops undated ones. Moved here from
// `tests/build.rs`, where it asserted the same thing through the binary.
#[test]
fn home_recent_respects_recent_count_and_skips_undated() {
    let p = project("home_recent_respects_recent_count_and_skips_undated");
    let config = p.dir.join("mango.json");
    write_file(&config, r#"{"recent_count": 2}"#);
    write_file(
        &p.site.join("posts/old.md"),
        &frontmatter("Oldest Page", Some("2020-01-01"), None, false),
    );
    write_file(
        &p.site.join("posts/mid.md"),
        &frontmatter("Middle Page", Some("2025-06-01"), None, false),
    );
    write_file(
        &p.site.join("notes/new.md"),
        &frontmatter("Newest Page", Some("2026-03-01"), None, false),
    );
    write_file(
        &p.site.join("posts/undated.md"),
        &frontmatter("Undated Page", None, None, false),
    );

    let plan = plan(&options(&p, Some(&config))).expect("planning must succeed");

    assert_eq!(file(&plan, "index.html"), "Newest Page\nMiddle Page\n");
}

// AC-3.2 (batch 1); AC-4.7; AC-arch-1.7.9: a draft page appears in none of the
// plan's outputs or listings. Moved here from `tests/build.rs`, which checked
// the page and its section index on disk; the plan shows every listing at
// once.
#[test]
fn build_excludes_draft_pages() {
    let p = project("build_excludes_draft_pages");
    let config = p.dir.join("mango.json");
    write_file(&config, r#"{"base_url": "https://example.com"}"#);
    write_file(
        &p.site.join("posts/published.md"),
        &frontmatter("Published", Some("2026-01-01"), Some(r#"["rust"]"#), false),
    );
    write_file(
        &p.site.join("posts/secret.md"),
        &frontmatter("Secret Draft", Some("2026-02-01"), Some(r#"["wip"]"#), true),
    );

    let plan = plan(&options(&p, Some(&config))).expect("planning must succeed");

    assert_eq!(
        paths(&plan),
        [
            "posts/published/index.html",
            "posts/index.html",
            "index.html",
            "tags/index.html",
            "tags/rust/index.html",
            "feed.xml",
            "sitemap.xml",
            "assets/style.css",
        ]
        .map(PathBuf::from),
        "the draft must have no output and no tag page"
    );
    for rel in [
        "posts/index.html",
        "index.html",
        "tags/index.html",
        "tags/rust/index.html",
        "feed.xml",
        "sitemap.xml",
    ] {
        let contents = file(&plan, rel);
        assert!(!contents.contains("Secret"), "{rel}: {contents}");
        assert!(!contents.contains("secret"), "{rel}: {contents}");
        assert!(!contents.contains("wip"), "{rel}: {contents}");
    }
}

// AC-arch-1.7.11: drafts are parsed and validated like any other page, so a
// draft with malformed frontmatter, an invalid date or an invalid tag still
// fails planning, naming the file and the offending value.
#[test]
fn draft_with_bad_frontmatter_date_or_tag_fails_planning() {
    let p = project("draft_with_bad_frontmatter_date_or_tag_fails_planning");
    write_file(
        &p.site.join("posts/good.md"),
        &frontmatter("Good", None, None, false),
    );

    let cases = [
        (
            "malformed frontmatter",
            "---\n{\"title\": \"Broken\"\n---\n# Broken\n".to_string(),
            None,
        ),
        (
            "invalid date",
            frontmatter("Dated", Some("2026-13-45"), None, true),
            Some("2026-13-45"),
        ),
        (
            "invalid tag",
            frontmatter("Tagged", None, Some(r#"["Rust"]"#), true),
            Some("Rust"),
        ),
    ];

    for (label, body, value) in cases {
        let path = p.site.join("posts/draft.md");
        write_file(&path, &body);

        let err = plan(&options(&p, None)).unwrap_err();

        let msg = err.to_string();
        assert!(
            matches!(err, MangoError::Frontmatter(_)),
            "{label}: {err:?}"
        );
        assert!(msg.contains(path.to_str().unwrap()), "{label}: {msg}");
        // Malformed JSON has no offending value of its own: serde names the
        // parse position instead, so only the file is asserted there.
        if let Some(value) = value {
            assert!(msg.contains(value), "{label}: {msg}");
        }
        fs::remove_file(&path).unwrap();
    }
}

// AC-arch-1.6.3: `mango clean`'s behaviour lives in the library: an existing
// folder is removed entirely, a missing one is success.
#[test]
fn clean_is_reachable_through_the_library() {
    let dir = temp_dir("clean_is_reachable_through_the_library");
    let dist = dir.join("dist");
    write_file(&dist.join("posts/index.html"), "built");

    clean(&dist).expect("an existing output folder is removed");
    assert!(!dist.exists());

    clean(&dist).expect("a missing output folder is success");
    assert!(dir.is_dir(), "only the output folder is removed");
}
