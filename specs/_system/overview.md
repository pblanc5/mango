<!-- Published by dev-pipeline run 20260914-223413-bootstrap-specs-for-this-project, stage overview v2, approved 2026-09-14T22:59:58-05:00. -->

# System overview

## Purpose
mango is a Rust command-line tool that builds static websites (`Cargo.toml:6`, `README.md:3`). It reads markdown files with JSON frontmatter from a site folder and renders them through Tera templates. It writes one HTML page per content file, plus:
- a section index for each folder level
- a home page
- a tag index (always) and one page per tag
- `feed.xml` and `sitemap.xml`, only when `base_url` is set
- a copy of the assets folder

The build loads, checks and renders everything in memory first. It only empties the output folder once nothing can fail on bad input (`src/cli.rs:120-121`, `CLAUDE.md:49`). The crate has two targets: a library (`src/lib.rs`), which holds the pipeline and exposes it as `plan` (write-free) and `commit` (the only writer), and a binary (`src/main.rs` plus the binary-only `src/cli.rs`), which parses arguments and calls it.

## Components
Line counts include each file's `#[cfg(test)]` module. "Unit tests" is the number of `#[test]` functions in that file (counted with a search).

| Path | Responsibility | Tests |
|---|---|---|
| `src/main.rs` (18 lines) | Entry point. Calls `cli::run()`, prints errors to stderr, returns exit code 1 on failure. | partial (no unit tests; `tests/build.rs` runs the binary through `run_mango` (`:18`) and checks exit status and stderr) |
| `src/cli.rs` (431) | clap CLI (`build`, `clean`, `run`, `publish`). `build()` (`:108-190`) wires the pipeline together. `ensure_safe_to_clean` and `clean_contents` guard the output folder. | yes (`src/cli.rs` 7 unit tests; `tests/build.rs` `build_refuses_to_clean_cwd`, `clean_refuses_cwd`, `failed_rebuild_keeps_previous_output`, `run_command_is_not_implemented_error`, `run_command_takes_no_options`, `publish_command_is_not_implemented_error`) |
| `src/config.rs` (271) | `SiteConfig` (`deny_unknown_fields, default`, `:19`), `load(path, explicit)` with the `base_url` scheme check (`:58-66`), `base_url_root` (`:73`) | yes (`src/config.rs` 10 unit tests; `tests/build.rs` `build_fails_on_malformed_or_unknown_field_config`, `build_fails_on_invalid_base_url_keeping_output`) |
| `src/error.rs` (94) | `MangoError` enum (thiserror): `Io`, `IoPath`, `Frontmatter`, `General`, `Config`, `Template` (Tera source chain joined, `:41-50`); `io_at` helper | yes (`src/error.rs` 4 unit tests) |
| `src/content/loader.rs` (307) | Walks the site folder recursively, picks up `.md`/`.markdown` in any case, parses each page, drops drafts (`:22`) | yes (`src/content/loader.rs` 14 unit tests; `tests/build.rs` `build_fails_when_subdirectory_page_lacks_frontmatter`, `build_accepts_any_case_md_and_markdown_extensions`) |
| `src/content/frontmatter.rs` (118) | Splits the `---` JSON `---` block from the body; ignores a leading BOM | yes (`src/content/frontmatter.rs` 6 unit tests; `tests/build.rs` `build_accepts_utf8_bom_before_frontmatter`) |
| `src/content/page.rs` (370) | `Page`, `generate_slug` (file-name check), `slug_url`, `tag_slug`, `validate_tags`, `parse_date` | yes (`src/content/page.rs` 15 unit tests; `tests/build.rs` `build_fails_on_invalid_date_naming_file_and_value`, `build_fails_on_invalid_file_name_naming_file`, `build_fails_on_invalid_tag_naming_file_and_value`) |
| `src/content/summary.rs` (24) | `PageSummary` (`title`, `date`, `slug`, `url`) built from a `Page` | partial (no tests of its own; used by the unit tests in `src/build/index/{section,tag}.rs`, `src/render/template.rs` and in E2E fixture listings) |
| `src/render/markdown.rs` (67) | pulldown-cmark to HTML, with tables, footnotes, strikethrough, task lists and heading attributes (`:4-9`) | yes (`src/render/markdown.rs` 5 unit tests; `tests/build.rs` `fixture_renders_markdown_extensions`) |
| `src/render/template.rs` (491, largest source file) | Loads Tera templates with a glob (`:167-174`). Defines the template contexts (`PageTemplate`, `SectionTemplate`, `HomeTemplate`, `TagTemplate`, link and entry structs), `RenderItem`, and the hardcoded template names (`:186`, `:216`, `:235`, `:252`, `:274`). | yes (`src/render/template.rs` 10 unit tests; `tests/build.rs` `missing_home_template_keeps_previous_output`, `missing_tags_template_keeps_previous_output`, `page_render_error_keeps_previous_output`) |
| `src/build/output.rs` (423) | `check_collisions` (`:32-88`, exact paths and file-vs-folder, across render items, generated files and assets); `render`, `render_generated`, `write`; slug to output path (`get_final_path`, `:132`) | yes (`src/build/output.rs` 11 unit tests; `tests/build.rs` `build_fails_on_page_and_section_output_collision`, `build_fails_on_file_vs_folder_conflict_keeping_output`) |
| `src/build/index/section.rs` (261) | `build_section_index`: a section for every ancestor folder, stored in a `BTreeMap`; `compare_summaries` (`:60-71`, newest first, undated last, then title, then slug) | yes (`src/build/index/section.rs` 12 unit tests; `tests/build.rs` `build_generates_ancestor_section_indexes`, `fixture_orders_listings_and_caps_recent`) |
| `src/build/index/tag.rs` (104) | `build_tag_index`: tag name to summaries sorted by `compare_summaries` | yes (`src/build/index/tag.rs` 4 unit tests; `tests/build.rs` `fixture_tag_index_counts_and_dedup`) |
| `src/build/generate/content.rs` (13) | Maps pages to page render items | none directly (no test references `content::build`); only exercised by E2E fixture builds |
| `src/build/generate/section.rs` (67) | Section render items in slug order | yes (`src/build/generate/section.rs` 1 unit test) |
| `src/build/generate/home.rs` (171) | Home render item; `recent_pages` (`:10`), which the feed also uses | yes (`src/build/generate/home.rs` 4 unit tests; `tests/build.rs` `home_recent_respects_recent_count_and_skips_undated`) |
| `src/build/generate/tag.rs` (92) | Tag index item first, then one item per tag in name order | yes (`src/build/generate/tag.rs` 2 unit tests; `tests/build.rs` `build_writes_tag_index_without_tags`) |
| `src/build/generate/feed.rs` (263) | RSS 2.0 `feed.xml`, built as a Rust string; `None` without `base_url` | yes (`src/build/generate/feed.rs` 7 unit tests; `tests/build.rs` `fixture_feed_and_sitemap`, `feed_and_sitemap_only_with_base_url`) |
| `src/build/generate/sitemap.rs` (235) | sitemaps.org 0.9 `sitemap.xml` built from the render items, sorted by `loc` | yes (`src/build/generate/sitemap.rs` 6 unit tests; same E2E tests as the feed) |
| `src/build/generate/xml.rs` (46) | XML escaping of `& < > " '` | yes (`src/build/generate/xml.rs` 2 unit tests; `tests/build.rs` `fixture_escapes_special_characters`) |
| `src/build/generate/assets.rs` (166) | `plan` (`:21`) lists asset files, sorted by destination, before cleaning; `copy` (`:71`) copies them after writing | yes (`src/build/generate/assets.rs` 4 unit tests; `tests/build.rs` `build_fails_on_missing_assets_folder_keeping_output`, `build_fails_when_page_lands_on_an_asset_keeping_output`) |
| `src/*/mod.rs` (5 files, 2-8 lines each) | Module declarations only | n/a |
| `tests/build.rs` (1838) | E2E tests (57 `#[test]`): runs the compiled binary against the fixture (`fixture_dist()`, `:142`) and against temp sites | n/a (test code) |
| `example/site`, `example/meta`, `example/mango.json` | Committed fixture site covering every success-path feature (`CLAUDE.md:69`); the full output list is checked by `build_generates_site_from_fixture` (`tests/build.rs:475`) | n/a (fixture) |

