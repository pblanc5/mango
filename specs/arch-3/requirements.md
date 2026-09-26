<!-- Published by dev-pipeline run 20260923-220004-arch-3-slug-tag-newtypes, stage spec v4, approved 2026-09-23T22:38:41-05:00. -->

# `Slug` and `Tag` newtypes: Requirements

Spec ID: `arch-3`

## Summary
A page's slug is the path of its content file relative to the site folder, without the extension. The slug sets the page's URL, its output file, its parent section, whether it appears on the home page, and where it sorts in listings. Today the slug is a plain string, and the code that builds it and takes it apart is spread over five modules. Tags are also plain strings: they are checked once and then trusted by convention. This spec makes slugs and tags into values that can only exist after they pass validation. Every derived fact (URL, output location, parent, top-level status, segments, tag page location) must come from those values, not from string handling at each call site. Only one thing changes for site authors: a file-name segment made only of dots (for example `...md`) is now rejected with an error that gives that reason (REQ-8). Today such a segment can write output outside its intended location.

## Context
- **Who needs it.** Maintainers get one home for the slug rules, and "valid by construction" becomes a property the compiler checks. ARCH-2 (one output model) depends on this item: its `Tag(Tag)` item kind and its output paths assume these types exist.
- **Source.** `specs/_system/backlog.md#arch-3`, which states the problem, proposal and done-when. The proposal names `Slug`, `Tag`, `parent()`, `is_top_level()`, `segments()`, `url()`, `output_path(dist)` and `slug()`. This document treats those as capabilities and leaves names and shapes to the design.
- **Where slug logic lives today** (from the backlog, confirmed by reading the code at `cf83051`):
  - Building and validating: `src/content/page.rs:48-74` (`generate_slug`, `is_valid_slug_segment`).
  - URL: `src/content/page.rs:78-84` (`slug_url`). It is called from `src/render/template.rs` (tag links, tag index entries, page, section, subsection, home-section and tag-page URLs), `src/content/summary.rs:21`, `src/build/generate/feed.rs:36` and `src/build/generate/sitemap.rs:23`.
  - Tag page location: `src/content/page.rs:87-89` (`tag_slug`). The tag index slug is the literal `"tags"` at `src/render/template.rs:250`, and the home slug is the literal `""` at `src/render/template.rs:233`.
  - Output path: `src/build/output.rs:133-144` (`get_final_path`).
  - Parent section: `src/build/index/section.rs:73-75` (`extract_parent_slug`, `rsplit_once('/')`).
  - Top-level test: `src/build/generate/home.rs:32` (`!slug.contains('/')`).
  - Tag validation: `src/content/page.rs:93-118` (`validate_tags`).
- **Guarantees this change must keep** are in `specs/_system/legacy-criteria/` (batch 2 for ordering, collisions and URLs; batch 4 for tags, feed and sitemap) and in the `CLAUDE.md` rules "File names are strict" and "Tags are strict". The Current behavior section restates the parts this change touches.
- **Rules that bind this change** (`specs/constitution.md`):
  - no new dependencies
  - no dead code and no `#[allow(dead_code)]`
  - the library's public surface is exactly seven re-exports
  - every behavior change has a test that fails without it
  - determinism is enforced by `fixture_build_is_deterministic`
  - docs change with the code
  - new criteria tags are scoped to their spec. Following the `arch-1` precedent, code comments for this spec's criteria read `// AC-arch-3.<req>.<m>`, for example `// AC-arch-3.5.2` for AC-5.2.
- **Related items:**
  - ARCH-6 (tidy the `Page` model) owns two-phase `Page` construction: `Page::new` leaves the slug empty and `generate_slug` fills it in later. The decision on Open question 3 applies.
  - ARCH-2 removes the free-text collision `source` labels. The decision on Open question 4 applies.
  - ARCH-4 (structured error variants) owns error texts in general. This change adds exactly one new message (AC-8.1, Open question 5).
  - SPEC-3 rewrites legacy AC tags. This change only carries existing tags along.

