use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

// End-to-end tests: run the real binary. main() prints errors to stderr and
// exits non-zero on failure, so exit status is a meaningful check.

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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Build a temp site using the committed fixture templates and assets.
fn build_temp_site(site: &Path, out: &Path) -> Output {
    let root = root();
    run_mango(
        &[
            "build",
            "--site",
            site.to_str().unwrap(),
            "--templates",
            root.join("test/meta/templates").to_str().unwrap(),
            "--assets",
            root.join("test/meta/assets").to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ],
        &root,
    )
}

// AC-2.4, AC-2.8, AC-4.3, AC-5.2
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
}

// AC-1.5, AC-2.4
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

// AC-3.2
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

// AC-2.5, AC-2.4
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

// AC-2.6, AC-2.4
#[test]
fn clean_fails_with_clear_error_for_missing_dir() {
    let dir = temp_dir("clean_fails_with_clear_error_for_missing_dir");
    let missing = dir.join("nope");
    let os_message = fs::remove_dir_all(&missing).unwrap_err().to_string();

    let output = run_mango(&["clean", "-d", missing.to_str().unwrap()], &dir);

    assert!(
        !output.status.success(),
        "clean should fail for a missing dir"
    );
    let err = stderr(&output);
    assert!(
        err.contains(missing.to_str().unwrap()),
        "stderr should name the path, got: {err}"
    );
    assert!(
        err.contains(&os_message),
        "stderr should include the OS error '{os_message}', got: {err}"
    );
    assert!(
        stdout(&output).is_empty(),
        "errors must not go to stdout, got: {}",
        stdout(&output)
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
