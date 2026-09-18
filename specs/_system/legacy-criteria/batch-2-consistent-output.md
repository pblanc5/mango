# Batch 2 — consistent, correct output

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260914-011654-tasks-md/01-plan.v2.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260914-011654-tasks-md`
- Groups defined here: `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-5`, `AC-6`, `AC-7`, `AC-8`, `AC-9`
- Criteria: 45

## Requirements

### REQ-1 Strict date parsing and validation
Parse the frontmatter `date` into a real date at load time so it can be sorted and checked. Templates must not notice the change.
- AC-1.1 A frontmatter `date` of `"2026-01-24"` loads as `Page.date == Some(NaiveDate 2026-01-24)`. A leap day such as `"2024-02-29"` is accepted.
- AC-1.2 Each of these makes `loader::load` fail with `MangoError::Frontmatter`, and the message contains both the file path and the bad value: `"2026/01/24"`, `"2026-1-24"`, `"26-01-24"`, `"2026-01-24T10:00:00"`, `"+2026-01-24"`, `" 2026-01-24"`, `""`.
- AC-1.3 An impossible date (`"2026-02-30"`, `"2026-13-01"`) fails the same way as AC-1.2.
- AC-1.4 A missing `date` field, or `"date": null`, loads as `Page.date == None` without error.
- AC-1.5 In the template contexts, `page.date` and `section.pages[].date` serialize as the string `"YYYY-MM-DD"` when a date is set. With no date they serialize as `""`, so `{% if page.date %}` is false.
- AC-1.6 A draft page with an invalid date still fails the load, the same as malformed draft frontmatter today.
- AC-1.7 End to end: `mango build` on a site containing `posts/bad_date.md` with `"date": "2026-02-30"` exits non-zero. Stderr contains `bad_date.md` and `2026-02-30`. Stdout is empty.
- AC-1.8 End to end: the fixture page `posts/post_one/index.html` still contains `<time>2026-01-24</time>`. A page with no date renders no `<time>` element.

### REQ-2 Deterministic ordering of section listings and section generation
- AC-2.1 Within a section, dated pages are ordered newest date first.
- AC-2.2 Undated pages come after all dated pages in that section.
- AC-2.3 Pages with the same date (or both undated) are ordered by `title` ascending (plain `String` ordering), then by `slug` ascending.
- AC-2.4 `SectionIndex.sections` is a `BTreeMap`. `generate::section::build` returns render items in ascending section-slug order, for example `["a", "a/b", "posts", "projects"]`.
- AC-2.5 Unit tests feed pages to `build_section_index` in an order different from the expected sorted order, and assert the exact slug order. They do not rely on `read_dir`.
- AC-2.6 End to end: in the fixture build, `posts/index.html` lists "Post One", "Post Two", "Test Page" in that order (checked by byte offset of each title).

### REQ-3 Output collisions fail the build before anything is written
- AC-3.1 If two render items map to the same output file, the build fails with `MangoError::General`. For example, top-level `posts.md` (a page) and the `posts` section index both map to `dist/posts/index.html`. The message contains the output path, the page source (`page 'posts'`) and the section source (`section index 'posts'`).
- AC-3.2 When a collision is found, no output files are written and the output folder is not cleaned. A marker file already in the output folder is still there, and `posts/one/index.html` is not created.
- AC-3.3 `posts/index.md` alongside `posts/one.md` is not a collision. The build succeeds and writes both `posts/index/index.html` and `posts/index.html`.
- AC-3.4 A set of render items with distinct output paths passes the collision check.

### REQ-4 `build` cleans the output folder, with a safety check
- AC-4.1 If a page is deleted and the site rebuilt into the same output folder, that page's output directory is gone, it no longer appears in the section index, and any other stray file previously in the output folder is gone.
- AC-4.2 If a page is marked `"draft": true` and the site rebuilt, that page's output is gone.
- AC-4.3 If the rebuild fails at page loading (for example a page without frontmatter), at template loading (missing templates folder), or at template rendering (see REQ-9), the previous output is left untouched.
- AC-4.4 `build -o .` fails with `MangoError::General` naming the output path and the cwd, and deletes nothing. The site and template files in the cwd still exist.
- AC-4.5 `build -o <parent of cwd>` fails the same way and deletes nothing.
- AC-4.6 `build` fails and deletes nothing, with a `MangoError::General` naming both paths, when `-o` equals or is a parent of the `--site`, `--templates` or `--assets` folder. This applies to `-o site` and to `-o <folder containing site/>`.
- AC-4.7 A build into an output folder that doesn't exist yet succeeds and creates it.
- AC-4.8 If the output path exists but is a regular file, the build fails with an error naming the path.
- AC-4.9 Cleaning removes the output folder's contents but not the folder itself. Symlinks inside it are removed without following them.

### REQ-5 `mango clean` behavior and safety
- AC-5.1 `mango clean -d <missing>` exits 0 and prints nothing to stderr. This replaces `clean_fails_with_clear_error_for_missing_dir`.
- AC-5.2 `mango clean -d <existing dir>` removes the folder and exits 0.
- AC-5.3 `mango clean -d .` fails with an error naming the path, exits non-zero, and deletes nothing. So does a parent of the cwd, or a folder that equals or contains the default `site`, `meta/templates` or `meta/assets` under the cwd.
- AC-5.4 Any other failure exits non-zero with the path in stderr. For example, `-d` pointing at a regular file.

### REQ-6 Loader leftovers
- AC-6.1 A directory named `notes.md/` containing `inner.md` is traversed. The load succeeds and yields slug `notes.md/inner`. Only regular files (`path.is_file()`) ending in `.md` are treated as pages.
- AC-6.2 The "failed to generate frontmatter for page …" message is built with `path.display()`. The existing test asserting that it contains `bad.md` still passes.

### REQ-7 CLI cleanup
- AC-7.1 `mango build --help` stdout contains "Path to the templates directory". It also contains help text for `--assets`, `--site` and `--output`. `mango --help` shows descriptions for `build`, `run`, `publish` and `clean`. `mango clean --help` shows help for `--dist`.
- AC-7.2 `mango run` exits non-zero with stderr containing `the 'run' command is not implemented yet`. `ServerOpts` keeps its `address` and `port` fields.
- AC-7.3 `mango publish` exits non-zero with stderr containing `the 'publish' command is not implemented yet`.
- AC-7.4 `output::append_to_path` no longer exists. Assets are still copied to `dist.join(<assets folder name>)`, so the fixture build still produces `assets/minimal/main.css`.
- AC-7.5 `build --assets .` and `build --assets ..` fail with an error naming the given assets path. Nothing is copied into the output root, and the previous output is not cleaned.

### REQ-8 Dependency and documentation
- AC-8.1 `Cargo.toml` adds only `chrono` at version `0.4.43`, the version in `Cargo.lock`. `Cargo.lock` gains no new `[[package]]` entries.
- AC-8.2 CLAUDE.md is updated:
  - It describes date validation, sorted listings, collision errors, and output cleaning with the safety check.
  - It says every page and section is rendered in memory before the output folder is cleaned.
  - It says `run`/`publish` return errors and `clean` accepts a missing folder.
  - The clap `//` known issue is removed.
  - The home-stub and `config` known issues are kept.
