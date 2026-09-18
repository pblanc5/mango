# Batch 3 — make it a usable site

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260914-024944-tasks-md/01-plan.v1.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260914-024944-tasks-md`
- Groups defined here: `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-5`, `AC-6`, `AC-7`
- Criteria: 45

## Requirements

### REQ-1 Site config file
An optional JSON config file gives templates site-wide metadata. It loads before anything else can touch the output folder.
- AC-1.1 `mango build --help` lists a `--config` option described as "Path to the site config file", and the help mentions the default `mango.json`.
- AC-1.2 With no `--config` flag and no `mango.json` in the cwd, the build succeeds using defaults. The rendered `<title>` contains `My Site`.
- AC-1.3 With no `--config` flag and a valid `mango.json` in the cwd, the build uses its values. The `<title>` contains the configured title and the `<footer>` contains the configured author.
- AC-1.4 With `--config <path>` pointing to a file that doesn't exist, the build fails: non-zero exit, stderr names `<path>`, nothing on stdout, and the output folder is unchanged.
- AC-1.5 A config file with malformed JSON fails the build with `MangoError::Config`. stderr names the config path and includes the serde parse message. The output folder is unchanged.
- AC-1.6 A config file with an unknown field (for example `"titel"`) fails the build with `MangoError::Config`, naming the config path and the unknown field. The output folder is unchanged.
- AC-1.7 A negative or non-integer `recent_count` fails the build with `MangoError::Config`, naming the config path.
- AC-1.8 The config loads before templates: with a malformed config **and** a missing templates folder, the error is the config error.
- AC-1.9 `SiteConfig` defaults are: `title`, `author`, `description` and `base_url` unset, and `recent_count` 10. When given only some fields, the unset ones keep their defaults.
- AC-1.10 The `config` context object always has the keys `title`, `author`, `description`, `base_url` and `recent_count`. Unset string fields serialize as JSON `null`, so they render as empty text, are falsy in `{% if %}`, and trigger Tera's `default` filter. `recent_count` serializes as a number.
- AC-1.11 The page, section and home render items all have `config` in their Tera context.
- AC-1.12 `ensure_safe_to_clean` in `build` treats the config file path as protected. An output folder that contains the config file is refused before cleaning. stderr names the output path and the config path, and nothing in the output folder changes.
- AC-1.13 `MangoError::Config(String)` displays as `Mango Config Error: <msg>`.
- AC-1.14 `test/mango.json` exists with a title and an author. The fixture e2e test passes `--config test/mango.json` and asserts that the configured title is inside `<title>…</title>` and the author is inside `<footer>…</footer>` of `posts/post_one/index.html`.

### REQ-2 Home page
`home.html` renders to `dist/index.html`, listing recent pages and top-level sections.
- AC-2.1 `home::build` returns one `RenderItem` with slug `""`, template `home.html` and source `home page`. `output::render` maps it to `<dist>/index.html`.
- AC-2.2 `home.recent` contains dated pages from the whole site, sorted with `compare_summaries` (newest first, then title, then slug). Undated pages are left out, and the list stops at `config.recent_count` entries (0 gives an empty list).
- AC-2.3 Each `home.recent` entry is a page summary with `title`, `date`, `slug` and `url`.
- AC-2.4 `home.sections` lists the top-level sections (slugs without `/`), sorted by slug. Each entry has `slug`, `url` (`/<slug>/`) and `page_count` (the number of direct child pages, the same as the length of that section's `pages`). Nested sections such as `a/b` are not listed.
- AC-2.5 The home item goes through `check_collisions` with the page and section items, and through `output::render`, before `ensure_safe_to_clean` and `clean_contents`.
- AC-2.6 If `home.html` is missing from the templates folder, the build fails before cleaning. stderr contains `home.html`, and the previous output (including a marker file) is unchanged.
- AC-2.7 Building the fixture writes `index.html`, which extends `base.html`. "Mango Task Tracker" appears before "Post One", and the page links to sections using `url`.
- AC-2.8 The dead-code warning for `home::build` is gone.

### REQ-3 Index pages for every section level
- AC-3.1 A page at `a/b/c` creates section index entries for both `a/b` and `a`. A top-level page creates no section, and the root is never a section.
- AC-3.2 Each section's `pages` holds only its direct child pages, sorted as today. A section that only contains subfolders has an empty `pages` list.
- AC-3.3 Each section has `subsections`: its direct child sections, sorted by slug. Each entry has `slug` and `url`. For pages `a/b/c` and `a/d/e`, section `a` has subsections `a/b` and `a/d`, and `a/b` has none.
- AC-3.4 The section context also has `url` (`/<section slug>/`).
- AC-3.5 An e2e build of a temp site containing only `a/b/c.md` writes `a/index.html` (containing `href="/a/b/"`) and `a/b/index.html` (containing `href="/a/b/c/"`).
- AC-3.6 An ancestor section index takes part in the collision check. A temp site with `a.md` and `a/b/c.md` fails naming the path, `page 'a'` and `section index 'a'`, and cleans nothing.
- AC-3.7 Section render items still come out in ascending slug order, ancestors included (for example `a`, `a/b`, `posts`).

### REQ-4 Markdown extensions
`to_html` enables `ENABLE_TABLES`, `ENABLE_FOOTNOTES`, `ENABLE_STRIKETHROUGH`, `ENABLE_TASKLISTS` and `ENABLE_HEADING_ATTRIBUTES`.
- AC-4.1 A pipe table renders `<table>`.
- AC-4.2 `text[^1]` with `[^1]: note` renders `<sup class="footnote-reference">` and `<div class="footnote-definition"`.
- AC-4.3 `~~gone~~` renders `<del>gone</del>`.
- AC-4.4 `- [ ] todo` renders an `<input` element containing `type="checkbox"`.
- AC-4.5 `# Title {#my-id}` renders `<h1 id="my-id">`.

