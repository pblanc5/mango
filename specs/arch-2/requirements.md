<!-- Published by dev-pipeline run 20260923-230744-arch-2-one-output-model, stage spec v2, approved 2026-09-23T23:39:01-05:00. -->

# One output model instead of three: Requirements

Spec ID: `arch-2`

## Summary
A build produces eight kinds of output: content pages, section indexes, the home page, the tag index, tag pages, the RSS feed, the sitemap and asset copies. Today these come in three unrelated shapes: template-rendered items, generated files and asset files. Collision checking, rendering and writing each handle the three shapes separately. Two fields exist only to work around that split. One is a free-text `source` label, used only in error messages. The other is a page date that was added to every template item so the sitemap could find `<lastmod>`. This spec asks for one output model. Every output is one kind of value, collision labels and sitemap data come from what each output is, and collision checking, rendering and writing each work over one list. For site authors nothing changes: no output byte, message, order or exit status.

## Context
- **Who needs it.** Maintainers. Adding a new output kind means extending one model instead of three code paths, and the two workaround fields go away. ARCH-5 (break up `render/template.rs`) is listed as "easier after ARCH-2", and its proposal says the output type moves into `build/` "or is replaced by ARCH-2's `Output`". Open question 1 decided that ARCH-2 closes that part. FEAT-1 (dev server) consumes the plan, so this change must keep the plan's public shape stable (Open question 2).
- **Source.** `specs/_system/backlog.md#arch-2` gives the problem, the proposal and when it is done. The proposal sketches `ItemKind`, `Body` and `Output`, with the collision label as `Display for ItemKind`. This document treats those as capabilities. Names and shapes are left to the design.
- **Depends on.** ARCH-1 (done): the `plan`/`commit` seam, the `BuildPlan` with private fields, and `PlannedOutput`. ARCH-3 (done): `Slug`/`Tag` types, with `Slug::output_path` giving the output location of every slug-addressed kind.
- **Where the three shapes live today** (read at `6d79e2e`):
  - Template items: `RenderItem { slug, source, template, context, page_date }` in `src/render/template.rs:143-152`. Built by `render_page`, `render_section_page`, `render_home_page`, `render_tag_index` and `render_tag_page` (`src/render/template.rs:174-275`), and called from `src/build/generate/{content,section,home,tag}.rs`.
  - Generated files: `GeneratedFile { path, source, contents }` in `src/build/output.rs:17-25`. Built by `src/build/generate/feed.rs:58-62` and `src/build/generate/sitemap.rs:39-43`.
  - Asset files: `AssetFile { source, dest, label }` in `src/build/generate/assets.rs:8-16`. Built by `assets::plan` and copied by `assets::copy`.
  - The three are joined in `src/build/pipeline.rs:122-165`. `check_collisions` (`src/build/output.rs:33-89`) takes three separate inputs. `render` and `render_generated` (`output.rs:93-119`) produce `RenderedFile`s. `write` (`output.rs:121-131`) writes them and `assets::copy` copies the assets.
- **Guarantees this change must keep:**
  - `specs/_system/legacy-criteria/`: batch 2 covers collisions and rendering in memory before cleaning, batch 3 covers the home item, and batch 4 covers the collision list, feed and sitemap.
  - `specs/arch-1/requirements.md`: the plan/commit seam, plan enumeration and order, and determinism.
  - `specs/arch-3/requirements.md`: slug and tag types, output locations and labels.
  - The Current behavior section below restates the parts this change touches.
- **Rules that bind this change** (`specs/constitution.md`):
  - no new dependencies
  - no dead code and no `#[allow(dead_code)]`
  - the library's public surface is exactly seven re-exports
  - every behaviour change has a test that fails without it
  - determinism is enforced by `fixture_build_is_deterministic`
  - bad input fails before the output folder is cleaned
  - docs change with the code
  - new criteria tags are spec-scoped. Following the `arch-1` and `arch-3` precedent, code comments for this spec's criteria read `// AC-arch-2.<req>.<m>`, for example `// AC-arch-2.7.3` for AC-7.3.
