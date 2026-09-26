<!-- Published by dev-pipeline run 20260914-223413-bootstrap-specs-for-this-project, stage overview v2, approved 2026-09-14T22:59:58-05:00. -->

# System overview

## Purpose
mango is a Rust command-line tool that builds static websites (`Cargo.toml` `[package]` `description`; `README.md`, opening paragraph). It reads markdown files with JSON frontmatter from a site folder and renders them through Tera templates. It writes one HTML page per content file, plus:
- a section index for each folder level
- a home page
- a tag index (always) and one page per tag
- `feed.xml` and `sitemap.xml`, only when `base_url` is set
- a copy of the assets folder

The build loads, checks and renders everything in memory first. It only empties the output folder once nothing can fail on bad input. That guarantee comes from the plan/commit seam in `src/build/pipeline.rs`, not from statement order: `plan` does everything that can fail on bad input and writes nothing, and `commit` accepts only a `BuildPlan`, whose fields are private and whose sole constructor is `plan` (CLAUDE.md, "CLI surface (`src/cli.rs`) and the plan/commit seam"). The crate has two targets: a library (`src/lib.rs`), which holds the pipeline and exposes it as `plan` (write-free) and `commit` (the only writer), and a binary (`src/main.rs` plus the binary-only `src/cli.rs`), which parses arguments and calls it.