## Entry points
- **Binary `mango`** (`src/main.rs`) calls `cli::run()` (`src/cli.rs:86`).
- **`mango build`** (`src/cli.rs:108`). Flags and their defaults (`src/cli.rs:20-41`); every path is relative to the current directory:
  - `--site` (`site`)
  - `--templates` (`meta/templates`)
  - `--assets` (`meta/assets`)
  - `-o/--output` (`dist`)
  - `--config` (`mango.json`, optional; `config::DEFAULT_CONFIG_PATH`, `src/config.rs:8`)
- **`mango clean`** (`src/cli.rs:192-207`). `-d/--dist` (`dist`). A missing folder counts as success. It protects the cwd, `site`, `meta/templates` and `meta/assets`, but not a config file (`README.md:105`).
- **`mango run`** and **`mango publish`** are not implemented and take no options. Both return `MangoError::General` and exit 1 (`tests/build.rs` `run_command_is_not_implemented_error`, `run_command_takes_no_options`, `publish_command_is_not_implemented_error`).
- **Build order** in `build()` (`src/cli.rs:122-187`):
  1. load pages
  2. load config
  3. load templates
  4. work out the assets destination, then plan the asset files
  5. create page, section index, home and tag items
  6. build feed and sitemap in memory
  7. `check_collisions`
  8. render everything to strings
  9. `ensure_safe_to_clean`, then `clean_contents`
  10. write the files
  11. copy assets

