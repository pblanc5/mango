use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

// End-to-end tests: run the real binary. main() prints errors to stderr and
// exits non-zero on failure, so exit status is a meaningful check.
//
// Tests that exercise the output-cleaning safety check always run with the
// cwd inside their own temp_dir, so a broken check can only delete scratch
// files.

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_mango(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mango"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run mango binary")
}

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

fn page(title: &str, draft: bool) -> String {
    format!(
        "---\n{{\"title\": \"{title}\", \"author\": \"tester\", \"description\": \"desc\", \"draft\": {draft}}}\n---\n# {title}\n"
    )
}

fn dated_page(title: &str, date: &str) -> String {
    format!(
        "---\n{{\"title\": \"{title}\", \"author\": \"tester\", \"description\": \"desc\", \"date\": \"{date}\", \"draft\": false}}\n---\n# {title}\n"
    )
}

/// A page with a raw JSON `tags` value, e.g. `r#"["blog"]"#`.
fn tagged_page(title: &str, tags_json: &str, draft: bool) -> String {
    format!(
        "---\n{{\"title\": \"{title}\", \"author\": \"tester\", \"description\": \"desc\", \"tags\": {tags_json}, \"draft\": {draft}}}\n---\n# {title}\n"
    )
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Build a temp site using the committed fixture templates and assets.
fn build_temp_site(site: &Path, out: &Path) -> Output {
    let root = root();
    build_with(
        site,
        &root.join("example/meta/templates"),
        &root.join("example/meta/assets"),
        out,
    )
}

/// Build with explicit absolute paths, run from the repo root.
fn build_with(site: &Path, templates: &Path, assets: &Path, out: &Path) -> Output {
    run_mango(
        &[
            "build",
            "--site",
            site.to_str().unwrap(),
            "--templates",
            templates.to_str().unwrap(),
            "--assets",
            assets.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ],
        &root(),
    )
}

/// Copy a directory tree (regular files and directories only).
fn copy_dir(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dest.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Lay out a project with the default folder names (`site/`, `meta/templates`,
/// `meta/assets`) under `dir`, using the committed fixture templates/assets.
fn default_layout_project(dir: &Path) {
    let root = root();
    write_file(&dir.join("site/posts/one.md"), &page("One", false));
    copy_dir(
        &root.join("example/meta/templates"),
        &dir.join("meta/templates"),
    );
    copy_dir(&root.join("example/meta/assets"), &dir.join("meta/assets"));
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "mango exited with failure\nstderr: {}",
        stderr(output)
    );
}

fn assert_failure(output: &Output, what: &str) {
    assert!(!output.status.success(), "{what}: expected failure");
    assert!(
        stdout(output).is_empty(),
        "errors must not go to stdout, got: {}",
        stdout(output)
    );
}

/// Builds the committed fixture site (`example/site`, `example/meta`,
/// `example/mango.json`) once per test run and returns its output folder. The
/// fixture is meant to exercise every feature on the success path; the
/// `fixture_*` tests below read this output and must never modify it.
fn fixture_dist() -> &'static Path {
    static DIST: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIST.get_or_init(|| {
        let root = root();
        let out = root.join("target").join("integration-dist");
        let _ = fs::remove_dir_all(&out);

        let output = run_mango(
            &[
                "build",
                "--site",
                "example/site",
                "--templates",
                "example/meta/templates",
                "--assets",
                "example/meta/assets",
                "-o",
                out.to_str().unwrap(),
                "--config",
                "example/mango.json",
            ],
            &root,
        );
        assert_success(&output);
        out
    })
}

fn fixture_file(path: &str) -> String {
    fs::read_to_string(fixture_dist().join(path))
        .unwrap_or_else(|e| panic!("reading fixture output {path}: {e}"))
}

/// Asserts each needle occurs in `text`, each after the previous one.
fn assert_in_order(text: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let at = text[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("'{needle}' missing or out of order in:\n{text}"));
        from += at + needle.len();
    }
}