## Components
| Path | Responsibility | Tests |
|---|---|---|
| `src/main.rs` | Entry point. Declares `mod cli`, calls `cli::run`, prints errors to stderr and returns exit code 1 on failure. | partial (no unit tests; `tests/build.rs` runs the binary through its `run_mango` helper and checks exit status and stderr) |
| `src/lib.rs` | Library root. Every module is private; the public surface is exactly seven re-exports: `plan`, `commit`, `clean`, `BuildOptions`, `BuildPlan`, `PlannedOutput`, `MangoError`. An internal seam with no semver promise. | yes (`tests/plan.rs` uses nothing but these seven names, e.g. `clean_is_reachable_through_the_library`) |
| `src/cli.rs` | Binary only. clap derive structs (`MangoCli`, `MangoActions`, `BuildOpts`, `CleanOpts`), flag defaults and help text, `BuildOpts::into_options`, and `run`, which dispatches `build`/`clean`/`run` and nothing else. | yes (no unit tests; `tests/build.rs` `help_shows_flag_descriptions`, `run_command_is_not_implemented_error`, `run_command_takes_no_options`, `publish_is_not_a_command`) |
| `src/build/pipeline.rs` | `BuildOptions`, `BuildPlan` (private fields; `output_dir`, `outputs`), `PlannedOutput`, `plan` (load, check and render, writing nothing) and `commit` (the only function that empties or writes the output folder). | yes (no unit tests; `tests/plan.rs` `planning_creates_modifies_and_deletes_nothing`, `plan_enumerates_every_kind_in_order`, `commit_writes_exactly_the_enumerated_outputs`, `first_failure_is_reported_in_pipeline_order`; `tests/build.rs` `failed_rebuild_keeps_previous_output`, `build_reports_first_failure_in_pipeline_order`) |
| `src/build/clean.rs` | `clean` (`mango clean`), plus the crate-private `current_dir`, `ensure_safe_to_clean` (refuses the cwd, its parents, and any protected input or its parents, compared canonicalized) and `clean_contents` (empties the folder, removes symlinks without following them). | yes (`src/build/clean.rs` unit tests, e.g. `refuses_protected_path_or_its_parent`, `clean_contents_removes_dir_symlink`; `tests/build.rs` `build_refuses_to_clean_cwd`, `clean_refuses_cwd`, `build_refuses_output_containing_config`; `tests/plan.rs` `commit_refuses_output_containing_an_input_and_touches_nothing`) |
| `src/hidden.rs` | `is_hidden(&OsStr)` (crate-private): the single hidden-name rule, a name whose first byte is `.` (RISK-10). Called by `loader::traverse` and `assets::collect` on each `DirEntry::file_name()` before any resolution, so the `--site` and `--assets` folders themselves are never skipped. | yes (`src/hidden.rs` unit tests, e.g. `names_starting_with_a_dot_are_hidden`, `other_names_are_not_hidden`; `tests/build.rs` `build_skips_hidden_entries`) |
| `src/config.rs` | `SiteConfig` (`deny_unknown_fields, default`), `DEFAULT_CONFIG_PATH`, `load(path, explicit)` with the `base_url` scheme check, `base_url_root` | yes (`src/config.rs` unit tests, e.g. `base_url_rejects_other_values_naming_file_and_value`; `tests/build.rs` `build_fails_on_malformed_or_unknown_field_config`, `build_fails_on_invalid_base_url_keeping_output`) |
| `src/error.rs` | `MangoError` enum (thiserror): `Io`, `IoPath`, `Frontmatter`, `General`, `Config`, `Template` (Tera source chain joined by `tera_chain`); `io_at` helper | yes (`src/error.rs` unit tests, e.g. `template_display_includes_source_chain`) |
| `src/content/loader.rs` | `load` walks the site folder recursively (`traverse`), picks up `.md`/`.markdown` in any case (`is_markdown`), parses each page, skips hidden entries (a name starting with `.`) without resolving them, rejects a symlinked content folder, fails on an unresolvable entry (a dangling link, a looping chain) and drops drafts | yes (`src/content/loader.rs` unit tests, e.g. `load_rejects_symlink_loop_to_ancestor`, `load_fails_on_dangling_symlink`, `load_skips_hidden_files_and_folders`; `tests/build.rs` `build_fails_when_subdirectory_page_lacks_frontmatter`, `build_accepts_any_case_md_and_markdown_extensions`, `build_fails_on_symlinked_content_folder_keeping_output`) |
| `src/content/frontmatter.rs` | `parse` splits the `---` JSON `---` block from the body into a `MangoFrontmatter`; ignores a leading BOM; normalizes CRLF; `check_keys` rejects unknown keys, listing every unknown and missing key in one error (RISK-4) | yes (`src/content/frontmatter.rs` unit tests, e.g. `every_unknown_and_missing_key_is_listed_in_a_fixed_order`, `crlf_and_lf_documents_parse_identically`; `tests/build.rs` `build_accepts_utf8_bom_before_frontmatter`, `build_fails_on_unknown_frontmatter_keys_keeping_output`, `build_accepts_crlf_line_endings`) |
| `src/content/page.rs` | `Page`, `PageType`, `Page::new` (validates the date, then the tags, then the slug, so a frontmatter error wins over a file-name error), `parse_date`, `serialize_date` | yes (`src/content/page.rs` unit tests, e.g. `parse_date_rejects_impossible_dates`, `new_rejects_invalid_tag`; `tests/build.rs` `build_fails_on_invalid_date_naming_file_and_value`) |
| `src/content/slug.rs` | `Slug` (crate-private): file-name validation in `Slug::from_content_path`, and every slug rule: `home`, `tag_index`, `tag_page`, `parent`, `is_top_level`, `segments`, `url`, `output_path`. Orders byte-wise on the `/`-joined text. | yes (`src/content/slug.rs` unit tests, e.g. `dot_only_segments_get_their_own_message`, `orders_bytewise_on_joined_text`, `backslash_in_name_is_an_invalid_file_name`; `tests/build.rs` `build_fails_on_invalid_file_name_naming_file`) |
| `src/content/tag.rs` | `Tag` (crate-private): `Tag::parse` (tag rule checked by hand), `Tag::parse_list` (parse and de-duplicate), `Tag::slug`, `Tag::url` | yes (`src/content/tag.rs` unit tests, e.g. `orders_bytewise_and_serializes_as_plain_string`; `tests/build.rs` `build_fails_on_invalid_tag_naming_file_and_value`, `build_fails_on_invalid_tag_in_draft`) |
| `src/content/summary.rs` | `PageSummary` (`title`, `date`, `slug`, `url`) built from a `Page` | partial (no tests of its own; covered by `src/build/index/section.rs` `summary_serializes_url`, the index and template unit tests, and the E2E fixture listings) |
| `src/render/markdown.rs` | `to_html`: pulldown-cmark to HTML, with tables, footnotes, strikethrough, task lists and heading attributes | yes (`src/render/markdown.rs` unit tests, e.g. `renders_heading_attributes`; `tests/build.rs` `fixture_renders_markdown_extensions`) |
| `src/render/template.rs` | `load_templates` (Tera glob load). The template contexts (`PageTemplate`, `SectionTemplate`, `HomeTemplate`, `TagTemplate`, plus `TagLink`, `TagIndexEntry`, `SectionLink`, `HomeSection`) and the constructors of template `Output`s, each with its `OutputKind` and a `Body::Template` holding the hardcoded template name and a context that also gets `config`: `render_page`, `render_section_page`, `render_home_page`, `render_tag_index`, `render_tag_page` | yes (`src/render/template.rs` unit tests, e.g. `page_context_tags_are_links`, `null_config_title_uses_default_filter`; `tests/build.rs` `missing_home_template_keeps_previous_output`, `missing_tags_template_keeps_previous_output`, `page_render_error_keeps_previous_output`) |
| `src/build/output.rs` | The single output model: `Output`, `OutputKind`, `Body`; `Output::path` derives each location from the kind, and `Display` for `OutputKind` gives the collision label. `check_collisions` (exact paths and file-vs-folder, over one list); `render` to in-memory `RenderedOutput`/`Contents`; `write` (writes text, copies assets) | yes (`src/build/output.rs` unit tests, e.g. `file_vs_directory_conflict_is_detected`, `labels_derive_from_kind`, `write_failure_names_the_path`; `tests/build.rs` `build_fails_on_page_and_section_output_collision`, `build_fails_on_file_vs_folder_conflict_keeping_output`; `tests/plan.rs` `collision_labels_are_exact_for_reachable_kinds`) |
| `src/build/index/section.rs` | `build_section_index`: a `Section` for every ancestor folder, stored in the `BTreeMap` of a `SectionIndex`; `compare_summaries` (newest first, undated last, then title, then slug) | yes (`src/build/index/section.rs` unit tests, e.g. `ancestors_get_section_entries`, `ties_broken_by_title_then_slug`; `tests/build.rs` `build_generates_ancestor_section_indexes`, `fixture_orders_listings_and_caps_recent`; `tests/plan.rs` `section_lists_newest_first_undated_last_then_title_then_slug`) |
| `src/build/index/tag.rs` | `build_tag_index`: a `BTreeMap` from tag to summaries sorted by `compare_summaries` | yes (`src/build/index/tag.rs` unit tests, e.g. `groups_pages_by_tag_sorted`; `tests/build.rs` `fixture_tag_index_counts_and_dedup`) |
| `src/build/generate/content.rs` | `build`: maps pages to page outputs through `render_page` | none directly; exercised by `tests/plan.rs` `plan_enumerates_every_kind_in_order` and the E2E fixture builds |
| `src/build/generate/section.rs` | `build`: section index outputs in slug order | yes (`src/build/generate/section.rs` unit test `render_items_in_section_slug_order`) |
| `src/build/generate/home.rs` | `build`: the home output from pages and the section index; `recent_pages`, which the feed also uses | yes (`src/build/generate/home.rs` unit tests, e.g. `recent_pages_matches_home_recent`; `tests/plan.rs` `home_recent_respects_recent_count_and_skips_undated`) |
| `src/build/generate/tag.rs` | `build`: the tag index output first, then one output per tag in name order | yes (`src/build/generate/tag.rs` unit tests `index_first_then_tags_in_name_order`, `index_generated_without_tags`; `tests/build.rs` `build_writes_tag_index_without_tags`) |
| `src/build/generate/feed.rs` | `build`: RSS 2.0 `feed.xml`, built as a Rust string; `None` without `base_url` | yes (`src/build/generate/feed.rs` unit tests, e.g. `golden_feed`; `tests/build.rs` `fixture_feed_and_sitemap`, `feed_and_sitemap_only_with_base_url`) |
| `src/build/generate/sitemap.rs` | `build`: sitemaps.org 0.9 `sitemap.xml`, one entry per HTML output picked out of the output list by kind (`entry`, an exhaustive `match` on `OutputKind`), sorted by `loc`; `None` without `base_url` | yes (`src/build/generate/sitemap.rs` unit tests, e.g. `lists_only_html_kinds_and_dates_only_content_pages`, `sorted_by_loc_as_plain_strings`; same E2E tests as the feed) |
| `src/build/generate/xml.rs` | `escape`: XML escaping of `& < > " '` | yes (`src/build/generate/xml.rs` unit tests, e.g. `escapes_special_characters`; `tests/build.rs` `fixture_escapes_special_characters`) |
| `src/build/generate/assets.rs` | `plan` only: lists every asset file (through the private `collect`, which skips hidden entries without resolving them) as an `Asset` output with a `Copy` body, sorted by relative path, before cleaning. The copy itself is done by `output::write` in `commit`. | yes (`src/build/generate/assets.rs` unit tests, e.g. `plan_rejects_symlinked_directory`, `plan_fails_on_symlink_loop`; `tests/build.rs` `build_fails_on_missing_assets_folder_keeping_output`, `build_fails_on_symlinked_asset_folder_keeping_output`, `build_fails_when_page_lands_on_an_asset_keeping_output`) |
| `src/*/mod.rs` (5 files) | Module declarations only | n/a |
| `tests/build.rs` | E2E tests: runs the compiled binary (`run_mango`) against the fixture, built once by `fixture_dist()` into `target/integration-dist`, and against per-test temp sites under `CARGO_TARGET_TMPDIR` | n/a (test code) |
| `tests/plan.rs` | In-process tests of `plan`/`commit`/`clean` through the library's seven re-exports, with a minimal five-template theme so listing order can be asserted as exact strings; temp sites under `CARGO_TARGET_TMPDIR`; never changes the cwd | n/a (test code) |
| `example/site`, `example/meta`, `example/mango.json` | Committed fixture site covering every success-path feature (CLAUDE.md, "Conventions"); the full output list is checked by `build_generates_site_from_fixture` in `tests/build.rs` | n/a (fixture) |