- **Related items:**
  - ARCH-4 (structured error variants) owns error types and texts. This change keeps every collision message byte-identical and the `General` variant.
  - ARCH-5 owns splitting view models from Tera loading. Its bullet "`build` no longer imports its core type from `render`" is closed by this change (Open question 1).
  - ARCH-6 owns `recent_pages`/feed tidying and `compare_summaries`.
  - DOC-1 owns stale references in `specs/_system/overview.md`, including its mention of `GeneratedFile` (line 94).

## Current behavior

### REQ-1 [baseline] What a build plans, and where each output goes
Evidence: `src/build/pipeline.rs:104-165`, `src/render/template.rs:174-275`, `src/build/generate/feed.rs:14-63`, `src/build/generate/sitemap.rs:14-44`, `src/build/generate/assets.rs:21-85`, `src/content/slug.rs` (`output_path`); tests `plan_holds_every_output_fully_rendered` (`tests/plan.rs`), `build_generates_site_from_fixture` (`tests/build.rs`), `home_slug_maps_to_root_index` (`src/build/output.rs`), `plan_is_sorted_labelled_and_touches_nothing` (`src/build/generate/assets.rs`).
Status: inferred, confirm at approval
- AC-1.1 [baseline] THE planning phase SHALL produce one output per non-draft content page, one per section (every ancestor folder of a page), one home page, one tag index (always, even with no tags) and one per distinct tag. Each is rendered through its fixed template: `page.html`, `section.html`, `home.html`, `tags.html` or `tag.html`, respectively.
- AC-1.2 [baseline] THE planning phase SHALL place each output from AC-1.1 at `<output folder>/<slug>/index.html`, and the home page at `<output folder>/index.html`. The tag index slug is `tags` and a tag page slug is `tags/<tag>`.
- AC-1.3 [baseline] WHILE `base_url` is set THE planning phase SHALL produce the RSS feed at `<output folder>/feed.xml` and the sitemap at `<output folder>/sitemap.xml`. Their contents are generated in memory rather than through a template. WHILE `base_url` is unset THE planning phase SHALL produce neither.
- AC-1.4 [baseline] THE planning phase SHALL plan one copy per file under the assets folder, recursively. Each copy's destination is `<output folder>/<assets folder name>/<path relative to the assets folder>`, and asset copies are sorted by destination. The copy writes the source file's bytes; a symlinked file is copied as its contents.

### REQ-2 [baseline] Outputs have one fixed order, used for collision checking, rendering, enumeration and writing
Evidence: `src/build/pipeline.rs:122-165`, `src/content/loader.rs:25-90` (pages in directory-listing order), `src/build/generate/section.rs:7-21`, `src/build/generate/tag.rs:9-24`; tests `plan_holds_every_output_fully_rendered`, `planning_twice_gives_identical_outputs`, `commit_writes_exactly_the_enumerated_outputs` (`tests/plan.rs`), `render_items_in_section_slug_order`, `index_first_then_tags_in_name_order`.
Status: inferred, confirm at approval
- AC-2.1 [baseline] THE planning phase SHALL order outputs as follows:
  1. content pages, in the order the content loader returns them
  2. section indexes, in byte-wise slug order
  3. the home page
  4. the tag index
  5. tag pages, in tag-name order
  6. the feed
  7. the sitemap
  8. asset copies, in destination order
- AC-2.2 [baseline] THE build plan SHALL enumerate its outputs in the order of AC-2.1. Rendered and generated files come first, then asset copies. Each file is exposed with its path relative to the output folder and its exact contents, and each copy with its relative destination and source path.
- AC-2.3 [baseline] WHEN a plan is committed THE commit phase SHALL write every rendered and generated file in the order of AC-2.1 and then copy every asset in the order of AC-2.1. Parent folders are created as needed.
- AC-2.4 [baseline] THE content loader SHALL return pages in the order the operating system lists directory entries, which is not sorted. So the relative order of content pages in AC-2.1 (and in the enumeration of AC-2.2) can differ between filesystems. Output bytes do not depend on it. See Open question 4.