### REQ-5 Page and section URLs
- AC-5.1 `PageSummary` serializes a `url` field equal to `"/<slug>/"`.
- AC-5.2 `PageTemplate` (the `page` context) has `url` equal to `"/<slug>/"`.
- AC-5.3 Section contexts (`section.url`, `section.subsections[].url`) and home section entries (`home.sections[].url`) use `"/<section slug>/"`.
- AC-5.4 After building the fixture, `posts/index.html` contains `href="/posts/post_one/"` exactly and does not contain `&#x2F;`.
- AC-5.5 Generated URLs are root-relative and never include `base_url`.

### REQ-6 Batch 2 review leftovers
- AC-6.1 `page_context_date_is_formatted_or_empty` gives the dated page a real slug and asserts `item.source == "page '<slug>'"`. It no longer asserts `"page ''"`.
- AC-6.2 `cli::tests::refuses_cwd` passes a non-canonical target such as `dir.join("child/..")` (with `child` created) and `dir` as the cwd. It asserts that the message contains both the non-canonical target's display string and the cwd's display string.
- AC-6.3 In `clean_contents`, if an entry is a symlink and `remove_file` fails, `remove_dir` is tried. If that also fails, the error names the entry path. Non-symlink files behave as before, and the existing Unix test `clean_contents_empties_but_keeps_dir` still passes.

### REQ-7 Documentation and done criteria
- AC-7.1 CLAUDE.md is updated:
  - It describes the config file and `--config`: optional default, required when given explicitly, `deny_unknown_fields`, the field list, `null` for unset fields, and protection from cleaning.
  - It describes the home page and the `home`, `section.subsections` and `url` template contexts, and lists `home.html` as a required template.
  - It describes ancestor section indexes and the markdown extensions.
  - It removes the two known issues (the home stub and the missing `config` context), and adds `MangoError::Config` to the module map.
- AC-7.2 `cargo build`, `cargo test`, `cargo clippy --all-targets` and `cargo fmt --check` all pass with no new warnings.
- AC-7.3 `Cargo.toml` dependencies are unchanged.

## Verified by