#[test]
fn fixture_renders_markdown_extensions() {
    let html = fixture_file("blog/how-i-write/index.html");
    for needle in [
        "<table>",
        "<del>writing every morning</del>",
        r#"class="footnote-reference""#,
        r#"<div class="footnote-definition" id="origin">"#,
        r#"<h2 id="tables">"#,
        r#"<h2 id="checklist" class="no-toc">"#,
        r#"<code class="language-json">"#,
        r#"<a href="https://spec.commonmark.org">"#,
    ] {
        assert!(html.contains(needle), "missing {needle}:\n{html}");
    }
    assert_eq!(html.matches(r#"type="checkbox""#).count(), 3, "{html}");
    assert_eq!(html.matches(r#"checked="""#).count(), 1, "{html}");
}

#[test]
fn fixture_escapes_special_characters() {
    let title = "Rust &amp; WebAssembly: drawing on a &lt;canvas&gt;";
    let description = "Pixels, &quot;quotes&quot; &amp; a 60 fps render loop without a framework";
    let html = fixture_file("blog/rust-and-wasm-canvas/index.html");
    assert!(
        between(&html, "<title>", "</title>").contains(title),
        "{html}"
    );
    assert!(html.contains(&format!("<h1>{title}</h1>")), "{html}");
    assert!(
        html.contains(&format!(
            r#"<meta name="description" content="{description}">"#
        )),
        "{html}"
    );
    assert!(
        html.contains("width &gt; 0 &amp;&amp; height &lt; 4096"),
        "{html}"
    );
    assert!(!html.contains("<canvas>"), "{html}");

    let feed = fixture_file("feed.xml");
    assert!(feed.contains(&format!("<title>{title}</title>")), "{feed}");
    assert!(
        feed.contains(&format!("<description>{description}</description>")),
        "{feed}"
    );
}

#[test]
fn fixture_excludes_drafts_everywhere() {
    let out = fixture_dist();
    assert!(!out.join("blog/incremental-builds").exists());
    assert!(!out.join("tags/wip").exists());
    for file in [
        "index.html",
        "blog/index.html",
        "tags/index.html",
        "tags/rust/index.html",
        "feed.xml",
        "sitemap.xml",
    ] {
        let text = fixture_file(file);
        assert!(
            !text.contains("Notes on incremental builds")
                && !text.contains("incremental-builds")
                && !text.contains("/tags/wip/"),
            "{file} mentions the draft:\n{text}"
        );
    }
}

#[test]
fn fixture_orders_listings_and_caps_recent() {
    // Newest first, same date (2026-01-24) by title, undated last.
    let blog = fixture_file("blog/index.html");
    let posts = between(&blog, r#"<ul class="post-list">"#, "</ul>");
    assert_in_order(
        posts,
        &[
            "Rust &amp; WebAssembly",
            "How I write these posts",
            "Fixing a flaky test in 40 lines",
            "The weather station, one year on",
            "2025 in review",
            "Why I still reach for make",
            "Reading list",
        ],
    );
    assert_eq!(posts.matches("<li>").count(), 7, "{posts}");

    // recent_count is 5: the five newest dated pages site-wide, projects included.
    let home = fixture_file("index.html");
    let recent = between(&home, r#"<ul class="recent-list">"#, "</ul>");
    assert_in_order(
        recent,
        &[
            "Rust &amp; WebAssembly",
            "Lantern",
            "How I write these posts",
            "Fixing a flaky test in 40 lines",
            "The weather station, one year on",
        ],
    );
    assert_eq!(recent.matches("<li>").count(), 5, "{recent}");
    for absent in ["2025 in review", "Reading list", "Tidepool", "About"] {
        assert!(!recent.contains(absent), "{absent} in recent:\n{recent}");
    }

    // Undated pages come last in tag listings too.
    let writing = fixture_file("tags/writing/index.html");
    assert_in_order(&writing, &["How I write these posts", "Reading list"]);
}

#[test]
fn fixture_nested_sections_and_top_level_page() {
    let home = fixture_file("index.html");
    let explore = between(&home, r#"<ul class="card-grid">"#, "</ul>");
    assert_in_order(
        explore,
        &[
            r#"href="/blog/"><strong>Blog</strong><span>7 pages</span>"#,
            r#"href="/projects/"><strong>Projects</strong><span>Browse</span>"#,
        ],
    );
    assert!(!explore.contains("/about/"), "{explore}");

    // projects/ has only subsections, no pages of its own.
    let projects = fixture_file("projects/index.html");
    assert_in_order(
        &projects,
        &[
            r#"href="/projects/hardware/""#,
            r#"href="/projects/software/""#,
        ],
    );
    assert!(!projects.contains(r#"class="post-list""#), "{projects}");

    let hardware = fixture_file("projects/hardware/index.html");
    assert!(
        hardware.contains(r#"href="/projects/hardware/keyboards/""#),
        "{hardware}"
    );
    assert!(
        hardware.contains(r#"href="/projects/hardware/weather-station/""#),
        "{hardware}"
    );

    // Three levels deep: breadcrumbs link every ancestor section.
    let keyboards = fixture_file("projects/hardware/keyboards/index.html");
    let crumbs = between(&keyboards, r#"<nav class="breadcrumbs""#, "</nav>");
    assert_in_order(
        crumbs,
        &[
            r#"<a href="/">Home</a>"#,
            r#"<a href="/projects/">Projects</a>"#,
            r#"<a href="/projects/hardware/">Hardware</a>"#,
        ],
    );
    assert!(keyboards.contains("<h1>Keyboards</h1>"), "{keyboards}");
    assert!(
        keyboards.contains(r#"href="/projects/hardware/keyboards/corne-build/""#),
        "{keyboards}"
    );
    assert!(!keyboards.contains(r#"class="card-grid""#), "{keyboards}");
    assert!(!keyboards.contains("&#x2F;"), "{keyboards}");

    let about = fixture_file("about/index.html");
    assert!(about.contains("<h1>About</h1>"), "{about}");
    assert!(!about.contains("<time"), "{about}");
}

#[test]
fn fixture_tag_index_counts_and_dedup() {
    let tags = fixture_file("tags/index.html");
    let cloud = between(&tags, r#"<ul class="tag-cloud">"#, "</ul>");
    assert_in_order(
        cloud,
        &[
            r#">books<span class="count">1</span>"#,
            r#">electronics<span class="count">2</span>"#,
            r#">hardware<span class="count">3</span>"#,
            r#">keyboards<span class="count">1</span>"#,
            r#">markdown<span class="count">1</span>"#,
            r#">personal<span class="count">1</span>"#,
            r#">rust<span class="count">4</span>"#,
            r#">testing<span class="count">1</span>"#,
            r#">tools<span class="count">2</span>"#,
            r#">webassembly<span class="count">2</span>"#,
            r#">writing<span class="count">2</span>"#,
        ],
    );
    assert_eq!(cloud.matches("<li>").count(), 11, "{cloud}");

    // how-i-write.md lists "writing" twice.
    let page = fixture_file("blog/how-i-write/index.html");
    let links = between(&page, r#"<ul class="tag-list">"#, "</ul>");
    assert_eq!(
        links.matches(r#"href="/tags/writing/""#).count(),
        1,
        "duplicate tag must be dropped:\n{links}"
    );
    assert_in_order(
        links,
        &[r#"href="/tags/writing/""#, r#"href="/tags/markdown/""#],
    );
}

#[test]
fn fixture_feed_and_sitemap() {
    let feed = fixture_file("feed.xml");
    assert!(
        feed.contains(
            "<description>Notes on Rust, small tools &amp; things that go &lt;beep&gt;</description>"
        ),
        "{feed}"
    );
    // base_url has a trailing slash in example/mango.json.
    assert!(
        feed.contains("<link>https://wrencalloway.example/</link>"),
        "{feed}"
    );
    assert!(!feed.contains(".example//"), "{feed}");
    assert_eq!(feed.matches("<item>").count(), 5, "{feed}");
    assert_in_order(
        &feed,
        &[
            "Rust &amp; WebAssembly",
            "Lantern",
            "How I write these posts",
            "Fixing a flaky test in 40 lines",
            "The weather station, one year on",
        ],
    );
    assert!(
        feed.contains("<pubDate>Mon, 02 Mar 2026 00:00:00 +0000</pubDate>"),
        "{feed}"
    );
    assert!(
        feed.contains("<pubDate>Fri, 20 Feb 2026 00:00:00 +0000</pubDate>"),
        "{feed}"
    );

    let sitemap = fixture_file("sitemap.xml");
    assert_eq!(sitemap.matches("<loc>").count(), 30, "{sitemap}");
    assert_eq!(sitemap.matches("<lastmod>").count(), 10, "{sitemap}");
    assert!(!sitemap.contains(".example//"), "{sitemap}");
    for path in [
        "about/",
        "blog/",
        "projects/",
        "projects/hardware/keyboards/",
        "tags/",
        "tags/writing/",
    ] {
        assert!(
            sitemap.contains(&format!("<loc>https://wrencalloway.example/{path}</loc>")),
            "{path} missing:\n{sitemap}"
        );
    }
    assert!(
        sitemap.contains("<loc>https://wrencalloway.example/about/</loc>\n  </url>"),
        "undated page must have no lastmod:\n{sitemap}"
    );
    assert!(
        sitemap.contains(
            "<loc>https://wrencalloway.example/projects/hardware/keyboards/corne-build/</loc>\n    <lastmod>2025-03-10</lastmod>"
        ),
        "{sitemap}"
    );
}

#[test]
fn fixture_copies_nested_assets_and_fills_page_context() {
    let out = fixture_dist();
    let source = root().join("example/meta/assets");
    for asset in [
        "css/main.css",
        "images/logo.svg",
        "images/projects/weather-station.svg",
    ] {
        assert_eq!(
            fs::read(out.join("assets").join(asset)).unwrap(),
            fs::read(source.join(asset)).unwrap(),
            "{asset}"
        );
    }

    let page = fixture_file("blog/fixing-a-flaky-test/index.html");
    assert!(
        page.contains(r#"<link rel="canonical" href="/blog/fixing-a-flaky-test/">"#),
        "{page}"
    );
    assert!(page.contains("by Wren Calloway"), "{page}");
    assert!(
        page.contains(
            r#"<p class="lede">A test that failed one run in fifty, and the directory listing that caused it</p>"#
        ),
        "{page}"
    );

    let home = fixture_file("index.html");
    assert!(
        home.contains(
            r#"<meta name="description" content="Notes on Rust, small tools &amp; things that go &lt;beep&gt;">"#
        ),
        "{home}"
    );
    assert!(
        home.contains(r#"<img src="/assets/images/logo.svg""#),
        "{home}"
    );

    let station = fixture_file("blog/weather-station-one-year-on/index.html");
    assert!(
        station.contains(r#"<img src="/assets/images/projects/weather-station.svg""#),
        "{station}"
    );
}

// AC-2.4, AC-2.8, AC-4.3, AC-5.2 (batch 1); AC-1.8, AC-2.6, AC-7.4 (batch 2);
// AC-2.5, AC-8.1, AC-8.2, AC-8.3 (batch 4)
#[test]
fn build_generates_site_from_fixture() {
    let out = fixture_dist();

    // The exact output manifest: catches missing files and stray ones
    // (drafts, dropped tags, misplaced assets).
    let mut expected = vec![
        "about/index.html",
        "assets/css/main.css",
        "assets/images/logo.svg",
        "assets/images/projects/weather-station.svg",
        "blog/2025-in-review/index.html",
        "blog/fixing-a-flaky-test/index.html",
        "blog/how-i-write/index.html",
        "blog/index.html",
        "blog/reading-list/index.html",
        "blog/rust-and-wasm-canvas/index.html",
        "blog/weather-station-one-year-on/index.html",
        "blog/why-i-still-use-make/index.html",
        "feed.xml",
        "index.html",
        "projects/hardware/index.html",
        "projects/hardware/keyboards/corne-build/index.html",
        "projects/hardware/keyboards/index.html",
        "projects/hardware/weather-station/index.html",
        "projects/index.html",
        "projects/software/index.html",
        "projects/software/lantern/index.html",
        "projects/software/tidepool/index.html",
        "sitemap.xml",
        "tags/books/index.html",
        "tags/electronics/index.html",
        "tags/hardware/index.html",
        "tags/index.html",
        "tags/keyboards/index.html",
        "tags/markdown/index.html",
        "tags/personal/index.html",
        "tags/rust/index.html",
        "tags/testing/index.html",
        "tags/tools/index.html",
        "tags/webassembly/index.html",
        "tags/writing/index.html",
    ];
    expected.sort();
    let actual: Vec<String> = snapshot(out)
        .into_iter()
        .map(|f| f.replace('\\', "/"))
        .collect();
    assert_eq!(actual, expected);

    let page = fs::read_to_string(out.join("blog/fixing-a-flaky-test/index.html")).unwrap();
    assert!(
        page.contains("Fixing a flaky test in 40 lines"),
        "rendered page should contain the post title"
    );
    // AC-5.2
    assert!(
        page.contains(
            r#"<meta name="description" content="A test that failed one run in fifty, and the directory listing that caused it">"#
        ),
        "rendered page should contain the frontmatter description meta tag, got:\n{page}"
    );
    // AC-1.8
    assert!(
        page.contains(r#"<time datetime="2026-01-24">2026-01-24</time>"#),
        "rendered page should contain the formatted date, got:\n{page}"
    );

    // AC-2.6: both posts share a date, so they are ordered by title.
    let index = fs::read_to_string(out.join("blog/index.html")).unwrap();
    assert_in_order(
        &index,
        &[
            "Fixing a flaky test in 40 lines",
            "The weather station, one year on",
        ],
    );

    // AC-5.4 (batch 3)
    assert!(
        index.contains(r#"href="/blog/fixing-a-flaky-test/""#),
        "{index}"
    );
    assert!(!index.contains("&#x2F;"), "{index}");

    // AC-1.14 (batch 3)
    assert!(
        between(&page, "<title>", "</title>").contains("Wren Calloway"),
        "configured title missing from <title>:\n{page}"
    );
    // Each page names itself in <title> and has exactly one <h1>.
    assert!(
        between(&page, "<title>", "</title>").contains("Fixing a flaky test in 40 lines |"),
        "page title missing from <title>:\n{page}"
    );
    assert_eq!(page.matches("<h1").count(), 1, "{page}");
    assert!(
        between(&page, "<footer>", "</footer>").contains("Wren Calloway"),
        "configured author missing from <footer>:\n{page}"
    );

    // AC-2.7 (batch 3)
    let home = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(
        home.contains("<!DOCTYPE html>"),
        "home must extend base:\n{home}"
    );
    assert_in_order(&home, &["Rust &amp; WebAssembly", "Lantern"]);
    assert!(home.contains(r#"href="/blog/""#), "{home}");
    assert!(
        home.contains(r#"href="/projects/software/lantern/""#),
        "{home}"
    );

    // AC-8.3 (batch 4): tag pages, tag links, feed and sitemap.
    let rust = fs::read_to_string(out.join("tags/rust/index.html")).unwrap();
    assert!(
        rust.contains(r#"href="/blog/fixing-a-flaky-test/""#),
        "{rust}"
    );
    assert!(
        rust.contains(r#"href="/blog/rust-and-wasm-canvas/""#),
        "{rust}"
    );
    assert!(page.contains(r#"href="/tags/rust/""#), "{page}");

    // AC-8.2 (batch 4)
    assert!(
        page.contains(r#"<link rel="alternate" type="application/rss+xml" href="/feed.xml">"#),
        "{page}"
    );
    let tags = fs::read_to_string(out.join("tags/index.html")).unwrap();
    assert!(tags.contains(r#"href="/tags/rust/""#), "{tags}");

    let feed = fs::read_to_string(out.join("feed.xml")).unwrap();
    assert!(
        feed.contains("<link>https://wrencalloway.example/blog/fixing-a-flaky-test/</link>"),
        "{feed}"
    );
    assert_in_order(&feed, &["Rust &amp; WebAssembly", "Lantern"]);

    let sitemap = fs::read_to_string(out.join("sitemap.xml")).unwrap();
    assert!(
        sitemap.contains("<loc>https://wrencalloway.example/</loc>"),
        "{sitemap}"
    );
    assert!(
        sitemap.contains("<loc>https://wrencalloway.example/tags/rust/</loc>"),
        "{sitemap}"
    );
}

// AC-1.5 (batch 4)
#[test]
fn build_fails_on_invalid_tag_naming_file_and_value() {
    let dir = temp_dir("build_fails_on_invalid_tag_naming_file_and_value");
    let site = dir.join("site");
    write_file(&site.join("posts/good.md"), &page("Good", false));
    write_file(
        &site.join("posts/bad_tag.md"),
        &tagged_page("Bad", r#"["blog", "Rust"]"#, false),
    );

    let output = build_temp_site(&site, &dir.join("dist"));

    assert_failure(&output, "invalid tag");
    let err = stderr(&output);
    assert!(err.contains("bad_tag.md"), "{err}");
    assert!(err.contains("'Rust'"), "{err}");
}

// AC-1.6 (batch 4)
#[test]
fn build_fails_on_invalid_tag_in_draft() {
    let dir = temp_dir("build_fails_on_invalid_tag_in_draft");
    let site = dir.join("site");
    write_file(&site.join("posts/good.md"), &page("Good", false));
    write_file(
        &site.join("posts/draft_tag.md"),
        &tagged_page("Draft", r#"["Rust"]"#, true),
    );

    let output = build_temp_site(&site, &dir.join("dist"));

    assert_failure(&output, "invalid tag in draft");
    let err = stderr(&output);
    assert!(err.contains("draft_tag.md"), "{err}");
    assert!(err.contains("'Rust'"), "{err}");
}

// AC-2.6 (batch 4)
#[test]
fn build_fails_on_tags_folder_collision() {
    let dir = temp_dir("build_fails_on_tags_folder_collision");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("tags/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");

    let output = build_temp_site(&site, &out);

    assert_failure(&output, "tags folder collision");
    let err = stderr(&output);
    let collided = out.join("tags").join("index.html");
    assert!(err.contains(collided.to_str().unwrap()), "{err}");
    assert!(err.contains("section index 'tags'"), "{err}");
    assert!(err.contains("tag index"), "{err}");
    assert_eq!(snapshot(&out), vec!["marker.txt".to_string()]);
}

// AC-2.7 (batch 4)
#[test]
fn build_fails_on_top_level_tags_page_collision() {
    let dir = temp_dir("build_fails_on_top_level_tags_page_collision");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("tags.md"), &page("Tags Page", false));
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");

    let output = build_temp_site(&site, &out);

    assert_failure(&output, "tags page collision");
    let err = stderr(&output);
    let collided = out.join("tags").join("index.html");
    assert!(err.contains(collided.to_str().unwrap()), "{err}");
    assert!(err.contains("page 'tags'"), "{err}");
    assert!(err.contains("tag index"), "{err}");
    assert_eq!(snapshot(&out), vec!["marker.txt".to_string()]);
}

// AC-2.8 (batch 4)
#[test]
fn build_writes_tag_index_without_tags() {
    let dir = temp_dir("build_writes_tag_index_without_tags");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));

    let output = build_temp_site(&site, &out);

    assert_success(&output);
    assert!(
        out.join("tags/index.html").is_file(),
        "got: {}",
        list_files(&out).join(", ")
    );
}

// AC-2.9 (batch 4)
#[test]
fn missing_tags_template_keeps_previous_output() {
    let (site, templates, assets, out) =
        built_site_with_private_templates("missing_tags_template_keeps_previous_output");
    assert!(out.join("tags/index.html").is_file());
    let before = snapshot(&out);

    fs::remove_file(templates.join("tags.html")).unwrap();
    let output = build_with(&site, &templates, &assets, &out);

    assert_failure(&output, "missing tags.html");
    assert!(stderr(&output).contains("tags.html"), "{}", stderr(&output));
    assert_previous_output_intact(&out);
    assert_eq!(snapshot(&out), before);
}

// AC-4.3 (batch 4)
#[test]
fn build_fails_on_invalid_base_url_keeping_output() {
    let dir = temp_dir("build_fails_on_invalid_base_url_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    let config = dir.join("mango.json");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&config, r#"{"base_url": "https://example.com"}"#);

    assert_success(&build_temp_site_with_config(&site, &out, &config));
    let before = snapshot(&out);

    write_file(&config, r#"{"base_url": "example.com"}"#);
    let output = build_temp_site_with_config(&site, &out, &config);

    assert_failure(&output, "invalid base_url");
    let err = stderr(&output);
    assert!(err.contains("Mango Config Error"), "{err}");
    assert!(err.contains(config.to_str().unwrap()), "{err}");
    assert!(err.contains("example.com"), "{err}");
    assert_eq!(snapshot(&out), before, "output changed");
}

// AC-6.1, AC-7.1, AC-8.4, AC-8.5 (batch 4)
#[test]
fn feed_and_sitemap_only_with_base_url() {
    let dir = temp_dir("feed_and_sitemap_only_with_base_url");
    let site = dir.join("site");
    let out = dir.join("dist");
    let config = dir.join("mango.json");
    write_file(&site.join("posts/one.md"), &dated_page("One", "2026-01-24"));

    // No config at all.
    assert_success(&build_temp_site(&site, &out));
    assert!(out.join("posts/one/index.html").is_file());
    assert!(!out.join("feed.xml").exists(), "feed without base_url");
    assert!(
        !out.join("sitemap.xml").exists(),
        "sitemap without base_url"
    );

    // With base_url: both written.
    write_file(&config, r#"{"base_url": "https://example.com/"}"#);
    assert_success(&build_temp_site_with_config(&site, &out, &config));
    let feed = fs::read_to_string(out.join("feed.xml")).unwrap();
    assert!(
        feed.contains("<link>https://example.com/posts/one/</link>"),
        "{feed}"
    );
    let sitemap = fs::read_to_string(out.join("sitemap.xml")).unwrap();
    assert!(
        sitemap.contains("<loc>https://example.com/posts/one/</loc>"),
        "{sitemap}"
    );

    // base_url removed: a rebuild deletes the old files.
    write_file(&config, r#"{"title": "No Base"}"#);
    assert_success(&build_temp_site_with_config(&site, &out, &config));
    assert!(out.join("posts/one/index.html").is_file());
    assert!(!out.join("feed.xml").exists(), "stale feed.xml kept");
    assert!(!out.join("sitemap.xml").exists(), "stale sitemap.xml kept");
}

/// Text between the first `start` and the following `end` marker.
fn between<'a>(html: &'a str, start: &str, end: &str) -> &'a str {
    let from = html
        .find(start)
        .unwrap_or_else(|| panic!("'{start}' missing from:\n{html}"))
        + start.len();
    let len = html[from..]
        .find(end)
        .unwrap_or_else(|| panic!("'{end}' missing from:\n{html}"));
    &html[from..from + len]
}

/// Like `build_temp_site`, but passes `--config <config>`.
fn build_temp_site_with_config(site: &Path, out: &Path, config: &Path) -> Output {
    let root = root();
    run_mango(
        &[
            "build",
            "--site",
            site.to_str().unwrap(),
            "--templates",
            root.join("example/meta/templates").to_str().unwrap(),
            "--assets",
            root.join("example/meta/assets").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
        ],
        &root,
    )
}

// AC-1.2
#[test]
fn build_without_config_uses_defaults() {
    let dir = temp_dir("build_without_config_uses_defaults");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));

    let output = build_temp_site(&site, &out);

    assert_success(&output);
    for file in ["posts/one/index.html", "index.html"] {
        let html = fs::read_to_string(out.join(file)).unwrap();
        assert!(
            between(&html, "<title>", "</title>").contains("My Site"),
            "{file}:\n{html}"
        );
    }
}

// AC-1.3
#[test]
fn build_reads_default_mango_json_from_cwd() {
    let dir = temp_dir("build_reads_default_mango_json_from_cwd");
    default_layout_project(&dir);
    write_file(
        &dir.join("mango.json"),
        r#"{"title": "Default Json Title", "author": "Default Json Author"}"#,
    );

    let output = run_mango(&["build"], &dir);

    assert_success(&output);
    let html = fs::read_to_string(dir.join("dist/posts/one/index.html")).unwrap();
    assert!(
        between(&html, "<title>", "</title>").contains("Default Json Title"),
        "{html}"
    );
    assert!(
        between(&html, "<footer>", "</footer>").contains("Default Json Author"),
        "{html}"
    );
}

// AC-1.4
#[test]
fn build_fails_when_explicit_config_missing() {
    let dir = temp_dir("build_fails_when_explicit_config_missing");
    let site = dir.join("site");
    let out = dir.join("dist");
    let config = dir.join("missing.json");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");
    let before = snapshot(&out);

    let output = build_temp_site_with_config(&site, &out, &config);

    assert_failure(&output, "explicit missing config");
    assert!(
        stderr(&output).contains(config.to_str().unwrap()),
        "{}",
        stderr(&output)
    );
    assert_eq!(snapshot(&out), before, "output changed");
}

// AC-1.5, AC-1.6
#[test]
fn build_fails_on_malformed_or_unknown_field_config() {
    let dir = temp_dir("build_fails_on_malformed_or_unknown_field_config");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");
    let before = snapshot(&out);

    let malformed = "{not valid json";
    let serde_msg = serde_json::from_str::<serde_json::Value>(malformed)
        .unwrap_err()
        .to_string();

    for (name, content, expected) in [
        ("malformed.json", malformed, serde_msg.as_str()),
        ("unknown.json", r#"{"titel": "Typo"}"#, "titel"),
    ] {
        let config = dir.join(name);
        write_file(&config, content);

        let output = build_temp_site_with_config(&site, &out, &config);

        assert_failure(&output, name);
        let err = stderr(&output);
        assert!(err.contains("Mango Config Error"), "{name}: {err}");
        assert!(err.contains(config.to_str().unwrap()), "{name}: {err}");
        assert!(err.contains(expected), "{name}: {err}");
        assert_eq!(snapshot(&out), before, "{name}: output changed");
    }
}

// AC-1.8
#[test]
fn config_error_reported_before_templates_error() {
    let dir = temp_dir("config_error_reported_before_templates_error");
    let site = dir.join("site");
    let config = dir.join("bad.json");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&config, "{not valid json");

    let output = run_mango(
        &[
            "build",
            "--site",
            site.to_str().unwrap(),
            "--templates",
            dir.join("no-templates").to_str().unwrap(),
            "--assets",
            root().join("example/meta/assets").to_str().unwrap(),
            "-o",
            dir.join("dist").to_str().unwrap(),
            "--config",
            config.to_str().unwrap(),
        ],
        &root(),
    );

    assert_failure(&output, "bad config and missing templates");
    let err = stderr(&output);
    assert!(err.contains("Mango Config Error"), "{err}");
    assert!(!err.contains("no-templates"), "{err}");
}

// AC-1.12
#[test]
fn build_refuses_output_containing_config() {
    let dir = temp_dir("build_refuses_output_containing_config");
    default_layout_project(&dir);
    write_file(&dir.join("out/mango.json"), r#"{"title": "T"}"#);
    write_file(&dir.join("out/marker.txt"), "keep me");
    let before = snapshot(&dir);

    let output = run_mango(&["build", "--config", "out/mango.json", "-o", "out"], &dir);

    assert_failure(&output, "output contains config");
    let err = stderr(&output);
    assert!(err.contains("'out'"), "{err}");
    assert!(err.contains("'out/mango.json'"), "{err}");
    assert_eq!(snapshot(&dir), before, "files were changed");
}

// AC-2.2
#[test]
fn home_recent_respects_recent_count_and_skips_undated() {
    let dir = temp_dir("home_recent_respects_recent_count_and_skips_undated");
    let site = dir.join("site");
    let out = dir.join("dist");
    let config = dir.join("mango.json");
    write_file(
        &site.join("posts/old.md"),
        &dated_page("Oldest Page", "2020-01-01"),
    );
    write_file(
        &site.join("posts/mid.md"),
        &dated_page("Middle Page", "2025-06-01"),
    );
    write_file(
        &site.join("notes/new.md"),
        &dated_page("Newest Page", "2026-03-01"),
    );
    write_file(&site.join("posts/undated.md"), &page("Undated Page", false));
    write_file(&config, r#"{"recent_count": 2}"#);

    let output = build_temp_site_with_config(&site, &out, &config);

    assert_success(&output);
    let home = fs::read_to_string(out.join("index.html")).unwrap();
    let newest = home.find("Newest Page").expect(&home);
    let middle = home.find("Middle Page").expect(&home);
    assert!(newest < middle, "{home}");
    assert!(!home.contains("Oldest Page"), "{home}");
    assert!(!home.contains("Undated Page"), "{home}");
}

// AC-2.6
#[test]
fn missing_home_template_keeps_previous_output() {
    let (site, templates, assets, out) =
        built_site_with_private_templates("missing_home_template_keeps_previous_output");
    assert!(out.join("index.html").is_file());
    let before = snapshot(&out);

    fs::remove_file(templates.join("home.html")).unwrap();
    let output = build_with(&site, &templates, &assets, &out);

    assert_failure(&output, "missing home.html");
    assert!(stderr(&output).contains("home.html"), "{}", stderr(&output));
    assert_previous_output_intact(&out);
    assert_eq!(snapshot(&out), before);
}

// AC-3.5, AC-5.3
#[test]
fn build_generates_ancestor_section_indexes() {
    let dir = temp_dir("build_generates_ancestor_section_indexes");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("a/b/c.md"), &page("Deep Page", false));

    let output = build_temp_site(&site, &out);

    assert_success(&output);
    let a = fs::read_to_string(out.join("a/index.html")).unwrap();
    assert!(a.contains(r#"href="/a/b/""#), "{a}");
    let ab = fs::read_to_string(out.join("a/b/index.html")).unwrap();
    assert!(ab.contains(r#"href="/a/b/c/""#), "{ab}");
    let home = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(home.contains(r#"href="/a/""#), "{home}");
}

// AC-3.6
#[test]
fn build_fails_on_page_and_ancestor_section_collision() {
    let dir = temp_dir("build_fails_on_page_and_ancestor_section_collision");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("a.md"), &page("A Page", false));
    write_file(&site.join("a/b/c.md"), &page("Deep", false));
    write_file(&out.join("marker.txt"), "keep me");

    let output = build_temp_site(&site, &out);

    assert_failure(&output, "ancestor collision");
    let err = stderr(&output);
    let collided = out.join("a").join("index.html");
    assert!(err.contains(collided.to_str().unwrap()), "{err}");
    assert!(err.contains("page 'a'"), "{err}");
    assert!(err.contains("section index 'a'"), "{err}");
    assert_eq!(snapshot(&out), vec!["marker.txt".to_string()]);
}

// AC-1.5, AC-2.4 (batch 1); AC-6.2
#[test]
fn build_fails_when_subdirectory_page_lacks_frontmatter() {
    let dir = temp_dir("build_fails_when_subdirectory_page_lacks_frontmatter");
    let site = dir.join("site");
    write_file(&site.join("posts/bad.md"), "# no frontmatter here\n");
    write_file(&site.join("posts/good.md"), &page("Good", false));

    let output = build_temp_site(&site, &dir.join("dist"));

    assert!(
        !output.status.success(),
        "build should fail when a page lacks frontmatter"
    );
    assert!(
        stderr(&output).contains("bad.md"),
        "stderr should name the offending file, got: {}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "errors must not go to stdout, got: {}",
        stdout(&output)
    );
}

// AC-3.2 (batch 1); AC-4.7
#[test]
fn build_excludes_draft_pages() {
    let dir = temp_dir("build_excludes_draft_pages");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/published.md"), &page("Published", false));
    write_file(&site.join("posts/secret.md"), &page("Secret Draft", true));

    let output = build_temp_site(&site, &out);

    assert!(
        output.status.success(),
        "build failed\nstderr: {}",
        stderr(&output)
    );
    assert!(
        out.join("posts/published/index.html").is_file(),
        "published page missing, got: {}",
        list_files(&out).join(", ")
    );
    assert!(
        !out.join("posts/secret").exists(),
        "draft page must not be written"
    );
    let index = fs::read_to_string(out.join("posts/index.html")).unwrap();
    assert!(
        !index.contains("secret") && !index.contains("Secret Draft"),
        "section index must not list the draft, got:\n{index}"
    );
}

// AC-2.5, AC-2.4 (batch 1)
#[test]
fn build_fails_with_missing_templates_dir() {
    let root = root();
    let dir = temp_dir("build_fails_with_missing_templates_dir");
    let templates = dir.join("nowhere-templates");

    let output = run_mango(
        &[
            "build",
            "--site",
            "example/site",
            "--templates",
            templates.to_str().unwrap(),
            "--assets",
            "example/meta/assets",
            "-o",
            dir.join("dist").to_str().unwrap(),
        ],
        &root,
    );

    assert!(
        !output.status.success(),
        "build should fail with a missing templates dir"
    );
    assert!(
        stderr(&output).contains(templates.to_str().unwrap()),
        "stderr should name the templates path, got: {}",
        stderr(&output)
    );
    assert!(
        stdout(&output).is_empty(),
        "errors must not go to stdout, got: {}",
        stdout(&output)
    );
}

// AC-1.7
#[test]
fn build_fails_on_invalid_date_naming_file_and_value() {
    let dir = temp_dir("build_fails_on_invalid_date_naming_file_and_value");
    let site = dir.join("site");
    write_file(&site.join("posts/good.md"), &page("Good", false));
    write_file(
        &site.join("posts/bad_date.md"),
        &dated_page("Bad", "2026-02-30"),
    );

    let output = build_temp_site(&site, &dir.join("dist"));

    assert_failure(&output, "invalid date");
    let err = stderr(&output);
    assert!(err.contains("bad_date.md"), "{err}");
    assert!(err.contains("2026-02-30"), "{err}");
}

// AC-1.8
#[test]
fn build_undated_page_renders_without_time() {
    let dir = temp_dir("build_undated_page_renders_without_time");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/undated.md"), &page("Undated", false));

    let output = build_temp_site(&site, &out);

    assert_success(&output);
    let html = fs::read_to_string(out.join("posts/undated/index.html")).unwrap();
    assert!(!html.contains("<time"), "undated page has <time>:\n{html}");
}

// AC-3.1, AC-3.2
#[test]
fn build_fails_on_page_and_section_output_collision() {
    let dir = temp_dir("build_fails_on_page_and_section_output_collision");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts.md"), &page("Posts Page", false));
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");

    let output = build_temp_site(&site, &out);

    assert_failure(&output, "collision");
    let err = stderr(&output);
    let collided = out.join("posts").join("index.html");
    assert!(err.contains(collided.to_str().unwrap()), "{err}");
    assert!(err.contains("page 'posts'"), "{err}");
    assert!(err.contains("section index 'posts'"), "{err}");

    // AC-3.2
    assert!(
        out.join("marker.txt").is_file(),
        "output must not be cleaned"
    );
    assert!(
        !out.join("posts/one/index.html").exists(),
        "nothing may be written on collision"
    );
}

// AC-3.3
#[test]
fn build_allows_index_md_inside_section() {
    let dir = temp_dir("build_allows_index_md_inside_section");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/index.md"), &page("Index Page", false));
    write_file(&site.join("posts/one.md"), &page("One", false));

    let output = build_temp_site(&site, &out);

    assert_success(&output);
    assert!(out.join("posts/index/index.html").is_file());
    assert!(out.join("posts/index.html").is_file());
}

// AC-4.1
#[test]
fn rebuild_removes_deleted_page_and_stale_files() {
    let dir = temp_dir("rebuild_removes_deleted_page_and_stale_files");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/keep.md"), &page("Keep", false));
    write_file(&site.join("posts/gone.md"), &page("Gone Page", false));

    assert_success(&build_temp_site(&site, &out));
    assert!(out.join("posts/gone/index.html").is_file());
    write_file(&out.join("stray/file.txt"), "stale");
    write_file(&out.join("stray.txt"), "stale");

    fs::remove_file(site.join("posts/gone.md")).unwrap();
    assert_success(&build_temp_site(&site, &out));

    assert!(
        !out.join("posts/gone").exists(),
        "deleted page still output"
    );
    assert!(!out.join("stray").exists(), "stray dir still present");
    assert!(!out.join("stray.txt").exists(), "stray file still present");
    assert!(out.join("posts/keep/index.html").is_file());
    let index = fs::read_to_string(out.join("posts/index.html")).unwrap();
    assert!(!index.contains("Gone Page"), "{index}");
}

// AC-4.2
#[test]
fn rebuild_removes_newly_drafted_page() {
    let dir = temp_dir("rebuild_removes_newly_drafted_page");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/keep.md"), &page("Keep", false));
    write_file(&site.join("posts/later.md"), &page("Later", false));

    assert_success(&build_temp_site(&site, &out));
    assert!(out.join("posts/later/index.html").is_file());

    write_file(&site.join("posts/later.md"), &page("Later", true));
    assert_success(&build_temp_site(&site, &out));

    assert!(
        !out.join("posts/later").exists(),
        "drafted page still output"
    );
    assert!(out.join("posts/keep/index.html").is_file());
}

/// Snapshot of relative paths under `dir`, sorted, for before/after checks.
fn snapshot(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = list_files(&dir.to_path_buf())
        .into_iter()
        .map(|f| {
            Path::new(&f)
                .strip_prefix(dir)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort();
    files
}

// AC-4.3
#[test]
fn failed_rebuild_keeps_previous_output() {
    let root = root();
    let dir = temp_dir("failed_rebuild_keeps_previous_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));

    assert_success(&build_temp_site(&site, &out));
    write_file(&out.join("marker.txt"), "keep me");
    let before = snapshot(&out);

    // Bad page.
    write_file(&site.join("posts/bad.md"), "# no frontmatter\n");
    assert_failure(&build_temp_site(&site, &out), "bad page");
    assert_eq!(snapshot(&out), before, "bad page rebuild changed output");
    fs::remove_file(site.join("posts/bad.md")).unwrap();

    // Missing templates folder.
    let output = build_with(
        &site,
        &dir.join("no-templates"),
        &root.join("example/meta/assets"),
        &out,
    );
    assert_failure(&output, "missing templates");
    assert_eq!(snapshot(&out), before, "missing templates changed output");
}

// AC-4.4
#[test]
fn build_refuses_to_clean_cwd() {
    let dir = temp_dir("build_refuses_to_clean_cwd");
    default_layout_project(&dir);
    let before = snapshot(&dir);

    let output = run_mango(&["build", "-o", "."], &dir);

    assert_failure(&output, "build -o .");
    let err = stderr(&output);
    assert!(err.contains("'.'"), "{err}");
    assert!(err.contains(dir.to_str().unwrap()), "{err}");
    assert_eq!(snapshot(&dir), before, "files in cwd were changed");
}

// AC-4.5
#[test]
fn build_refuses_to_clean_parent_of_cwd() {
    let dir = temp_dir("build_refuses_to_clean_parent_of_cwd");
    let project = dir.join("project");
    default_layout_project(&project);
    write_file(&dir.join("sibling.txt"), "keep");
    let before = snapshot(&dir);

    let output = run_mango(&["build", "-o", ".."], &project);

    assert_failure(&output, "build -o ..");
    let err = stderr(&output);
    assert!(err.contains("'..'"), "{err}");
    assert_eq!(snapshot(&dir), before, "parent of cwd was changed");
}

// AC-4.6
#[test]
fn build_refuses_output_equal_to_site() {
    let dir = temp_dir("build_refuses_output_equal_to_site");
    let cwd = dir.join("cwd");
    default_layout_project(&cwd);
    let before = snapshot(&cwd);

    // -o site
    let output = run_mango(&["build", "-o", "site"], &cwd);
    assert_failure(&output, "build -o site");
    let err = stderr(&output);
    assert!(err.contains("'site'"), "{err}");
    assert_eq!(snapshot(&cwd), before, "site was changed");

    // -o <folder containing site/>, run from a different cwd.
    let elsewhere = dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let output = run_mango(
        &[
            "build",
            "--site",
            cwd.join("site").to_str().unwrap(),
            "--templates",
            cwd.join("meta/templates").to_str().unwrap(),
            "--assets",
            cwd.join("meta/assets").to_str().unwrap(),
            "-o",
            cwd.to_str().unwrap(),
        ],
        &elsewhere,
    );
    assert_failure(&output, "build -o <parent of site>");
    let err = stderr(&output);
    assert!(err.contains(cwd.join("site").to_str().unwrap()), "{err}");
    assert_eq!(snapshot(&cwd), before, "parent of site was changed");
}

// AC-7.5
#[test]
fn build_fails_when_assets_path_has_no_name() {
    let dir = temp_dir("build_fails_when_assets_path_has_no_name");
    let root = root();
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&out.join("marker.txt"), "keep me");

    for assets in [".", ".."] {
        let output = run_mango(
            &[
                "build",
                "--site",
                site.to_str().unwrap(),
                "--templates",
                root.join("example/meta/templates").to_str().unwrap(),
                "--assets",
                assets,
                "-o",
                out.to_str().unwrap(),
            ],
            &dir,
        );

        assert_failure(&output, &format!("--assets {assets}"));
        let err = stderr(&output);
        assert!(err.contains(&format!("'{assets}'")), "{err}");
        assert_eq!(
            snapshot(&out),
            vec!["marker.txt".to_string()],
            "--assets {assets} changed the output"
        );
    }
}

/// Build once with a private copy of the fixture templates, then return
/// (site, templates, assets, out) so a test can break a template and rebuild.
fn built_site_with_private_templates(test_name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = root();
    let dir = temp_dir(test_name);
    let site = dir.join("site");
    let templates = dir.join("templates");
    let assets = root.join("example/meta/assets");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    copy_dir(&root.join("example/meta/templates"), &templates);

    assert_success(&build_with(&site, &templates, &assets, &out));
    write_file(&out.join("marker.txt"), "keep me");
    (site, templates, assets, out)
}

fn assert_previous_output_intact(out: &Path) {
    for path in [
        "posts/one/index.html",
        "posts/index.html",
        "assets/css/main.css",
        "marker.txt",
    ] {
        assert!(
            out.join(path).is_file(),
            "{path} missing after failed rebuild, got: {}",
            list_files(&out.to_path_buf()).join(", ")
        );
    }
}

// AC-9.2, AC-4.3, AC-9.4
#[test]
fn page_render_error_keeps_previous_output() {
    let (site, templates, assets, out) =
        built_site_with_private_templates("page_render_error_keeps_previous_output");
    let before = snapshot(&out);

    fs::write(templates.join("page.html"), "{{ missing.value }}").unwrap();
    let output = build_with(&site, &templates, &assets, &out);

    assert_failure(&output, "page render error");
    assert!(stderr(&output).contains("page.html"), "{}", stderr(&output));
    assert_previous_output_intact(&out);
    assert_eq!(snapshot(&out), before);
}

// AC-9.3, AC-4.3, AC-9.4
#[test]
fn section_render_error_keeps_previous_output() {
    let (site, templates, assets, out) =
        built_site_with_private_templates("section_render_error_keeps_previous_output");
    let before = snapshot(&out);

    fs::write(templates.join("section.html"), "{{ missing.value }}").unwrap();
    write_file(&site.join("posts/new.md"), &page("New", false));
    let output = build_with(&site, &templates, &assets, &out);

    assert_failure(&output, "section render error");
    assert!(
        stderr(&output).contains("section.html"),
        "{}",
        stderr(&output)
    );
    assert_previous_output_intact(&out);
    assert!(
        !out.join("posts/new").exists(),
        "no new page may be written when a section fails to render"
    );
    assert_eq!(snapshot(&out), before);
}

// AC-5.1 (replaces clean_fails_with_clear_error_for_missing_dir)
#[test]
fn clean_succeeds_for_missing_dir() {
    let dir = temp_dir("clean_succeeds_for_missing_dir");
    let missing = dir.join("nope");

    let output = run_mango(&["clean", "-d", missing.to_str().unwrap()], &dir);

    assert_success(&output);
    assert!(
        stderr(&output).is_empty(),
        "stderr should be empty, got: {}",
        stderr(&output)
    );
}

// AC-5.2
#[test]
fn clean_removes_existing_dir() {
    let dir = temp_dir("clean_removes_existing_dir");
    let dist = dir.join("dist");
    write_file(&dist.join("posts/one/index.html"), "x");

    let output = run_mango(&["clean", "-d", dist.to_str().unwrap()], &dir);

    assert_success(&output);
    assert!(!dist.exists(), "dist should be removed");
}

// AC-5.3
#[test]
fn clean_refuses_cwd() {
    let dir = temp_dir("clean_refuses_cwd");
    let cwd = dir.join("project");
    default_layout_project(&cwd);
    write_file(&dir.join("sibling.txt"), "keep");
    let before = snapshot(&dir);

    for target in [".", "..", "site", "meta", "meta/templates", "meta/assets"] {
        let output = run_mango(&["clean", "-d", target], &cwd);
        assert_failure(&output, &format!("clean -d {target}"));
        let err = stderr(&output);
        assert!(err.contains(&format!("'{target}'")), "{target}: {err}");
        assert_eq!(snapshot(&dir), before, "clean -d {target} deleted files");
    }
}

// AC-5.4
#[test]
fn clean_fails_naming_path_when_target_is_a_file() {
    let dir = temp_dir("clean_fails_naming_path_when_target_is_a_file");
    let file = dir.join("dist");
    write_file(&file, "not a dir");

    let output = run_mango(&["clean", "-d", file.to_str().unwrap()], &dir);

    assert_failure(&output, "clean -d <file>");
    let err = stderr(&output);
    assert!(err.contains(file.to_str().unwrap()), "{err}");
    assert!(file.is_file());
}

// AC-7.1
#[test]
fn help_shows_flag_descriptions() {
    let dir = temp_dir("help_shows_flag_descriptions");

    let build_help = stdout(&run_mango(&["build", "--help"], &dir));
    for text in [
        "Path to the templates directory",
        "Path to the assets directory",
        "Path to the site content directory",
        "Output directory for the built site",
        "Path to the site config file",
        "mango.json",
        "--config",
    ] {
        assert!(build_help.contains(text), "missing '{text}':\n{build_help}");
    }

    let main_help = stdout(&run_mango(&["--help"], &dir));
    for text in [
        "Build the site",
        "Run the dev server",
        "Publish the site",
        "Remove the output directory",
    ] {
        assert!(main_help.contains(text), "missing '{text}':\n{main_help}");
    }

    let clean_help = stdout(&run_mango(&["clean", "--help"], &dir));
    assert!(
        clean_help.contains("Output directory to remove"),
        "{clean_help}"
    );
}

// AC-7.2
#[test]
fn run_command_is_not_implemented_error() {
    let dir = temp_dir("run_command_is_not_implemented_error");
    let output = run_mango(&["run"], &dir);

    assert_failure(&output, "run");
    assert!(
        stderr(&output).contains("the 'run' command is not implemented yet"),
        "{}",
        stderr(&output)
    );
}

// `run` has no options until it is implemented (no dead flags).
#[test]
fn run_command_takes_no_options() {
    let dir = temp_dir("run_command_takes_no_options");
    let output = run_mango(&["run", "--port", "8080"], &dir);

    assert_failure(&output, "run --port");
    assert!(
        stderr(&output).contains("unexpected argument '--port'"),
        "{}",
        stderr(&output)
    );
}

// Two builds of the same input are byte-identical.
#[test]
fn fixture_build_is_deterministic() {
    let first = fixture_dist();
    let second = temp_dir("fixture_build_is_deterministic").join("dist");

    let output = run_mango(
        &[
            "build",
            "--site",
            "example/site",
            "--templates",
            "example/meta/templates",
            "--assets",
            "example/meta/assets",
            "-o",
            second.to_str().unwrap(),
            "--config",
            "example/mango.json",
        ],
        &root(),
    );
    assert_success(&output);

    let files = snapshot(first);
    assert_eq!(
        files,
        snapshot(&second),
        "the two builds wrote different files"
    );
    for file in &files {
        assert!(
            fs::read(first.join(file)).unwrap() == fs::read(second.join(file)).unwrap(),
            "{file} differs between two builds of the same input"
        );
    }
}

// AC-7.3
#[test]
fn publish_command_is_not_implemented_error() {
    let dir = temp_dir("publish_command_is_not_implemented_error");
    let output = run_mango(&["publish"], &dir);

    assert_failure(&output, "publish");
    assert!(
        stderr(&output).contains("the 'publish' command is not implemented yet"),
        "{}",
        stderr(&output)
    );
}

// Every root-relative href/src in the fixture output points at a file that
// was written: the broken-link check a crawler would do.
#[test]
fn fixture_internal_links_resolve() {
    let out = fixture_dist();
    let mut checked = 0;

    for file in snapshot(out).into_iter().filter(|f| f.ends_with(".html")) {
        let html = fs::read_to_string(out.join(&file)).unwrap();
        for attr in ["href=\"", "src=\""] {
            for rest in html.split(attr).skip(1) {
                let target = rest.split('"').next().unwrap();
                if !target.starts_with('/') {
                    continue; // external links and in-page anchors
                }
                let path = target
                    .split(['#', '?'])
                    .next()
                    .unwrap()
                    .trim_start_matches('/');
                let resolved = if path.is_empty() || path.ends_with('/') {
                    out.join(path).join("index.html")
                } else {
                    out.join(path)
                };
                assert!(resolved.is_file(), "{file}: broken link '{target}'");
                checked += 1;
            }
        }
    }

    assert!(checked > 100, "only {checked} internal links checked");
}

#[test]
fn build_fails_on_file_vs_folder_conflict_keeping_output() {
    let dir = temp_dir("build_fails_on_file_vs_folder_conflict_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    let config = dir.join("mango.json");
    write_file(&config, r#"{"base_url": "https://example.com"}"#);
    write_file(&site.join("posts/one.md"), &page("One", false));
    assert_success(&build_temp_site_with_config(&site, &out, &config));
    let before = snapshot(&out);

    // dist/feed.xml/index.html would need dist/feed.xml to be a directory.
    write_file(&site.join("feed.xml.md"), &page("Feed Page", false));
    let output = build_temp_site_with_config(&site, &out, &config);

    assert_failure(&output, "feed.xml.md vs feed.xml");
    let err = stderr(&output);
    assert!(
        err.contains("written as a file by RSS feed, but page 'feed.xml'"),
        "{err}"
    );
    assert_eq!(
        snapshot(&out),
        before,
        "a conflict must not touch the output"
    );
}

#[test]
fn build_fails_when_page_lands_on_an_asset_keeping_output() {
    let dir = temp_dir("build_fails_when_page_lands_on_an_asset_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    assert_success(&build_temp_site(&site, &out));
    let before = snapshot(&out);

    // The fixture assets contain css/main.css, copied to assets/css/main.css.
    write_file(&site.join("assets/css/main.css.md"), &page("Clash", false));
    let output = build_temp_site(&site, &out);

    assert_failure(&output, "page nested under an asset file");
    let err = stderr(&output);
    assert!(err.contains("asset 'css/main.css'"), "{err}");
    assert!(err.contains("page 'assets/css/main.css'"), "{err}");
    assert_eq!(
        snapshot(&out),
        before,
        "a conflict must not touch the output"
    );
}

#[test]
fn build_fails_on_missing_assets_folder_keeping_output() {
    let root = root();
    let dir = temp_dir("build_fails_on_missing_assets_folder_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    assert_success(&build_temp_site(&site, &out));
    let before = snapshot(&out);

    let output = build_with(
        &site,
        &root.join("example/meta/templates"),
        &dir.join("missing-assets"),
        &out,
    );

    assert_failure(&output, "missing assets folder");
    assert!(
        stderr(&output).contains("missing-assets"),
        "{}",
        stderr(&output)
    );
    assert_eq!(snapshot(&out), before, "missing assets changed output");
}

// AC-10.4
#[cfg(unix)]
#[test]
fn build_fails_on_symlinked_asset_folder_keeping_output() {
    use std::os::unix::fs::symlink;

    let root = root();
    let dir = temp_dir("build_fails_on_symlinked_asset_folder_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    let assets = dir.join("assets");
    write_file(&site.join("posts/one.md"), &page("One", false));
    copy_dir(&root.join("example/meta/assets"), &assets);
    let templates = root.join("example/meta/templates");

    assert_success(&build_with(&site, &templates, &assets, &out));
    write_file(&out.join("marker.txt"), "keep me");
    let before = snapshot(&out);

    // A symlink to a folder: planned as a file today, so `fs::copy` used to
    // fail after the output folder had already been emptied.
    write_file(&dir.join("shared/reset.css"), "* { margin: 0 }");
    symlink(dir.join("shared"), assets.join("vendor")).unwrap();
    let output = build_with(&site, &templates, &assets, &out);

    assert_failure(&output, "symlinked asset folder");
    let err = stderr(&output);
    assert!(err.contains("asset 'vendor'"), "{err}");
    assert!(err.contains("symlink to a directory"), "{err}");
    assert_eq!(
        snapshot(&out),
        before,
        "a symlinked asset folder must not touch the output"
    );
}

// AC-10.8
#[cfg(unix)]
#[test]
fn build_copies_symlinked_asset_file() {
    use std::os::unix::fs::symlink;

    let root = root();
    let dir = temp_dir("build_copies_symlinked_asset_file");
    let site = dir.join("site");
    let out = dir.join("dist");
    let assets = dir.join("assets");
    write_file(&site.join("posts/one.md"), &page("One", false));
    copy_dir(&root.join("example/meta/assets"), &assets);
    write_file(&dir.join("shared/reset.css"), "* { margin: 0 }");
    symlink(dir.join("shared/reset.css"), assets.join("reset.css")).unwrap();

    assert_success(&build_with(
        &site,
        &root.join("example/meta/templates"),
        &assets,
        &out,
    ));

    let copied = out.join("assets/reset.css");
    assert!(!copied.is_symlink(), "the link target must be copied");
    assert_eq!(fs::read_to_string(copied).unwrap(), "* { margin: 0 }");
}

// AC-10.9
#[cfg(unix)]
#[test]
fn build_accepts_symlinked_assets_root() {
    use std::os::unix::fs::symlink;

    let root = root();
    let dir = temp_dir("build_accepts_symlinked_assets_root");
    let site = dir.join("site");
    let out = dir.join("dist");
    let assets = dir.join("shared-theme/assets");
    write_file(&site.join("posts/one.md"), &page("One", false));
    copy_dir(&root.join("example/meta/assets"), &assets);
    // The assets folder passed to --assets is itself a symlink to a folder.
    let link = dir.join("meta/assets");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    symlink(&assets, &link).unwrap();

    assert_success(&build_with(
        &site,
        &root.join("example/meta/templates"),
        &link,
        &out,
    ));

    assert!(
        out.join("assets/css/main.css").is_file(),
        "{:?}",
        snapshot(&out)
    );
}

// AC-11.9
#[cfg(unix)]
#[test]
fn build_fails_on_symlinked_content_folder_keeping_output() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("build_fails_on_symlinked_content_folder_keeping_output");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));

    assert_success(&build_temp_site(&site, &out));
    write_file(&out.join("marker.txt"), "keep me");
    let before = snapshot(&out);

    // A symlink pointing at the site folder itself: the loader used to follow
    // it, publishing ~40 duplicate copies of the site with exit status 0.
    symlink(&site, site.join("loop")).unwrap();
    let output = build_temp_site(&site, &out);

    assert_failure(&output, "symlinked content folder");
    let err = stderr(&output);
    assert!(err.contains("content folder 'loop'"), "{err}");
    assert!(err.contains("symlink to a directory"), "{err}");
    assert_eq!(
        snapshot(&out),
        before,
        "a symlinked content folder must not touch the output"
    );
}

// AC-11.10
#[cfg(unix)]
#[test]
fn build_reads_symlinked_markdown_file() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("build_reads_symlinked_markdown_file");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    write_file(&dir.join("shared/post.md"), &page("Shared", false));
    symlink(dir.join("shared/post.md"), site.join("posts/linked.md")).unwrap();

    assert_success(&build_temp_site(&site, &out));

    // The slug follows the link's path under the site folder.
    let rendered = out.join("posts/linked/index.html");
    assert!(rendered.is_file(), "{:?}", snapshot(&out));
    assert!(
        fs::read_to_string(rendered).unwrap().contains("Shared"),
        "the link target's content must be rendered"
    );
}

#[test]
fn build_accepts_any_case_md_and_markdown_extensions() {
    let dir = temp_dir("build_accepts_any_case_md_and_markdown_extensions");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(&site.join("one.md"), &page("One", false));
    write_file(&site.join("upper.MD"), &page("Upper", false));
    write_file(&site.join("long.markdown"), &page("Long", false));
    write_file(&site.join("mixed.Markdown"), &page("Mixed", false));
    write_file(&site.join("notes.txt"), "not a page");

    assert_success(&build_temp_site(&site, &out));
    for slug in ["one", "upper", "long", "mixed"] {
        assert!(
            out.join(slug).join("index.html").is_file(),
            "{slug}: {:?}",
            snapshot(&out)
        );
    }
    assert!(!out.join("notes").exists());
}

#[test]
fn build_fails_on_invalid_file_name_naming_file() {
    for (i, (name, segment, draft)) in [
        ("my posts/one.md", "my posts", false),
        ("héllo.md", "héllo", true),
    ]
    .into_iter()
    .enumerate()
    {
        let dir = temp_dir(&format!("build_fails_on_invalid_file_name_{i}"));
        let site = dir.join("site");
        write_file(&site.join(name), &page("Bad", draft));

        let output = build_temp_site(&site, &dir.join("dist"));

        assert_failure(&output, name);
        let err = stderr(&output);
        assert!(
            err.contains(&format!("invalid file name '{segment}'")),
            "{err}"
        );
        assert!(!dir.join("dist").exists(), "{name}: nothing may be written");
    }
}

#[test]
fn build_accepts_utf8_bom_before_frontmatter() {
    let dir = temp_dir("build_accepts_utf8_bom_before_frontmatter");
    let site = dir.join("site");
    let out = dir.join("dist");
    write_file(
        &site.join("bom.md"),
        &format!("\u{feff}{}", page("Bom", false)),
    );

    assert_success(&build_temp_site(&site, &out));
    let html = fs::read_to_string(out.join("bom/index.html")).unwrap();
    assert!(html.contains("<h1>Bom</h1>"), "{html}");
}

fn list_files(dir: &PathBuf) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(list_files(&path));
            } else {
                files.push(path.to_string_lossy().into_owned());
            }
        }
    }
    files
}