## Current behavior

### REQ-1 [baseline] A page's slug comes from its file path, and file names are validated strictly
Evidence: `src/content/page.rs:48-74`, `src/content/loader.rs:74-75`; tests `generate_slug_strips_site_prefix_and_extension`, `generate_slug_fails_when_path_is_outside_site`, `generate_slug_accepts_safe_names_and_rejects_others` (`src/content/page.rs`), `build_fails_on_invalid_file_name_naming_file` (`tests/build.rs`); `README.md:68`.
Status: inferred, confirm at approval
- AC-1.1 [baseline] WHEN a markdown file is loaded THE content loader SHALL set the page's slug to the file's path relative to the site folder, with its final extension removed and path separators written as `/`. For example, `site/posts/post_one.md` gives `posts/post_one`.
- AC-1.2 [baseline] IF any `/`-separated segment of the slug is empty or contains a character other than ASCII letters, digits, `-`, `_` or `.` THEN THE content loader SHALL fail the build with a `General` error reading `<file path>: invalid file name '<segment>': use only ASCII letters, digits, '-', '_' and '.'`. This applies to draft pages too.
- AC-1.3 [baseline] WHEN a file has both a frontmatter, date or tag error and an invalid file name THE content loader SHALL report the frontmatter, date or tag error. This is because the page is built from its frontmatter before its slug is derived.
- AC-1.4 [baseline] THE content loader SHALL accept a segment made only of dots. For example, a file named `...md` has stem `..` and gets the slug `..`, and `posts/...md` gets `posts/..`. This behavior is replaced by REQ-8 (Open question 1, accepted).
- AC-1.5 [baseline] WHERE a file name contains a backslash on a platform where backslash is not a path separator THE content loader SHALL treat the backslash as `/` rather than reject it. So a Unix file literally named `a\b.md` gets the slug `a/b`. This change keeps the behavior (AC-7.7) and records it as a backlog risk (AC-9.4). See Open question 2. **No longer current behavior as of 2026-09-25: superseded by specs/risk-6/requirements.md.**
- AC-1.6 [baseline] IF a file name is not valid UTF-8 THEN THE content loader SHALL replace the invalid bytes with U+FFFD and reject the resulting segment under AC-1.2.
- AC-1.7 [baseline] IF the file path is not under the site folder THEN THE content loader SHALL fail with `General("unable to generate slug from path")`. The CLI cannot reach this case.

### REQ-2 [baseline] What a slug determines
Evidence: `src/content/page.rs:76-89`, `src/build/output.rs:133-144`, `src/build/index/section.rs:24-57,73-75`, `src/build/generate/home.rs:29-34`, `src/render/template.rs:177-278`, `src/content/summary.rs:15-24`, `src/build/generate/feed.rs:36`, `src/build/generate/sitemap.rs:23`; tests `slug_url_is_root_relative_with_trailing_slash`, `tag_slug_is_under_tags`, `extract_parent_slug_uses_last_separator`, `ancestors_get_section_entries`, `top_level_pages_are_not_indexed`, `fixture_nested_sections_and_top_level_page`, `fixture_feed_and_sitemap`, `build_fails_on_tags_folder_collision`, `build_fails_on_top_level_tags_page_collision`.
Status: inferred, confirm at approval
- AC-2.1 [baseline] THE build SHALL give each page, section, tag index and tag page the root-relative URL `/<slug>/`, and SHALL give the home page (the empty slug) the URL `/`.
- AC-2.2 [baseline] THE build SHALL write each slug's output to `<output folder>/<slug segments>/index.html`, and the home page to `<output folder>/index.html`.
- AC-2.3 [baseline] THE section index SHALL treat everything before a slug's last `/` as its parent section. It SHALL create a section for every ancestor of every page (`a/b/c` gives `a/b` and `a`). It SHALL place a slug with no `/` (a top-level page) in no section. The root is never a section.
- AC-2.4 [baseline] THE home page SHALL list as `home.sections` exactly the sections whose slug contains no `/`.
- AC-2.5 [baseline] THE build SHALL place the tag index at slug `tags` and each tag page at slug `tags/<tag>`. As a result, a `tags/` content folder (`section index 'tags'`) or a top-level `tags.md` (`page 'tags'`) collides with `tag index`.
- AC-2.6 [baseline] THE build SHALL label outputs in collision errors as `page '<slug>'`, `section index '<slug>'`, `home page`, `tag index` and `tag page '<tag>'`.
- AC-2.7 [baseline] WHILE `base_url` is set THE feed SHALL use `<base_url without trailing slashes><url>` for each item's `link` and `guid`, and THE sitemap SHALL use the same form for each `loc`.