## Entry points
- **Binary `mango`** (`src/main.rs`) calls `cli::run` (`src/cli.rs`).
- **`mango build`** runs `mango::commit(mango::plan(..))`, with the options from `BuildOpts::into_options`. Flags and their defaults (`BuildOpts` in `src/cli.rs`); every path is relative to the current directory:
  - `--site` (`site`)
  - `--templates` (`meta/templates`)
  - `--assets` (`meta/assets`)
  - `-o/--output` (`dist`)
  - `--config` (`mango.json`, optional; `config::DEFAULT_CONFIG_PATH`)
- **`mango clean`** calls `mango::clean`. `-d/--dist` (`dist`, `CleanOpts` in `src/cli.rs`). A missing folder counts as success. It protects the cwd, `site`, `meta/templates` and `meta/assets`, but not a config file (`README.md`, "Known limitations").
- **`mango run`** is not implemented and takes no options. It returns `MangoError::General` and exits 1 (`tests/build.rs` `run_command_is_not_implemented_error`, `run_command_takes_no_options`).
- **`publish`** is not a command. clap rejects `mango publish` as an "unrecognized subcommand" (`tests/build.rs` `publish_is_not_a_command`; backlog FEAT-2, dropped).
- **Build order**, in `plan` and then `commit` (`src/build/pipeline.rs`):
  1. `plan`: load pages (`loader::load`), then config (`config::load`), then templates (`template::load_templates`)
  2. resolve the assets destination (an assets path with no final name is an error)
  3. `assets::plan` lists the asset files
  4. build one output list, in this order: pages (markdown is converted to HTML here), section indexes, home page, tag index and tag pages, `feed.xml`, `sitemap.xml` (both only with `base_url`), then the asset copies
  5. `output::check_collisions` over that list
  6. `output::render` renders every template to a string, in memory
  7. `commit`: `ensure_safe_to_clean`, then `clean_contents`, then `output::write` (every file, then every asset copy, since assets come last)