### REQ-3 [baseline] Collision checking and its messages
Evidence: `src/build/output.rs:27-89`, `src/build/generate/assets.rs:58,77-81`, `src/render/template.rs:186,211,231,248,269`, `src/build/generate/feed.rs:60`, `src/build/generate/sitemap.rs:41`; tests `collision_names_path_and_both_sources`, `index_md_is_not_a_collision`, `distinct_paths_pass`, `home_slug_maps_to_root_index`, `generated_file_collides_with_render_item`, `file_vs_directory_conflict_is_detected`, `asset_conflicts_are_detected` (`src/build/output.rs`), `page_and_section_index_collision_fails_planning`, `file_vs_folder_conflict_fails_planning` (`tests/plan.rs`), `build_fails_on_page_and_section_output_collision`, `build_fails_on_page_and_ancestor_section_collision`, `build_fails_on_tags_folder_collision`, `build_fails_on_top_level_tags_page_collision`, `build_fails_on_file_vs_folder_conflict_keeping_output`, `build_fails_when_page_lands_on_an_asset_keeping_output` (`tests/build.rs`).
Status: inferred, confirm at approval
- AC-3.1 [baseline] THE collision check SHALL label each output as follows:
  - a content page: `page '<slug>'`
  - a section index: `section index '<slug>'`
  - the home page: `home page`
  - the tag index: `tag index`
  - a tag page: `tag page '<tag>'`
  - the feed: `RSS feed`
  - the sitemap: `sitemap`
  - an asset copy: `asset '<path relative to the assets folder, with / separators>'`