### REQ-3 [baseline] Listing order compares slugs and tags as plain strings
Evidence: `src/build/index/section.rs:18-22,59-71` (`BTreeMap<String, _>`, `compare_summaries`), `src/build/index/tag.rs:11-28`, `src/build/generate/tag.rs:11-24`, `src/build/generate/sitemap.rs` (sorted by `loc`); tests `sections_iterate_in_slug_order`, `subsections_are_direct_children_sorted`, `ties_broken_by_title_then_slug`, `render_items_in_section_slug_order`, `fixture_orders_listings_and_caps_recent`, `fixture_tag_index_counts_and_dedup`.
Status: inferred, confirm at approval
- AC-3.1 [baseline] THE build SHALL order sections by byte-wise comparison of the full `/`-joined slug text. This covers their generation order, `section.subsections` and `home.sections`. Because `-` (0x2D) and `.` (0x2E) sort before `/` (0x2F), section `a-c` sorts before `a/b`. A segment-by-segment comparison would give a different order.
- AC-3.2 [baseline] WHEN two listed pages have the same date state and the same title THE build SHALL break the tie by byte-wise comparison of their slugs.
- AC-3.3 [baseline] THE build SHALL order the tag index entries and the tag pages by byte-wise comparison of the tag name.

### REQ-4 [baseline] Tags are validated, de-duplicated and exposed to templates as strings
Evidence: `src/content/page.rs:91-118`, `src/render/template.rs:29-61,84-96`; tests `validate_tags_accepts_valid_values`, `validate_tags_rejects_invalid_values_naming_value`, `validate_tags_removes_duplicates_keeping_order`, `new_rejects_invalid_tag` (`src/content/page.rs`), `build_fails_on_invalid_tag_naming_file_and_value`, `build_fails_on_invalid_tag_in_draft`, `fixture_tag_index_counts_and_dedup` (`tests/build.rs`).
Status: inferred, confirm at approval
- AC-4.1 [baseline] IF a frontmatter tag does not match `^[a-z0-9]+(-[a-z0-9]+)*$` THEN THE page loader SHALL fail with a `Frontmatter` error reading `<file path>: invalid tag '<tag>': expected lowercase ASCII letters and digits separated by single hyphens`. This applies to draft pages too.
- AC-4.2 [baseline] WHEN a page lists the same tag more than once THE page loader SHALL keep only the first occurrence, in its original position.
- AC-4.3 [baseline] THE template contexts SHALL expose slugs and tags as plain strings:
  - `page.tags[]` is `{ name, url }`, with `url` = `/tags/<name>/`
  - `section.slug` and `section.subsections[].slug`
  - `home.sections[].slug`
  - `slug` on every page summary
  - `tags[]` is `{ name, url, page_count }`
  - `tag.name` and `tag.url`

## User stories
- As a maintainer, I want every slug and tag in the build to have passed validation, so that no module has to trust that another module checked it.
- As a maintainer, I want the rules for a slug's URL, output location, parent and top-level status in one place, so that changing a rule means changing one module.
- As the author of ARCH-2, I want typed slugs and tags to exist, so that the unified output model can carry them instead of free-text strings.
- As a site author, I want my site to build to exactly the same bytes as before, so that this refactor is invisible to me.
- As a site author, I want a file name that would write output outside its intended location to be rejected with an error that tells me why, so that one page can never silently overwrite another and I know how to fix the name.

