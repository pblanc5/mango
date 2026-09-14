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
        &root.join("test/meta/templates"),
        &root.join("test/meta/assets"),
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
        &root.join("test/meta/templates"),
        &dir.join("meta/templates"),
    );
    copy_dir(&root.join("test/meta/assets"), &dir.join("meta/assets"));
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

// AC-2.4, AC-2.8, AC-4.3, AC-5.2 (batch 1); AC-1.8, AC-2.6, AC-7.4 (batch 2)
#[test]
fn build_generates_site_from_fixture() {
    let root = root();
    let out = root.join("target").join("integration-dist");
    let _ = fs::remove_dir_all(&out);

    let output = run_mango(
        &[
            "build",
            "--site",
            "test/site",
            "--templates",
            "test/meta/templates",
            "--assets",
            "test/meta/assets",
            "-o",
            out.to_str().unwrap(),
        ],
        &root,
    );

    assert!(
        output.status.success(),
        "mango build exited with failure\nstderr: {}",
        stderr(&output)
    );

    let expected = [
        "posts/post_one/index.html",
        "posts/post_two/index.html",
        "posts/test/index.html",
        "projects/mango/index.html",
        "posts/index.html",
        "projects/index.html",
        "assets/minimal/main.css",
    ];

    for path in expected {
        assert!(
            out.join(path).is_file(),
            "expected {path} in output, got: {}\nstderr: {}",
            list_files(&out).join(", "),
            stderr(&output)
        );
    }

    let page = fs::read_to_string(out.join("posts/post_one/index.html")).unwrap();
    assert!(
        page.contains("Post One"),
        "rendered page should contain the post title"
    );
    // AC-5.2
    assert!(
        page.contains(r#"<meta name="description" content="my first post">"#),
        "rendered page should contain the frontmatter description meta tag, got:\n{page}"
    );
    // AC-1.8
    assert!(
        page.contains("<time>2026-01-24</time>"),
        "rendered page should contain the formatted date, got:\n{page}"
    );

    // AC-2.6: all three posts share a date, so they are ordered by title.
    let index = fs::read_to_string(out.join("posts/index.html")).unwrap();
    let offsets: Vec<usize> = ["Post One", "Post Two", "Test Page"]
        .iter()
        .map(|t| {
            index
                .find(t)
                .unwrap_or_else(|| panic!("'{t}' missing from:\n{index}"))
        })
        .collect();
    assert!(
        offsets[0] < offsets[1] && offsets[1] < offsets[2],
        "posts index must list Post One, Post Two, Test Page in order, got:\n{index}"
    );
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
            "test/site",
            "--templates",
            templates.to_str().unwrap(),
            "--assets",
            "test/meta/assets",
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
    assert!(!html.contains("<time>"), "undated page has <time>:\n{html}");
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
        &root.join("test/meta/assets"),
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
                root.join("test/meta/templates").to_str().unwrap(),
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
    let assets = root.join("test/meta/assets");
    let out = dir.join("dist");
    write_file(&site.join("posts/one.md"), &page("One", false));
    copy_dir(&root.join("test/meta/templates"), &templates);

    assert_success(&build_with(&site, &templates, &assets, &out));
    write_file(&out.join("marker.txt"), "keep me");
    (site, templates, assets, out)
}

fn assert_previous_output_intact(out: &Path) {
    for path in [
        "posts/one/index.html",
        "posts/index.html",
        "assets/minimal/main.css",
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