## External dependencies
From `Cargo.toml:13-20`. There are no `[dev-dependencies]` and no build script. `tide` is no longer a dependency (commit `dc66348`).
- `clap` 4.5 (derive): CLI parsing
- `serde` 1.0 (derive) and `serde_json` 1.0: JSON frontmatter, config, and serializing template contexts
- `tera` 1.20: HTML templates
- `pulldown-cmark` 0.13: markdown rendering
- `chrono` 0.4 (`std` only, no default features): `NaiveDate` parsing and formatting
- `thiserror` 2.0: `MangoError`
- Toolchain: Rust 1.92.0 with rustfmt and clippy (`rust-toolchain.toml`; `rust-version = "1.92"` at `Cargo.toml:5`), edition 2024 (`Cargo.toml:4`). `[lints.clippy] all = "warn"` (`Cargo.toml:10-11`).
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
    - `<slug>/index.html` per item
    - `index.html` for the home page
    - `tags/index.html` and `tags/<tag>/index.html`
    - `feed.xml` and `sitemap.xml`
    - `<assets folder name>/...` (`src/cli.rs:126-127`)
- **In memory:** every rendered file is held in memory until the write step (`output::RenderedFile`, `GeneratedFile`, `src/build/output.rs:11-24`), so memory use grows with site size.
- **Test artifacts** (all gitignored under `/target` or `/example/dist`, `.gitignore:1-2`):
  - `target/unit-fixtures/<module>/<test>` (e.g. `src/config.rs:87`)
  - `target/integration-dist`
  - per-test temp sites under `CARGO_TARGET_TMPDIR`
  - `example/dist`
- No caches, databases or incremental build state: every build is a full rebuild.

## Risky areas
| Area | Why |
|---|---|
| `src/cli.rs` `build()`, `ensure_safe_to_clean`, `clean_contents` | **Deletes user files.** Safety depends on two things: the step order (everything that can fail runs before `clean_contents`, `:179`) and comparing canonicalized paths. If a protected path fails to canonicalize for any reason, not only NotFound, it is skipped (`:234-236`). An I/O error after the clean leaves partial output (documented, `CLAUDE.md:49`). Well tested, but any new output type must be wired in before `:173`. |
| Determinism (whole build) | **Guarded by one test.** Byte-identical rebuilds are required (`CLAUDE.md:79`, constitution) and checked by `fixture_build_is_deterministic` in `tests/build.rs`, which builds the fixture twice and compares every file. Order depends on `BTreeMap`s and explicit sorts (`src/build/index/section.rs:21`, `src/build/generate/assets.rs:32`, sitemap sorted by `loc`), so any new listing or output must be sorted too. The fixture footer prints the current year, so two builds that straddle New Year differ. |
| `src/content/loader.rs` `traverse` (`:25-67`) | **Inferred, confirm.** `path.is_dir()` (`:30`) follows symlinks and there is no cycle guard, so a symlink loop inside `site/` could recurse without limit. No loader test mentions symlinks; the only symlink tests are for `clean_contents` in `src/cli.rs`. It also uses `fs::read_dir` order without sorting. Listings are sorted later, but the order of page items, and so which source is named first in a collision message, can depend on the filesystem. |
| `src/build/generate/assets.rs` `collect` and `copy` (`:36-80`) | **Inferred, confirm.** `collect` uses `DirEntry::file_type()` (`:45-48`), which does not follow symlinks. A symlink to a folder inside the assets folder would be planned as a file, and `fs::copy` (`:76`) would then fail after the output was cleaned. No asset symlink test was found. |
| Raw HTML in content (`src/render/markdown.rs:11-13`, `example/meta/templates/page.html:26`) | **Inferred, confirm.** pulldown-cmark passes raw HTML through, and the fixture template prints `page.content \| safe`. Content is effectively trusted. `README.md` "Known limitations" (`:101-105`) does not say so. |
| `src/content/frontmatter.rs` `MangoFrontmatter` (`:8-16`) | **Unknown keys and CRLF untested.** Frontmatter has no `deny_unknown_fields`, unlike `SiteConfig` (`src/config.rs:19`). A misspelled optional key such as `"tag"` or `"dates"` is silently ignored, and the docs don't promise either behavior. CRLF line endings are normalized by `lines()` and `join("\n")` (`:25`, `:35-36`), but no test uses `\r\n`. |
| `src/render/template.rs` (491 lines) | **Largest source file; a contract with users.** It holds every template context and `RenderItem`. Context field names are a public contract with site templates (`CLAUDE.md:17-25`), so renaming a field breaks users' templates, and it only shows up at render time. |

## Not read
- **Unit test bodies:** only the tests in `src/cli.rs`, `src/render/markdown.rs` and the start of `src/config.rs` and `src/build/generate/section.rs` were read. The rest were counted by `#[test]` and sampled by name or helper.
- **`tests/build.rs`:** only the function names were read (confirmed by search), not the bodies.
- **Source beyond the first part of each file:** `src/content/page.rs` and `src/render/template.rs` were read in attempt 1 (non-test parts only) and checked this time by search. `src/build/generate/{feed,sitemap}.rs` non-test parts come from attempt 1.
- **Fixture contents:** the markdown under `example/site/` and all templates except `page.html` (searched for `safe` only).
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