## Requirements

### REQ-5 A slug is a value that only exists after validation
Every place the build names an output location (page, section index, home page, tag index, tag page) uses a slug value. The only ways to get one are the file-name validation of REQ-1 and REQ-8, or a derivation from a slug or tag that is already valid.
- AC-5.1 THE slug type SHALL be obtainable only by one of these:
  - validating a content file's path under AC-1.1, AC-1.2 and AC-8.1
  - taking the parent of an existing slug
  - asking a tag for its page slug
  - the fixed home and tag-index locations

  No constructor SHALL skip validation, including one used only by tests. A test helper MAY build a slug from `/`-joined text only if it applies the same segment rules as AC-1.2 and AC-8.1 and fails on invalid input. Existing unit tests that assign slug text directly (for example `page.slug = slug.to_string()` in the test helpers of `src/build/index/section.rs`, `src/build/generate/*.rs` and `src/render/template.rs`) SHALL go through such a validating path. This criterion is verified by reviewing every constructor of the slug type, test-only ones included, and confirming that each one validates or derives from an already-valid slug or tag.
- AC-5.2 [changed] IF slug validation is given text containing an empty segment or a segment with a character outside ASCII letters, digits, `-`, `_` and `.` THEN THE slug type SHALL refuse it with the error of AC-1.2. An empty segment always gets the AC-1.2 error, never the AC-8.1 error. IF it is given text containing a non-empty segment made only of `.` characters THEN THE slug type SHALL refuse it with the error of AC-8.1. Both errors name the file and the segment.
  Previously: all three cases were refused with the error of AC-1.2 (gate 1). At gate 3 the dot-only case got the AC-8.1 error, but the text did not say that a dot-only segment must be non-empty.
- AC-5.3 THE slug type SHALL provide, for any slug:
  - its root-relative URL, as in AC-2.1
  - its output file path under a given output folder, as in AC-2.2
  - its parent section, as in AC-2.3, or none for a top-level slug or the home slug
  - whether it is top-level, as in AC-2.4
  - its `/`-separated segments
- AC-5.4 THE slug type SHALL order slugs by byte-wise comparison of their `/`-joined text (AC-3.1, AC-3.2). A unit test SHALL pin that `a-c` sorts before `a/b`.
- AC-5.5 THE slug type SHALL serialize to template contexts as the same `/`-joined string that is exposed today (AC-4.3), and SHALL display as that string in error messages.
- AC-5.6 THE slug type SHALL be the only code in the crate that splits a slug into segments, joins segments into a slug, or builds a URL or output path from a slug. At minimum, these call sites SHALL no longer handle slug text themselves:
  - `get_final_path`
  - `extract_parent_slug`
  - the `!slug.contains('/')` test
  - every `slug_url` and `tag_slug` call outside the slug and tag implementation, including `summary.rs:21`, `feed.rs:36` and `sitemap.rs:23`
  - the literal `""` and `"tags"` slugs

  Error messages and collision labels MAY print a slug's or tag's display form (Open question 4, decided). This criterion is verified by reviewing the design's list of replaced call sites, backed by the tests of REQ-7.
- AC-5.7 [changed] THE page model SHALL never hold unvalidated slug text, at any point in a page's life. Whenever a page has a slug, that slug is a value of the slug type. Whether a page receives its slug when it is constructed or later is left to the design (Open question 3, decided).
  Previously: WHERE the design keeps two-phase `Page` construction THE page model SHALL hold a slug value (for example a placeholder) rather than unvalidated text until the page's path is known. The design MAY instead give each `Page` its slug when it is constructed (Open question 3, decided). It is not required to.