- AC-8.3 `cargo build`, `cargo test`, `cargo clippy --all-targets` and `cargo fmt --check` all pass. Clippy shows no warnings other than the existing `home::build` dead-code warning.

### REQ-9 Render everything in memory before cleaning
A Tera render error must not destroy a working build. Today `output::write` renders and writes one item at a time. Once cleaning is added (REQ-4), a render error would leave the output folder empty or half written. Every page and section index is rendered to a string before the safety check, the clean and any writes.
- AC-9.1 `output::render` takes the Tera instance, the output folder and the render items. It returns one rendered file per item, holding the output path and the HTML string, and touches no files. If any item fails to render, it returns `Err(MangoError::Template)` and produces no files.
- AC-9.2 End to end: a site is built successfully into an output folder. It is then rebuilt with a templates folder whose `page.html` references an undefined variable (for example `{{ missing.value }}`). That template parses when loaded but fails when rendered. The rebuild exits non-zero, stderr contains `page.html`, and the output folder is unchanged: the earlier page file, the section index, the copied assets and a stray marker file all still exist.
- AC-9.3 End to end: a render error in `section.html` only, with `page.html` valid, leaves the previous output untouched in the same way. No new page files are written.
- AC-9.4 In `cli::build`, the order is: load pages → load templates → resolve the assets destination → build render items → collision check → render all items to strings → safety check → clean → write rendered files → copy assets. Nothing is deleted or written before rendering has succeeded for every item.

