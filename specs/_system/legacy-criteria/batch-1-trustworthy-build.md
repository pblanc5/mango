# Batch 1: make `mango build` trustworthy

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260914-004144-tasks-md/01-plan.v1.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260914-004144-tasks-md`
- Groups defined here: `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-5`, `AC-6`
- Criteria: 25

## Requirements

### REQ-1 Load errors at any depth fail the build and name the file
Any read or parse error for any `.md` file at any directory depth must stop the build, and the message must name the offending file. The site-directory check runs once.
- AC-1.1 If a site has `posts/bad.md` with no frontmatter, `loader::load` returns `Err`, and the error's display string contains `bad.md`.
- AC-1.2 If a site has `posts/bad.md` with malformed JSON frontmatter, `loader::load` returns `Err` of kind `MangoError::Frontmatter`, and the display string contains both `bad.md` and the underlying serde_json message.
- AC-1.3 Valid pages nested two levels deep (e.g. `a/b/page.md`) are loaded, with slug `a/b/page`.
- AC-1.4 If the `site` path is not a directory, `loader::load` returns `MangoError::General` naming the path.
- AC-1.5 End to end: `mango build` against a temp site with `posts/bad.md` (no frontmatter) and `posts/good.md` exits non-zero, and stderr contains `bad.md`.
- AC-1.6 An I/O failure reading a directory or file during traversal produces an error that includes that path. This is covered by the path-carrying variant from REQ-2 (AC-2.2).

### REQ-2 Errors show their cause and the process exits non-zero
- AC-2.1 `MangoError::Io` displays the wrapped `std::io::Error` message, e.g. `Mango I/O Error: <io message>`.
- AC-2.2 A new path-carrying I/O variant displays both the path and the underlying io error message.
- AC-2.3 `MangoError::Template` displays the top-level Tera message followed by every message in its `source()` chain. For example, `tera::Error::chain("Failed to render 'page.html'", tera::Error::msg("Variable `x` not found"))` displays text containing both strings.
- AC-2.4 On failure, `main` writes the error to stderr (nothing to stdout) and exits with status 1. On success it exits 0.
- AC-2.5 `mango build --templates <nonexistent>` (other paths valid) exits non-zero, and stderr contains the nonexistent templates path.
- AC-2.6 `mango clean -d <nonexistent>` exits non-zero, and stderr contains the path plus the OS error text. This is the "clear error message" allowance; `clean` behavior is not otherwise changed.
- AC-2.7 Path-level I/O failures in `cli::clean`, `output::write` (create dir, write file), `loader` (read_dir, read file) and `assets::build` (create dir, read_dir, copy) use the path-carrying variant.
- AC-2.8 `tests/build.rs` no longer claims main exits 0 on error. The fixture build test asserts success and prints stderr in its failure messages.

### REQ-3 Draft pages are excluded
- AC-3.1 `loader::load` does not return pages whose frontmatter has `"draft": true`. Non-draft siblings are still returned.
- AC-3.2 End to end: with a temp site containing `posts/published.md` (draft false) and `posts/secret.md` (draft true, title "Secret Draft"):
  - `dist/posts/published/index.html` exists.
  - `dist/posts/secret/` does not exist.
  - `dist/posts/index.html` contains neither `secret` nor `Secret Draft`.
- AC-3.3 Draft files are still parsed. A draft with malformed frontmatter still fails the build, same as REQ-1.

### REQ-4 Assets are copied recursively and completely
- AC-4.1 Every file is copied to the matching relative path under the destination: files in `a_dir/`, `b_dir/`, and three top-level files, and those subdirectories may themselves contain files.
- AC-4.2 If a single copy fails (for example, the destination path for a file already exists as a directory), `assets::build` returns `Err`, and the display string contains the failing path. Nothing is printed to stdout.
- AC-4.3 End to end: the fixture build still produces `assets/minimal/main.css`.

### REQ-5 `page.description` reaches templates
- AC-5.1 `render_page` puts a `page` object into the context whose `description` equals the frontmatter description.
- AC-5.2 End to end: the fixture build's `posts/post_one/index.html` contains `<meta name="description" content="my first post">`.

### REQ-6 Documentation and formatting
- AC-6.1 CLAUDE.md no longer lists these as known issues: the asset-copy recursion bug, main printing to stdout and exiting 0, `fs::copy` failures being ignored, and `let _ =` in loader.rs. Any text relying on those behaviors (e.g. "the integration test asserts on output files instead") is corrected. The home-page stub, missing `config` context, and clap `//` items stay.
- AC-6.2 After all functional changes pass, `cargo fmt` is applied, and `cargo fmt --check` passes.
- AC-6.3 `cargo build`, `cargo test` and `cargo clippy` pass, with no new clippy warnings.