### REQ-6 A tag is a value that only exists after validation
- AC-6.1 THE tag type SHALL be obtainable only through the tag validation of AC-4.1. No constructor SHALL skip validation, including one used only by tests. A test helper MAY build a tag from text only by running that validation. This criterion is verified by reviewing every constructor of the tag type, test-only ones included.
- AC-6.2 IF tag validation is given text that does not match `^[a-z0-9]+(-[a-z0-9]+)*$` THEN THE tag type SHALL refuse it with the error of AC-4.1, naming the value.
- AC-6.3 THE page model SHALL hold its tags as tag values, de-duplicated and in order as in AC-4.2.
- AC-6.4 THE tag type SHALL provide its page slug (`tags/<tag>`, AC-2.5) and its URL (`/tags/<tag>/`). THE tag index and the tag pages SHALL take these from the tag rather than formatting them.
- AC-6.5 THE tag type SHALL order tags by byte-wise comparison of their text (AC-3.3), and SHALL serialize and display as the plain tag string (AC-4.3).

### REQ-7 Nothing else changes for site authors
The only change a site author can see is REQ-8.
- AC-7.1 THE `mango build` command SHALL produce byte-identical output for the fixture site, which contains no dot-only segment. `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `fixture_internal_links_resolve` and every other `fixture_*` test SHALL pass without edits to their expectations.
- AC-7.2 THE `mango build` command SHALL keep every existing error text, error variant, exit status and failure order (AC-1.2, AC-1.3, AC-2.6, AC-4.1). Every existing E2E test in `tests/build.rs` and every in-process test in `tests/plan.rs` SHALL pass with no change to an expected string, path or status. The only new error text is the one of AC-8.1.
- AC-7.3 WHEN a unit test targets a function this change removes (for example `extract_parent_slug_uses_last_separator`, `slug_url_is_root_relative_with_trailing_slash`, `tag_slug_is_under_tags`) THE test suite SHALL keep an equivalent assertion against the new types, and the rewritten test SHALL carry the original `// AC-…` tag. No legacy criterion in `specs/_system/legacy-criteria/` SHALL lose its last verifying test.
- AC-7.4 [changed] THE test suite SHALL include unit tests, tagged `// AC-arch-3.<req>.<m>`, showing that the type's own validation refuses each invalid slug input from AC-5.2 and each invalid tag input from AC-6.2. For slugs, these tests SHALL check which message each input gets: an empty segment (for example `a//b` or empty text) gets the AC-1.2 message, and a dot-only segment (for example `..`) gets the AC-8.1 message.
  Previously: the tests had to show that each invalid slug input was refused, but did not have to check which message an empty segment and a dot-only segment each get.