## External dependencies
From `Cargo.toml` `[dependencies]`. There are no `[dev-dependencies]` and no build script. `tide` is no longer a dependency.
- `clap` 4.5 (derive): CLI parsing
- `serde` 1.0 (derive) and `serde_json` 1.0: JSON frontmatter, config, and serializing template contexts
- `tera` 1.20: HTML templates
- `pulldown-cmark` 0.13: markdown rendering
- `chrono` 0.4 (`std` only, no default features): `NaiveDate` parsing and formatting
- `thiserror` 2.0: `MangoError`
- Toolchain: `rust-toolchain.toml` pins channel 1.92.0 with the components `rustfmt`, `clippy` and `rust-analyzer`. `Cargo.toml` `[package]` sets `rust-version = "1.92"` and edition 2024, and `[lints.clippy]` sets `all = "warn"`.
- Runtime: local filesystem only. No network, database or environment configuration was found.

## Data and state
- **Inputs** (read only):
  - site folder of markdown files with JSON frontmatter
  - templates folder: everything matching the glob is loaded; `page.html`, `section.html`, `home.html`, `tags.html` and `tag.html` are required
  - assets folder
  - optional `mango.json`
- **Output:** the output folder (default `dist/`).
  - `build` empties it but keeps the folder itself.
  - `clean` removes it entirely.
  - Layout:
    - `<slug>/index.html` per page and section index
    - `index.html` for the home page
    - `tags/index.html` and `tags/<tag>/index.html`
    - `feed.xml` and `sitemap.xml`
    - `<assets folder name>/...` (the folder name is taken from the `--assets` path in `plan`)