- AC-3.2 [baseline] IF two outputs map to the same file THEN THE collision check SHALL fail with a `General` error that displays as `Mango Error: output path '<full output path>' would be written by both <first label> and <second label>`. Here `<first label>` is the output that comes earlier in the order of AC-2.1, and only the first such pair in that order is reported.
- AC-3.3 [baseline] IF no two outputs map to the same file, and an output's path needs a folder where another output is a file, THEN THE collision check SHALL fail with a `General` error that displays as `Mango Error: output path '<file path>' would be written as a file by <file's label>, but <label> needs it to be a directory for '<full output path>'`. Outputs are examined in the order of AC-2.1. For each output, its ancestors are examined from the nearest upward, stopping at the output folder, and the first conflict found is reported.
- AC-3.4 [baseline] THE collision check SHALL NOT treat `posts/index.md` (output `posts/index/index.html`) as colliding with the `posts` section index.
- AC-3.5 [baseline] THE planning phase SHALL run the collision check after building every output and before rendering any template. A collision is therefore reported even when a template would also fail to render. Like every planning failure, it leaves the output folder untouched.

### REQ-4 [baseline] The sitemap lists every HTML output and dates content pages only
Evidence: `src/build/generate/sitemap.rs:11-44`, `src/build/pipeline.rs:130-145`, `src/render/template.rs:149-151,189,215,234,251,273`; tests `covers_every_item_kind_sorted`, `sorted_by_loc_as_plain_strings`, `lastmod_only_on_dated_pages`, `escapes_loc`, `trailing_slash_base_url` (`src/build/generate/sitemap.rs`), `page_context_tags_are_links`, `tag_index_context_and_source`, `tag_page_context_and_source` (`src/render/template.rs`, `page_date` assertions), `fixture_feed_and_sitemap` (`tests/build.rs`).
Status: inferred, confirm at approval
- AC-4.1 [baseline] WHILE `base_url` is set THE sitemap SHALL contain exactly one `<url>` per content page, section index, home page, tag index and tag page. It SHALL NOT list the feed, the sitemap itself or any asset.
- AC-4.2 [baseline] THE sitemap SHALL set each `<loc>` to `<base_url without trailing slashes><url>`, XML-escaped, and SHALL sort entries by `<loc>` as plain strings.
- AC-4.3 [baseline] THE sitemap SHALL emit `<lastmod>YYYY-MM-DD</lastmod>` only for content pages that have a `date`. It SHALL emit none for undated pages, sections, the home page, the tag index or tag pages.
- AC-4.4 [baseline] THE template item type SHALL carry a page date that is set only for content pages and read only by the sitemap. Every other item kind carries an empty date.

### REQ-5 [baseline] Rendering happens during planning; writing and copying happen during commit
Evidence: `src/build/output.rs:91-131`, `src/build/generate/assets.rs:87-97`, `src/build/pipeline.rs:149-181`; tests `render_returns_paths_and_html_without_touching_fs`, `render_error_returns_template_error`, `render_generated_maps_paths_without_touching_fs`, `write_creates_parent_dirs_and_files` (`src/build/output.rs`), `copy_failure_is_an_error_naming_the_path` (`src/build/generate/assets.rs`), `page_render_error_keeps_previous_output`, `section_render_error_keeps_previous_output` (`tests/build.rs`).
Status: inferred, confirm at approval
- AC-5.1 [baseline] WHEN planning succeeds THE build plan SHALL already hold every template output rendered to text, and the feed and sitemap text, so no template error can occur during commit.
- AC-5.2 [baseline] IF a template fails to render THEN THE planning phase SHALL fail with a `Template` error. The error is for the first output in the order of AC-2.1 whose template fails, and the output folder is untouched.
- AC-5.3 [baseline] IF writing a file fails during commit THEN THE commit phase SHALL fail with an `IoPath` error naming the file's full output path, or the parent folder it could not create.
- AC-5.4 [baseline] IF copying an asset fails during commit THEN THE commit phase SHALL fail with an `IoPath` error naming the asset's **source** path. It does not name the destination. See Open question 3.

## User stories
- As a maintainer, I want every output the build produces to be one kind of value, so that adding an output kind means extending one model instead of three separate code paths.
- As a maintainer, I want an output's collision label to follow from what the output is, so that no free-text string exists only for error messages and a label cannot drift from its output.
- As a maintainer, I want the sitemap to learn a page's date from the page output itself, so that no field is carried on every output just for the sitemap.
- As the author of ARCH-5 and FEAT-1, I want one output type with a stable plan on top, so that I can build on it without reworking three types.
- As a site author, I want my site to build to exactly the same bytes, with the same error messages, so that this refactor is invisible to me.

## Requirements

### REQ-6 Every planned output is one type
- AC-6.1 THE build SHALL represent every planned output, of all eight kinds in AC-1.1 to AC-1.4, as a value of one output type. Each value says three things:
  - where it goes under the output folder
  - what kind of output it is
  - how its bytes are produced: a template to render with its context, text generated in memory, or a file to copy
- AC-6.2 THE build SHALL no longer contain the three separate output shapes (today's template item, generated file and asset file types) or any other type that duplicates their role. Verified by review of the design's list of removed and replaced types.
- AC-6.3 THE planning phase SHALL assemble one list of outputs in the order of AC-2.1. THE collision check, rendering, plan enumeration and commit SHALL each work over that one list, or over its rendered form, in that order. None of them SHALL take a separate input per output shape.
- AC-6.4 THE output kind SHALL carry what is needed to identify an output of that kind:
  - the slug of a content page or section index
  - the tag of a tag page
  - the asset's path relative to the assets folder
  - the date of a content page, or its absence

  No output SHALL carry a field that is meaningful for only some kinds and left empty for the rest.
- AC-6.5 WHEN planning succeeds THE build plan SHALL still hold every output fully rendered or generated, with only asset copies deferred to commit (AC-5.1). THE commit phase SHALL still be the only operation that empties or writes the output folder.

### REQ-7 Collision labels come from the output's kind
- AC-7.1 THE collision check SHALL derive each output's label from its kind and identity (AC-6.4). No output SHALL hold a free-text label or description whose only use is error messages. Verified by review, backed by AC-7.3.
- AC-7.2 THE collision check SHALL keep the exact message formats, variant, ordering and first-reported pair of AC-3.2 to AC-3.5.
- AC-7.3 THE test suite SHALL include a unit test, tagged `// AC-arch-2.7.3`, that checks the derived label of each of the eight kinds against the exact text of AC-3.1. For assets it SHALL use a nested path, for example `asset 'css/main.css'`.
- AC-7.4 THE test suite SHALL include an in-process test, tagged `// AC-arch-2.7.4`, that plans a site where an asset and a content page map to the same output file. It SHALL check the full exact-collision message of AC-3.2 and that the earlier output in AC-2.1 order is named first. This covers the one cross-kind pair (page vs asset) that no exact-string in-process test pins today.