## Verified by

From the run's test report (`03-test.v1.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-1.1 | new | `content::loader::tests::subdirectory_page_without_frontmatter_is_an_error_naming_the_file` (`posts/bad.md` next to `good.md`, asserts Err containing `bad.md`) | pass |
| AC-1.2 | new | `content::loader::tests::subdirectory_malformed_frontmatter_is_an_error_naming_the_file` (asserts `Frontmatter` variant, `bad.md`, and the exact serde_json message) | pass |
| AC-1.3 | new | `content::loader::tests::loads_pages_nested_two_levels_deep` (slug `a/b/page`) | pass |
| AC-1.4 | new | `content::loader::tests::non_directory_site_is_a_general_error` (`General` variant, path in message) | pass |
| AC-1.5 | new | `build_fails_when_subdirectory_page_lacks_frontmatter` (non-zero exit, `bad.md` in stderr) | pass |
| AC-1.6 | new | `error::tests::io_path_display_includes_path_and_cause`, plus code review: every `read_dir`, entry and `read_to_string` call in `loader::traverse` maps through `io_at` | pass |
| AC-2.1 | new | `error::tests::io_display_includes_underlying_message` (exact `Mango I/O Error: disk went away`) | pass |
| AC-2.2 | new | `error::tests::io_path_display_includes_path_and_cause` | pass |
| AC-2.3 | new | `error::tests::template_display_includes_source_chain`, plus the ad-hoc Tera render error check | pass |
| AC-2.4 | new | stdout-empty and non-zero exit assertions in the 3 failure e2e tests; `status.success()` in the fixture and draft tests; ad-hoc runs gave exit 1 with empty stdout and exit 0 on success | pass |
| AC-2.5 | new | `build_fails_with_missing_templates_dir`, plus the ad-hoc check | pass |
| AC-2.6 | new | `clean_fails_with_clear_error_for_missing_dir` (path and the platform's OS error text), plus the ad-hoc check | pass |
| AC-2.7 | new | Code review: `io_at` is used in `cli::clean` (remove_dir_all), `output::write` (create_dir_all, write), `loader::traverse` (read_dir, entry, read_to_string) and `assets::build` (create_dir_all, read_dir, entry, file_type, copy). The path content is covered by `copy_failure_is_an_error_naming_the_path` and the clean test | pass |
| AC-2.8 | changed | `tests/build.rs` header comment now says errors go to stderr with a non-zero exit. `build_generates_site_from_fixture` asserts success and prints stderr on failure | pass |
| AC-3.1 | new | `content::loader::tests::draft_pages_are_excluded` (slugs are exactly `["posts/published"]`) | pass |
| AC-3.2 | new | `build_excludes_draft_pages` (published page exists, `posts/secret` absent, index has neither `secret` nor `Secret Draft`) | pass |
| AC-3.3 | new | `content::loader::tests::malformed_draft_is_still_an_error`, plus the ad-hoc malformed-draft e2e (exit 1, path in stderr) | pass |
| AC-4.1 | changed | `build::generate::assets::tests::copies_all_files_in_all_nested_directories` (6 files including nested, content compared), plus the ad-hoc tree diff | pass |
| AC-4.2 | new | `build::generate::assets::tests::copy_failure_is_an_error_naming_the_path`. Code review found no `println!` left in assets.rs | pass |
| AC-4.3 | baseline | `build_generates_site_from_fixture` (`assets/minimal/main.css`) | pass |
| AC-5.1 | new | `render::template::tests::page_context_includes_description` | pass |
| AC-5.2 | new | `build_generates_site_from_fixture` meta-tag assertion, plus the ad-hoc grep (count 1) | pass |
| AC-6.1 | changed | Manual review of `/home/roguestar/workspace/mango/CLAUDE.md`. Removed from known issues: asset recursion, stdout/exit 0, ignored `fs::copy` failures, `let _ =`. Kept: home stub, `config` context, clap `//`. The "asserts on output files instead" wording is corrected | pass |
| AC-6.2 | new | `cargo fmt --check` exit 0 | pass |
| AC-6.3 | new | `cargo build`, `cargo test`, `cargo clippy` (and `--all-targets`) exit 0. The only warning is the old `home::build` dead-code warning | pass |