## Verified by

From the run's test report (`03-test.v1.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-1.1 | new | `page::tests::parse_date_accepts_valid_and_leap_day_dates`, `loader::tests::valid_date_is_parsed` (2026-01-24 and 2024-02-29) | pass |
| AC-1.2 | new | `page::tests::parse_date_rejects_wrong_formats` (all 7 values, `Frontmatter`, value in message), `loader::tests::invalid_date_format_is_frontmatter_error_naming_file_and_value` (file + value), `page::tests::new_rejects_invalid_date` | pass |
| AC-1.3 | new | `page::tests::parse_date_rejects_impossible_dates`, `loader::tests::impossible_date_is_frontmatter_error_naming_file_and_value` | pass |
| AC-1.4 | changed | `page::tests::new_defaults_missing_date_and_tags`, `loader::tests::missing_or_null_date_is_undated` (missing and `null`) | pass |
| AC-1.5 | new | `template::tests::page_context_date_is_formatted_or_empty`, `index::section::tests::summary_date_serializes_formatted_or_empty`, `page::tests::page_date_serializes_formatted_or_empty` | pass |
| AC-1.6 | new | `loader::tests::invalid_date_in_draft_is_still_an_error` | pass |
| AC-1.7 | new | `build_fails_on_invalid_date_naming_file_and_value` (non-zero, stderr has `bad_date.md` and `2026-02-30`, stdout empty via `assert_failure`) | pass |
| AC-1.8 | new | `build_generates_site_from_fixture` (`<time>2026-01-24</time>`), `build_undated_page_renders_without_time` | pass |
| AC-2.1 | new | `index::section::tests::pages_sorted_newest_first` (unsorted input) | pass |
| AC-2.2 | new | `index::section::tests::undated_pages_come_last` | pass |
| AC-2.3 | new | `index::section::tests::ties_broken_by_title_then_slug` (dated and undated ties) | pass |
| AC-2.4 | new | `index::section::tests::sections_iterate_in_slug_order`, `generate::section::tests::render_items_in_section_slug_order` (`["a","a/b","posts","projects"]`); `SectionIndex.sections` is a `BTreeMap` | pass |
| AC-2.5 | new | reviewed the inputs to the tests above: all unsorted, no `read_dir` | pass |
| AC-2.6 | changed | `build_generates_site_from_fixture` (byte offsets), plus the scratchpad fixture build | pass |
| AC-3.1 | new | `output::tests::collision_names_path_and_both_sources`, `build_fails_on_page_and_section_output_collision` | pass |
| AC-3.2 | new | `build_fails_on_page_and_section_output_collision` (marker survives, `posts/one/index.html` absent) | pass |
| AC-3.3 | new | `output::tests::index_md_is_not_a_collision`, `build_allows_index_md_inside_section` | pass |
| AC-3.4 | new | `output::tests::distinct_paths_pass` | pass |
| AC-4.1 | new | `rebuild_removes_deleted_page_and_stale_files`, plus the scratchpad stray-file check | pass |
| AC-4.2 | new | `rebuild_removes_newly_drafted_page` | pass |
| AC-4.3 | new | `failed_rebuild_keeps_previous_output` (bad page, missing templates; snapshot equality), `page_render_error_keeps_previous_output`, `section_render_error_keeps_previous_output` | pass |
| AC-4.4 | new | `cli::tests::refuses_cwd`, `build_refuses_to_clean_cwd` (cwd is a temp folder), plus the scratchpad `build -o .` check | pass |
| AC-4.5 | new | `cli::tests::refuses_parent_of_cwd`, `build_refuses_to_clean_parent_of_cwd` | pass |
| AC-4.6 | new | `cli::tests::refuses_protected_path_or_its_parent`, `build_refuses_output_equal_to_site` (`-o site` and `-o <parent of site>`) | pass |
| AC-4.7 | new | `cli::tests::missing_target_is_safe_and_clean_contents_is_noop`, `build_excludes_draft_pages` (fresh output folder) | pass |
| AC-4.8 | new | `cli::tests::clean_contents_errors_when_target_is_a_file` | pass |
| AC-4.9 | new | `cli::tests::clean_contents_empties_but_keeps_dir` (folder kept, unix symlink target survives) | pass |
| AC-5.1 | changed | `clean_succeeds_for_missing_dir` (exit 0, stderr empty) | pass |
| AC-5.2 | changed | `clean_removes_existing_dir` | pass |
| AC-5.3 | new | `clean_refuses_cwd` (`.`, `..`, `site`, `meta`, `meta/templates`, `meta/assets`; snapshot unchanged) | pass |
| AC-5.4 | changed | `clean_fails_naming_path_when_target_is_a_file` | pass |
| AC-6.1 | new | `loader::tests::directory_named_md_is_traversed` (slug `notes.md/inner`); code uses `is_dir()` then `is_file()` + `.md` | pass |
| AC-6.2 | changed | existing `loader::tests::subdirectory_page_without_frontmatter_is_an_error_naming_the_file`; code reviewed (`path.display()`) | pass |
| AC-7.1 | new | `help_shows_flag_descriptions` (build, top-level and clean help) | pass |
| AC-7.2 | changed | `run_command_is_not_implemented_error`; `ServerOpts` keeps `address`/`port` (reviewed) | pass |
| AC-7.3 | changed | `publish_command_is_not_implemented_error` | pass |
| AC-7.4 | changed | grep finds no `append_to_path`; `build_generates_site_from_fixture` asserts `assets/minimal/main.css` | pass |
| AC-7.5 | new | `build_fails_when_assets_path_has_no_name` (`.` and `..`, marker-only snapshot) | pass |
| AC-8.1 | new | reviewed the diff: only `chrono` 0.4.43 added, 294 packages before and after | pass |
| AC-8.2 | changed | reviewed the diff: dates, sorting, collisions, render-before-clean, safety check, `clean` missing folder, run/publish errors documented; clap `//` issue removed; home-stub and `config` issues kept | pass |
| AC-8.3 | new | done-gate commands all exit 0; only the `home::build` warning | pass |
| AC-9.1 | new | `output::tests::render_returns_paths_and_html_without_touching_fs`, `output::tests::render_error_returns_template_error` | pass |
| AC-9.2 | new | `page_render_error_keeps_previous_output` (stderr has `page.html`; page, section index, css and marker intact; snapshot equal) | pass |
| AC-9.3 | new | `section_render_error_keeps_previous_output` (also asserts the new page isn't written) | pass |
| AC-9.4 | new | read `cli::build` (`src/cli.rs` lines 103–144): load pages → templates → assets destination → items → `check_collisions` → `render` pages and sections → `ensure_safe_to_clean` → `clean_contents` → `write` → `assets::build`. Matches the spec; AC-3.2, 7.5, 9.2 and 9.3 tests pass. | pass |