### REQ-8 The sitemap reads page dates from the page output
- AC-8.1 THE sitemap SHALL take each `<lastmod>` from the date carried by a content page output's kind (AC-6.4). No field whose only reader is the sitemap SHALL exist on the output type.
- AC-8.2 THE sitemap SHALL decide whether an output is listed from the output's kind alone. Content pages, section indexes, the home page, the tag index and tag pages are listed. The feed, the sitemap and asset copies are not (AC-4.1). See Open question 5.
- AC-8.3 THE test suite SHALL include a unit test, tagged `// AC-arch-2.8.3`, that builds the sitemap from a list containing at least one output of every kind, including the feed and an asset copy. It SHALL assert that exactly the HTML kinds are listed and that `<lastmod>` appears only on dated content pages.
- AC-8.4 THE sitemap and feed SHALL keep their exact bytes, order and escaping (AC-4.2, AC-4.3). The existing sitemap and feed tests SHALL pass with no change to their expected XML.

### REQ-9 Nothing else changes
- AC-9.1 THE `mango build` command SHALL produce byte-identical output for the fixture site. `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `fixture_internal_links_resolve` and every other `fixture_*` test SHALL pass without edits to their expectations.
- AC-9.2 THE `mango build` command SHALL keep every error text, error variant, exit status and failure order in REQ-3 and REQ-5, including the precedence of a collision over a render error (AC-3.5). Every existing test in `tests/build.rs` and `tests/plan.rs` SHALL pass with no change to an expected string, path, order or status.
- AC-9.3 THE build plan SHALL keep the enumeration of AC-2.2 unchanged in content and order, including the file-then-copy split.
- AC-9.4 THE library's public surface SHALL stay exactly the seven re-exports (`plan`, `commit`, `clean`, `BuildOptions`, `BuildPlan`, `PlannedOutput`, `MangoError`), and `PlannedOutput` SHALL keep its two variants and fields (Open question 2). The new output type SHALL be crate-private, and no `#[allow(dead_code)]` SHALL be added.
- AC-9.5 WHEN a unit test asserts on a field or function this change removes (for example `.source`, `.page_date`, `.label`, `GeneratedFile`, `render_generated`) THE test suite SHALL keep an equivalent assertion against the new model. The rewritten test SHALL carry the original `// AC-…` tag, and no legacy criterion in `specs/_system/legacy-criteria/` SHALL lose its last verifying test. The design SHALL list each affected test with its replacement.
- AC-9.6 THE change SHALL add no dependency, dev-dependencies included, and SHALL keep the gate green: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- AC-9.7 THE `README.md` SHALL need no change, since nothing a site author sees changes.

### REQ-10 Developer documentation and backlog follow the code
- AC-10.1 THE `CLAUDE.md` SHALL describe the single output model in its pipeline, feed/sitemap and module-map text. It SHALL no longer name `RenderItem`, `RenderItem.page_date`, `GeneratedFile`, `AssetFile`, `render_generated` or free-text collision sources as current code. Where it says the sitemap's entries are "derived from the render items", it SHALL say they are derived from the outputs' kinds.
- AC-10.2 THE backlog SHALL mark ARCH-2 `done` in the squash commit that lands it, with `(ARCH-2)` in the subject and a "Landed." note. In the same squash commit, the backlog SHALL also:
  - update the ARCH-5 entry so that it says its "`build` no longer imports its core type from `render`" bullet landed with ARCH-2, because the unified output type lives under `build/` (Open question 1). The rest of ARCH-5 (view models, Tera loading, item construction, `add_content`) stays open, and its proposal no longer reads as if the output type still has to move.
  - add a new open item RISK-7, listed in the index and with its own section, describing the asset-copy quirk of AC-5.4 (the error names the source path even when the destination is the problem) and proposing to name the destination, or both paths (Open question 3).
  - add a new open item RISK-8, listed in the index and with its own section, describing that content page order (AC-2.4) makes plan enumeration, write order and which page's render error is reported first vary across filesystems while output bytes do not, and proposing to sort pages by slug in the loader (Open question 4).

  IF another item has taken RISK-7 or RISK-8 by the time this lands THEN the two items SHALL take the next free RISK numbers instead, in the same order.