- AC-7.5 THE library's public surface SHALL stay exactly the seven re-exports (`plan`, `commit`, `clean`, `BuildOptions`, `BuildPlan`, `PlannedOutput`, `MangoError`). The new types SHALL be crate-private, and no `#[allow(dead_code)]` SHALL be added.
- AC-7.6 THE change SHALL add no dependency, including dev-dependencies, and SHALL keep the gate green (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`).
- AC-7.7 THE content loader SHALL keep the backslash-to-`/` behavior of AC-1.5 unchanged. A unit test tagged `// AC-arch-3.7.7` SHALL pin that a path whose stem contains `\` (for example `a\b`) gets the slug `a/b`. **Withdrawn 2026-09-25: superseded by specs/risk-6/requirements.md.**

### REQ-8 Dot-only file-name segments are rejected
Accepted at the requirements gate (Open question 1). A segment such as `..` in a slug can put a page's output outside its intended location, where it silently overwrites another page. With `posts/...md`, the overwritten page is the home page's `index.html`. This requirement replaces AC-1.4. The error states the actual reason rather than reusing the AC-1.2 text, which lists `.` as allowed (Open question 5, decided).
- AC-8.1 [changed] IF a slug segment is non-empty and consists only of `.` characters (`.`, `..`, or a longer run of dots) THEN THE content loader SHALL fail the build with a `General` error reading `<file path>: invalid file name '<segment>': a segment cannot consist only of dots`. An empty segment is not dot-only and always gets the AC-1.2 error. The rejection covers every dot-only segment in the same way, not only the ones that can escape the output folder or silently collide with another output. This is a deliberate simplification. This applies to draft pages too, and happens before the output folder is cleaned. The AC-1.2 message for other invalid names is unchanged.
  Previously: IF a slug segment consists only of `.` characters THEN THE content loader SHALL fail the build with the AC-1.2 error, naming the file and the segment (gate 1). At gate 3 it had its own message, but it did not say that the segment must be non-empty or that the rejection is deliberately uniform.
  **No longer applies to files found by the content loader since 2026-09-26 (`specs/risk-10/requirements.md`):** every dot-only name starts with `.`, so the loader skips it as hidden before any file-name check. The slug type still rejects dot-only segments (AC-5.2, AC-7.4).
- AC-8.2 [changed] THE test suite SHALL include a temp-site E2E test in `tests/build.rs` in which `site/...md` fails with exit status 1 and a stderr that names the file, contains `'..'` and contains `a segment cannot consist only of dots`, and in which the previous output is left intact (a `snapshot` test). It SHALL also include a unit test for `.` and `..` segments, both top-level and nested (for example `posts/..`), that checks the AC-8.1 message. The same unit tests SHALL check that an empty segment gets the AC-1.2 message and not the AC-8.1 message.
  Previously: the E2E stderr assertion required only the file and `'..'`, and the unit test did not check the message (gate 1). At gate 3 the unit test did not have to cover the empty segment.
  **No longer applies to files found by the content loader since 2026-09-26 (`specs/risk-10/requirements.md`):** every dot-only name starts with `.`, so the loader skips it as hidden before any file-name check. The slug type still rejects dot-only segments (AC-5.2, AC-7.4). Its E2E test was replaced by `build_skips_dot_only_file_name`, which checks that `...md` and `..md` are skipped.
- AC-8.3 THE `README.md` file-name rule (`README.md:68`) and the `CLAUDE.md` "File names are strict" paragraph SHALL state that a segment made only of dots is rejected.

### REQ-9 Developer documentation and backlog follow the code
- AC-9.1 THE `CLAUDE.md` module map and rules SHALL describe the slug and tag types and where they live. They SHALL no longer name removed functions (`generate_slug`, `slug_url`, `tag_slug`, `validate_tags`, `get_final_path`, `extract_parent_slug`) as current code.
- AC-9.2 THE backlog SHALL mark ARCH-3 `done` in the squash commit that lands it, with `(ARCH-3)` in the subject. The notes on ARCH-2 and ARCH-6 SHALL still read correctly afterwards.
- AC-9.3 THE `README.md` SHALL change only as AC-8.3 requires.
- AC-9.4 THE backlog SHALL gain a new open RISK item in the same squash commit. It SHALL use the next free number (RISK-6 at the time of writing), be listed in the summary table and have its own section. It SHALL describe the backslash-to-separator quirk of AC-1.5: on platforms where `\` is not a path separator, a file named `a\b.md` is published at `/a/b/` and creates a section `a`, even though the file-name rule does not allow `\`. It SHALL propose rejecting it or documenting it.

## Out of scope
- The unified output model, and replacing collision `source` labels and `page_date` (ARCH-2).
- Structured error variants (ARCH-4). Error texts stay as they are, apart from the one new message of AC-8.1.
- Removing `PageType` and moving `compare_summaries` (ARCH-6). One-step `Page` construction is also ARCH-6's, but the design may adopt it here (AC-5.7). ARCH-6 keeps the item either way.
- Rewriting legacy AC tags (SPEC-3).
- Changing the backslash-to-separator behavior (AC-1.5). It is kept here and tracked by the new RISK item (AC-9.4).
- Any change to URLs, output paths, listing order, template context shape, feed or sitemap format, or the fixture site. The one exception is the rejection of dot-only segments (REQ-8).
- Making slugs or tags part of the library's public API.

## Open questions
All five questions were decided at the requirements gate on 2026-09-23. Questions 1-4 were decided with the reply "approved with recommendations", and question 5 with the reply "add the should-fixes and the nit in as well". They are kept here, with the decisions, so later stages can read them.

1. **Dot-only file names write outside their intended location. Decided: ACCEPTED, REQ-8 is a firm requirement.** `is_valid_slug_segment` (`src/content/page.rs:69-74`) accepts segments made only of dots. A file `site/...md` has stem `..`, so it gets the slug `..`, the URL `/../` and the output path `<output>/../index.html`, which is outside the output folder. `site/posts/...md` gets `<output>/posts/../index.html`. That is the home page's file, but the collision check compares paths without resolving `..`, so the later write silently overwrites the home page. By contrast, `..md` (slug `.`) collides as expected, because `Path` equality ignores non-leading `.` components. The spec critic confirmed this reasoning independently from `std::path` semantics. It has not been reproduced with a running build, and the E2E test of AC-8.2 will do that.
2. **Backslashes in Unix file names become folder separators (AC-1.5). Decided: preserve in this change (AC-7.7) and open a new RISK item in the backlog as part of this change (AC-9.4).**
3. **Overlap with ARCH-6 on `Page` construction. Decided: the design MAY adopt one-step `Page` construction, but is not required to. Either way a page never holds unvalidated slug text (AC-5.7). ARCH-6 keeps the item.**
4. **Can collision labels and error messages include a slug? Decided: yes. Printing a slug's or tag's display form inside error messages and collision labels is allowed (AC-5.6).** ARCH-2 later replaces the labels.
5. **Error wording for a dot-only segment. Decided: use its own message.** Reusing the AC-1.2 text (`use only ASCII letters, digits, '-', '_' and '.'`) would contradict itself for a segment made only of the allowed `.`. The error is instead `<file path>: invalid file name '<segment>': a segment cannot consist only of dots`, with the same `General` variant as AC-1.2 (AC-8.1, AC-5.2, AC-8.2). The AC-1.2 message for other invalid names is unchanged.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-23 | 20260923-220004-arch-3-slug-tag-newtypes | New spec from backlog ARCH-3. It records current slug and tag behavior, and requires validated slug and tag types that own all slug and tag-URL logic, crate-private, with site-author output otherwise unchanged. Approver decisions at the requirements gate: Q1 accepted, so dot-only file-name segments are rejected (REQ-8). Q2: backslash-to-separator is kept (AC-7.7) and a new RISK item is opened (AC-9.4). Q3: one-step `Page` construction is allowed but not required, and AC-5.7 is stated as an observable property (a page never holds unvalidated slug text). Q4: slug and tag display forms may appear in errors and collision labels. Q5: dot-only segments get their own error message, `a segment cannot consist only of dots` (AC-8.1, AC-5.2, AC-8.2). Critic's should-fix items applied: constructor review as the verification method (AC-5.1, AC-6.1), and decisions recorded in the document. |
| 2026-09-23 | 20260923-220004-arch-3-slug-tag-newtypes | Third-gate wording fixes, with no change to what gets built. A dot-only segment is defined as non-empty, and an empty segment always gets the AC-1.2 error (AC-5.2, AC-8.1). AC-8.1 now says that rejecting every dot-only segment in the same way is a deliberate simplification, and its list of examples reads "`.`, `..`, or a longer run of dots". The tests in AC-7.4 and AC-8.2 must check that an empty segment and a dot-only segment each get their own message. |
| 2026-09-26 | 20260926-163650-risk-10-skip-hidden-entries | Noted under AC-8.1 and AC-8.2 that they no longer apply to files found by the content loader: dot-only names start with `.` and are skipped as hidden (`specs/risk-10/requirements.md`, AC-3.5). The slug type's own dot-only check and its unit tests are unchanged. The original wording is kept. |