- **In memory:** a `BuildPlan` holds every output as a `RenderedOutput` until `commit` writes it: text files (pages, indexes, feed, sitemap) are held in memory as strings, and asset copies are held as source paths and copied at write time. Memory use grows with the size of the text output.
- **Test artifacts** (all gitignored by the `/target` and `/example/dist` entries in `.gitignore`):
  - `target/unit-fixtures/<module>/<test>` (e.g. the `fixture_dir` helpers in the `src/config.rs` and `src/build/clean.rs` tests)
  - `target/integration-dist`
  - per-test temp sites under `CARGO_TARGET_TMPDIR` (`tests/build.rs` and `tests/plan.rs`)
  - `example/dist`
- No caches, databases or incremental build state: every build is a full rebuild.

## Risky areas
| Area | Why |
|---|---|
| `pipeline::commit`, `clean::ensure_safe_to_clean`, `clean::clean_contents` | **Deletes user files.** Nothing is cleaned before everything that can fail on bad input has run, because `commit` accepts only a `BuildPlan` and `plan` is its only constructor. `ensure_safe_to_clean` compares canonicalized paths. If a protected path fails to canonicalize for any reason, not only NotFound, it is skipped (the `let Ok(canonical) = … else { continue }` in `ensure_safe_to_clean`). An I/O failure after the clean can still leave partial output (CLAUDE.md, "CLI surface (`src/cli.rs`) and the plan/commit seam"). A new kind of output is safe only if it is added as an `OutputKind` to `plan`'s single output list, so that the collision check and rendering see it before `commit`. Well tested (`commit_refuses_output_containing_an_input_and_touches_nothing`, `failed_rebuild_keeps_previous_output`). |
| Determinism (whole build) | **Guarded by tests, not by construction.** Byte-identical rebuilds are required (CLAUDE.md, "Definition of done"; constitution) and checked by `fixture_build_is_deterministic` in `tests/build.rs`, which builds the fixture twice and compares every file, and by `planning_twice_gives_identical_outputs` in `tests/plan.rs`. Order depends on `BTreeMap`s and explicit sorts: the `BTreeMap` of sections in `index/section.rs` (`SectionIndex`), the `BTreeMap` in `build_tag_index`, the sort in `assets::plan`, and the sort by `loc` in `sitemap::build`. Any new listing or output must be sorted too. Generated files carry no timestamps. |
| `src/content/loader.rs` `traverse` | A symlinked content folder is now a `General` error (`content folder '<rel>' is a symlink to a directory`), so a link to an ancestor can no longer re-walk the site (RISK-1, done; `load_rejects_symlink_loop_to_ancestor`). An entry whose target cannot be resolved (a dangling link, a looping chain) now fails the load with the same I/O error as in `assets::plan`, whatever its name (RISK-5, done; `load_fails_on_dangling_symlink`, `load_fails_on_symlink_loop`), unless it is hidden: an entry whose name starts with `.` is skipped first, with everything inside a hidden folder, and is never resolved, read or checked (RISK-10, done; `load_skips_hidden_files_and_folders`, `hidden_symlinks_are_not_resolved`). A page with a hidden name is therefore silently not published. One thing remains open: `loader::load` returns pages in unsorted `read_dir` order, which differs between filesystems (RISK-8, open). Content pages come first in the plan, so this order affects the order of `BuildPlan::outputs()`, the order files are written in, and which page's render error is reported when several pages fail. The output bytes do not depend on it: every listing is sorted, and a page-vs-page collision prints the same labels either way. |
| `src/build/generate/assets.rs` `plan` and `collect` | `collect` classifies entries with `fs::metadata`, which follows symlinks, so a symlinked folder, a dangling link or a link loop fails in `assets::plan`, before cleaning (RISK-2, done; `plan_rejects_symlinked_directory`, `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`, `build_fails_on_symlinked_asset_folder_keeping_output`). Hidden entries are skipped first, before `file_type()` or `metadata`, so a hidden broken link cannot fail the plan and a hidden file is never copied (RISK-10, done; `plan_skips_hidden_files_and_folders`, `plan_does_not_resolve_hidden_symlinks`). Still open: when an asset copy fails in `output::write`, the error names the source, not the destination (RISK-7, open; pinned by `copy_failure_names_the_source_path`). |
| Raw HTML in content (`to_html` in `src/render/markdown.rs`, `page.content \| safe` in `example/meta/templates/page.html`) | pulldown-cmark passes raw HTML through, and the fixture template prints `page.content \| safe`. Content is effectively trusted. `README.md` "Known limitations" does not mention raw HTML (RISK-3, open). |
| `src/content/frontmatter.rs` `parse`, `MangoFrontmatter` | Unknown keys are rejected since RISK-4 (`check_keys`). CRLF line endings are normalized (`str::lines` drops the `\r`, and the JSON block and body are re-joined with `\n`), and that handling is pinned by the TEST-1 tests, among them `crlf_and_lf_documents_parse_identically` in `src/content/frontmatter.rs` and `build_accepts_crlf_line_endings` in `tests/build.rs`. The one asymmetry, the verbatim "no frontmatter" return, is unreachable from the CLI and pinned by `crlf_body_is_returned_verbatim_when_no_frontmatter`. Still open: serde's derived deserializer for `MangoFrontmatter` also accepts a JSON array in field order as frontmatter (RISK-9, open). |
| `src/render/template.rs` | **A contract with users.** It holds every template context. Context field names are a public contract with site templates (CLAUDE.md, "Template contexts"), so renaming a field breaks users' templates, and that only shows up at render time. ARCH-5 (open) plans to move the `render_*` constructors into `build/generate/*`. |