From the run's test report (`03-test.v1.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-1.1 | new | `help_shows_flag_descriptions`; `build --help` check | pass |
| AC-1.2 | new | `config::tests::missing_default_file_uses_defaults`, `build_without_config_uses_defaults` (`<title>` of page and home contains My Site) | pass |
| AC-1.3 | new | `build_reads_default_mango_json_from_cwd` (checks both `<title>` and `<footer>`) | pass |
| AC-1.4 | new | `config::tests::explicit_missing_file_is_config_error_naming_path`, `build_fails_when_explicit_config_missing` (failure, empty stdout, path in stderr, output unchanged) | pass |
| AC-1.5 | new | `config::tests::malformed_json_is_config_error_naming_path`, `build_fails_on_malformed_or_unknown_field_config` (Config error, path, serde message, output unchanged) | pass |
| AC-1.6 | new | `config::tests::unknown_field_is_config_error_naming_path_and_field`, same end-to-end test (`titel`) | pass |
| AC-1.7 | new | `config::tests::negative_recent_count_is_config_error` (`-1`, `2.5`, `"3"`); `-1` end-to-end check | pass |
| AC-1.8 | new | `config_error_reported_before_templates_error`; `cli::build` loads config before `load_templates` | pass |
| AC-1.9 | new | `missing_default_file_uses_defaults`, `partial_config_keeps_defaults` | pass |
| AC-1.10 | new | `config::tests::unset_fields_serialize_as_null`, `template::tests::page_context_includes_config`, `template::tests::null_config_title_uses_default_filter` (renders `My Site\|\|no`) | pass |
| AC-1.11 | new | `page_context_includes_config`, `section_context_includes_config_url_and_subsections`, `home_item_fields_and_context`, `home_item_has_empty_slug_and_source` | pass |
| AC-1.12 | new | `build_refuses_output_containing_config` (stderr has `'out'` and `'out/mango.json'`, snapshot unchanged); `config_path` is in the protected list | pass |
| AC-1.13 | new | `error::tests::config_display` | pass |
| AC-1.14 | new | `build_generates_site_from_fixture` (title inside `<title>`, author inside `<footer>`); fixture build check; `test/mango.json` present | pass |
| AC-2.1 | new | `home_item_has_empty_slug_and_source`, `home_item_fields_and_context`, `output::tests::home_slug_maps_to_root_index` | pass |
| AC-2.2 | new | `home::tests::recent_is_dated_sorted_and_truncated` (undated dropped, sort and tie-break, limits 2 and 0), `home_recent_respects_recent_count_and_skips_undated` | pass |
| AC-2.3 | new | `recent_is_dated_sorted_and_truncated` (title, date, url, slug) | pass |
| AC-2.4 | new | `home::tests::sections_are_top_level_sorted_with_counts` (`a`, `posts`, `projects`; counts 0/2/1; `a/b` excluded) | pass |
| AC-2.5 | new | Code: home item is chained into `check_collisions` and rendered before `ensure_safe_to_clean` and `clean_contents`; `home_slug_maps_to_root_index` | pass |
| AC-2.6 | new | `missing_home_template_keeps_previous_output` (stderr has `home.html`, marker and snapshot unchanged) | pass |
| AC-2.7 | new | `build_generates_site_from_fixture` (DOCTYPE from base, Mango Task Tracker before Post One, `href="/posts/"`); fixture build check | pass |
| AC-2.8 | new | clean rebuild shows 0 warnings; clippy clean | pass |
| AC-3.1 | new | `ancestors_get_section_entries`, `top_level_pages_are_not_indexed` | pass |
| AC-3.2 | new | `section_with_only_subsections_has_empty_pages`, `subsections_are_direct_children_sorted` | pass |
| AC-3.3 | new | `subsections_are_direct_children_sorted` (`a` → `a/b`, `a/d`; `a/d` has none), `render_items_in_section_slug_order` | pass |
| AC-3.4 | new | `section_context_includes_config_url_and_subsections` (`url` = `/a/`) | pass |
| AC-3.5 | new | `build_generates_ancestor_section_indexes` | pass |
| AC-3.6 | new | `build_fails_on_page_and_ancestor_section_collision` (path, `page 'a'`, `section index 'a'`, snapshot is only the marker) | pass |
| AC-3.7 | new | `generate::section::tests::render_items_in_section_slug_order`, `index::section::tests::sections_iterate_in_slug_order` | pass |
| AC-4.1 | new | `markdown::tests::renders_tables` | pass |
| AC-4.2 | new | `markdown::tests::renders_footnotes` | pass |
| AC-4.3 | new | `markdown::tests::renders_strikethrough` | pass |
| AC-4.4 | new | `markdown::tests::renders_tasklists` | pass |
| AC-4.5 | new | `markdown::tests::renders_heading_attributes` | pass |
| AC-5.1 | new | `index::section::tests::summary_serializes_url`, `page::tests::slug_url_is_root_relative_with_trailing_slash` | pass |
| AC-5.2 | new | `template::tests::page_context_includes_url` | pass |
| AC-5.3 | new | `section_context_includes_config_url_and_subsections`, `home_item_fields_and_context`, `sections_are_top_level_sorted_with_counts` | pass |
| AC-5.4 | new | `build_generates_site_from_fixture` (exact `href`, no `&#x2F;`); fixture build check | pass |
| AC-5.5 | new | `slug_url` only formats the slug and never reads `base_url`; `slug_url_is_root_relative_with_trailing_slash` | pass |
| AC-6.1 | changed | `template::tests::page_context_date_is_formatted_or_empty` asserts `page 'posts/dated'` | pass |
| AC-6.2 | changed | `cli::tests::refuses_cwd` (target `dir/child/..`; message contains both quoted display strings) | pass |
| AC-6.3 | new | `clean_contents_empties_but_keeps_dir` passes on Linux. Windows fallback checked by reading `clean_contents`: on a failed `remove_file` for a symlink it tries `remove_dir`, and a second failure goes through `io_at(&path, …)`. `clean_contents_removes_dir_symlink` is `#[cfg(windows)]` and was not run. | pass (Windows path by reading only) |
| AC-7.1 | new | CLAUDE.md diff covers config and `--config` (optional default, explicit required, `deny_unknown_fields`, fields, `null`, protection), the `home`/`section.subsections`/`url` contexts, required `home.html`, ancestor indexes, markdown extensions, the two known issues removed, and `Config` in the module map | pass |
| AC-7.2 | new | build, test, clippy `--all-targets` and fmt `--check` all exit 0 with 0 warnings | pass |
| AC-7.3 | new | `git diff --quiet Cargo.toml Cargo.lock` exits 0 | pass |