## Out of scope
- Splitting view models from Tera loading, and moving item construction out of `render/template.rs` (ARCH-5), except the output type's placement under `build/` as decided under Open question 1.
- Structured error variants or any change to error text (ARCH-4).
- Tidying `recent_pages`, the feed's unreachable `continue`, or moving `compare_summaries` (ARCH-6).
- Changing the order of content pages returned by the loader (AC-2.4, Open question 4; tracked as RISK-8 by AC-10.2).
- Changing which path an asset-copy failure names (AC-5.4, Open question 3; tracked as RISK-7 by AC-10.2).
- Extending `PlannedOutput` or the public API, for example exposing output kinds (Open question 2).
- Refreshing `specs/_system/overview.md` (DOC-1).
- Any change to URLs, output paths, listing order, template contexts, feed or sitemap format, or the fixture site.

## Open questions
All five were decided by the approver at the requirements gate on 2026-09-23 ("go with your take").

1. **Should the unified output type live under `build/` rather than `render/`?** Today `build` imports `RenderItem` from `render`, which ARCH-5 lists as a smell ("`build` no longer imports its core type from `render`"). Since ARCH-2 creates the replacement type anyway, it could satisfy that ARCH-5 bullet for free. **Decided: yes.** The unified output type lives under `build/`, which closes that ARCH-5 bullet. AC-10.2 updates ARCH-5's backlog text to say the bullet landed with ARCH-2; the rest of ARCH-5 (view models, Tera loading, item construction, `add_content`) stays open.
2. **Should `PlannedOutput` change?** It is public and exposes `File { path, contents }` / `Copy { path, source }`. The new kind could be exposed too, for example for FEAT-1. **Decided: no.** `PlannedOutput` and the public surface stay unchanged (AC-9.4 as written). FEAT-1 may extend it under its own spec when it needs to.
3. **Suspected quirk: an asset-copy failure names the source, not the destination (AC-5.4).** `assets::copy` maps an `fs::copy` failure with `io_at(&file.source, …)`. If the destination is the problem, for example an existing folder, the message points at the input. `copy_failure_is_an_error_naming_the_path` passes only because both paths contain `style.css`. **Decided: keep the behavior unchanged in this change**, and open a new backlog item RISK-7 in the same squash commit, proposing to name the destination (or both). See AC-10.2.
4. **Content page order is filesystem-dependent (AC-2.4).** The loader returns pages in `read_dir` order. So the plan's enumeration order, the write order and which page's render error is reported first can differ across filesystems. Output bytes cannot differ, and a page-vs-page collision prints the same label either way. **Decided: keep the behavior unchanged in this change**, and open a new backlog item RISK-8 in the same squash commit, proposing to sort pages by slug in the loader. See AC-10.2.
5. **Sitemap selection by kind (AC-8.2, AC-8.3).** These criteria require the sitemap to filter by kind, and the unit test hands it the feed and an asset. So the rule "the sitemap lists only HTML outputs" holds by the kind, not by which list the caller happens to pass in. The alternative was to leave the filtering to the caller and test only the dates. **Decided: keep AC-8.2 and AC-8.3 as written.** This was settled at the gate because the backlog item's proposal asks for exactly this, so it is a requirement and not a design choice the architect may reopen.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-23 | 20260923-230744-arch-2-one-output-model | New spec from backlog ARCH-2. Baseline of output kinds, locations, ordering, collision labels and messages, sitemap selection and dating, and the render/write split. Requires one crate-private output type over which collision checking, rendering, enumeration and commit work; labels derived from kind; sitemap dates and selection from kind. Site-author output, messages and public API unchanged. |
| 2026-09-23 | 20260923-230744-arch-2-one-output-model | Gate decisions recorded under Open questions 1 to 5: the output type lives under `build/` and closes ARCH-5's "core type from `render`" bullet; `PlannedOutput` unchanged; asset-copy error and content page order kept, tracked as new items RISK-7 and RISK-8; sitemap filters by kind as written. AC-10.2 extended to update ARCH-5 and add RISK-7 and RISK-8 in the landing commit. No IDs changed. |