## Not read
This section describes the 2026-09-25 refresh (DOC-1).
- **Read in full (non-test code):** `src/lib.rs`, `src/main.rs`, `src/cli.rs`, `src/build/pipeline.rs`, `src/build/clean.rs`, `src/content/loader.rs`, `src/build/generate/assets.rs`, `src/build/generate/content.rs`, `src/build/generate/sitemap.rs` and `src/render/markdown.rs`; `Cargo.toml`, `rust-toolchain.toml`, `.gitignore`; the README headings and "Known limitations"; `specs/_system/backlog.md`.
- **Read in part:** the other source files were checked by searching for their item declarations (`fn`, `struct`, `enum`, `const`), plus the parts of `src/build/output.rs`, `src/render/template.rs`, `src/config.rs` and `src/error.rs` that the rows above describe. Their bodies were otherwise not re-read.
- **Tests:** only test names were read (by search, per file), not bodies; the number of tests per file is not recorded.
- **Fixture contents:** `example/meta/templates/page.html` (searched for `safe`) and the footer of `example/meta/templates/base.html`. The markdown under `example/site/` and the other templates were not read.
- **Not read:** `CHANGELOG.md`, `specs/constitution.md` and the other specs, `.github/workflows/ci.yml`.
- **Skipped as generated or not relevant to architecture:**
  - `target/` and `example/dist/` (generated)
  - `Cargo.lock`
  - `LICENSE`
  - `.claude/`
  - `tasks.md` (gitignored, not project policy per the constitution)

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-14 | 20260914-223413-bootstrap-specs-for-this-project | Initial system overview drafted from docs, manifests and source |
| 2026-09-14 | 20260914-223413-bootstrap-specs-for-this-project | Matched constitution v2. Added a `ServerOpts` dead-code risk. The determinism row now refers to the required test instead of an open conflict. Recorded that `tide` is gone. Read `config.rs`, `error.rs`, `output.rs`, `index/{section,tag}.rs`, `generate/{home,section,tag,xml}.rs` and corrected line references (template glob `:167-174`, CLI flags `:20-41`). Added the `run`/`publish` E2E tests and `clean`'s config-file gap to Entry points. |
| 2026-09-14 | follow-up to 20260914-223413-bootstrap-specs-for-this-project | `ServerOpts` removed (`run` takes no options), so its risky-area row is gone; the determinism row now names `fixture_build_is_deterministic`. |
| 2026-09-25 | 20260925-221145-doc-1-overview-line-references | Refreshed for DOC-1 after ARCH-1/2/3, RISK-1/2/4, TEST-1 and the FEAT-2 drop. Every line-number citation replaced by a symbol, test name or section name; per-file line and test counts removed. Components gained rows for `lib.rs`, `build/pipeline.rs`, `build/clean.rs`, `content/slug.rs`, `content/tag.rs` and `tests/plan.rs`, and dropped references to symbols that no longer exist. Entry points and the build order now describe `plan`/`commit`; `publish` is recorded as not a command. Risk rows updated: deleting user files (`commit`), loader (RISK-1 done; RISK-5, RISK-8 open), assets (RISK-2 done; RISK-7 open), frontmatter (TEST-1 done; RISK-9 open), raw HTML (RISK-3 open); the false footer claim removed from the determinism row. |
| 2026-09-25 | 20260926-004309-risk-5-broken-symlinks-in-site | RISK-5 done: the loader row and the `traverse` risk row now say an unresolvable entry under the site folder fails the load, as in `assets::plan`, and cite `load_fails_on_dangling_symlink` and `load_fails_on_symlink_loop` instead of the removed skip tests. |
| 2026-09-26 | 20260926-163650-risk-10-skip-hidden-entries | RISK-10 done: added a Components row for `src/hidden.rs`; the loader and assets rows and their risk rows now say hidden entries are skipped before resolution and cite `load_skips_hidden_files_and_folders`, `hidden_symlinks_are_not_resolved`, `plan_skips_hidden_files_and_folders` and `plan_does_not_resolve_hidden_symlinks`; the slug row no longer cites the removed `build_fails_on_dot_only_file_name_keeping_output`. |
