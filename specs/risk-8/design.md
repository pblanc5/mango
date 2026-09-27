<!-- Published by dev-pipeline run 20260927-191208-risk-8-sort-pages-by-slug, stage design v2, approved 2026-09-27T19:47:34Z. -->

# Content page order does not depend on the filesystem: Design

Spec ID: `risk-8` · Requirements: `specs/risk-8/requirements.md`

## Overview
The change is two small edits in `src/content/loader.rs`:
1. `traverse` reads each folder's entries into a `Vec`, sorts them byte-wise by file name, then processes them as it does today, descending into a subfolder where it meets it. The first failing entry is then the first by path, compared segment by segment (REQ-4).
2. `load` sorts the non-draft pages by `Slug` before returning them (REQ-2).

Everything downstream already follows the loader's order: plan order, write order, render-error choice and collision choice (REQ-3). No other production code changes. Output bytes stay the same because every listing is already sorted. Tests pin the new orders with names chosen so that neither creation order, its reverse nor per-folder name order gives the expected result. The changelog, README, `CLAUDE.md`, the system overview, the arch-2 requirements and the backlog are updated.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/loader.rs` | `traverse`: collect the `read_dir` entries, sort them by `DirEntry::file_name()`, then loop as today. `load`: sort the pages by `slug` after dropping drafts. Doc comments say what the order is. New unit tests. |
| `src/build/pipeline.rs` | Comments only: the "one list" comment in `plan` and the `BuildPlan::outputs` doc say that pages come in slug order. No code change. |
| `tests/plan.rs` | New in-process tests: safety net for a duplicate slug, plan enumeration in slug order, the render error for the first slug, and the collision for the first slug. |
| `tests/build.rs` | New E2E test: with two bad content files, the first by path is reported and the previous output is kept. |
| `CHANGELOG.md` | `### Changed` line under `## [Unreleased]`, not breaking. |
| `README.md` | Safe builds: the build stops at the first error, and among bad content files the first by path is reported. |
| `CLAUDE.md` | Pipeline and module map: pages are in slug order, and load errors are chosen by path. |
| `specs/_system/overview.md` | `traverse` risk row: RISK-8 no longer open, new order described. Determinism row: lists the loader sorts. |
| `specs/arch-2/requirements.md` | AC-2.4 marked superseded by `specs/risk-8/requirements.md` (wording kept), plus a Changelog row. |
| `specs/_system/backlog.md` | RISK-8 `done` in the index row and in its section, with a resolution note. |

## Approach
**Why two sorts.** The spec asks for two different orders (Open question 2, accepted at approval):
- Pages are ordered by slug text (AC-2.1). Here `a-c` comes before `a/b`, and `m` before `m-n`.
- Load errors are ordered by path, segment by segment (AC-4.1). Here the folder `a` comes before `a-c.md`.

Sorting the traversal alone would give name order, not slug order: `a/b` would come before `a-c`, and `m-n.md` before `m.md` because `-` (0x2D) is lower than `.` (0x2E). Sorting the pages alone would not change which load error is reported, because the loader stops at the first failure. So both sorts are needed. Each is one line in the one module that owns the walk.

**Traversal order (REQ-4).**
```rust
let mut entries = fs::read_dir(dir)
    .map_err(|e| MangoError::io_at(dir, e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| MangoError::io_at(dir, e))?;
entries.sort_by_key(|entry| entry.file_name());
for entry in entries { /* body unchanged: hidden check first, then metadata, … */ }
```
- **Byte-wise order.** `DirEntry::file_name()` returns an `OsString` (std `fs.rs:2521`). `Ord for OsString` delegates to `OsStr` (std `ffi/os_str.rs:796-800`), which compares `as_encoded_bytes()` (`os_str.rs:1548-1552`). That encoding is "a self-synchronizing superset of UTF-8 … also a superset of 7-bit ASCII" (`os_str.rs:1055-1057`).
  - For ASCII names, which covers every valid file name, this is exactly byte-wise name order on every platform.
  - For other names, the order is fixed for a given platform and Rust version (AC-4.6).
  - Names within one folder are distinct, so the sort has no ties and stability does not matter.
- **Where a subfolder's contents go.** The loop descends into a subfolder at its sorted position (`traverse(site, path, pages)?` as today), so the subfolder's contents take its place. A name that is a prefix of another sorts first (slice `Ord`), so the folder `a` comes before `a-c.md` and `a.md`.
- **Per-entry logic is unchanged.** The body of the loop stays as it is: the hidden check, `fs::metadata`, the symlinked-folder check, reading, frontmatter and `Page::new`. So each file's own error precedence stays the same (AC-4.4), and hidden entries are still skipped before being resolved (AC-4.5).
- **A folder that cannot be read** fails at `read_dir(dir)` or while its entries are collected. That happens when the walk reaches the folder, which is the folder's own position (AC-4.3). One side effect: an iteration error inside a folder is now reported before any entry of that folder is checked, whereas before, entries listed earlier were checked first. This is what AC-4.3 asks for.
- **Why the order is independent of listing order.** `std::fs::read_dir` documents that "the order in which this iterator returns entries is platform and filesystem dependent" (std `fs.rs:3137-3138`). That is the root cause, and the sort removes it.

**Page order (REQ-2).**
```rust
let mut pages: Vec<Page> = pages.into_iter().filter(|page| !page.draft).collect();
pages.sort_by(|a, b| a.slug.cmp(&b.slug));
Ok(pages)
```
- `Slug` derives `Ord` on its `/`-joined text (`src/content/slug.rs:18-20`), pinned by `orders_bytewise_on_joined_text` (`slug.rs:397-402`). This is the order section indexes use (AC-2.1).
- Two pages have the same slug only when two files in one folder differ only by extension (`a.md`, `a.markdown`). Traversal already meets them in name order, and their collision message is identical whichever comes first (AC-2.4). So a stable or unstable sort gives the same result.
- The same `sort_by(|a, b| a.slug.cmp(&b.slug))` pattern already passes `clippy --all-targets -D warnings` in `loader.rs:252`.

**Downstream (REQ-3, AC-5.3).** Nothing else changes:
- `content::build(&pages, …)` emits page outputs in page order (`src/build/generate/content.rs:6-11`), first in the list (`pipeline.rs:127`).
- `check_collisions` reports the first collision in list order (`output.rs:141-180`), and `render` the first render error in list order (`output.rs:185-202`). Both now follow slug order among pages.
- The stage order in `plan` is untouched (AC-3.3).
- Sections, tags, home, feed and sitemap sort on their own (`index/section.rs:47`, `index/tag.rs:24`, `generate/home.rs:19`, `generate/sitemap.rs:30`), so their contents and the output bytes are unchanged (AC-1.6, AC-5.1, AC-5.3).
- `assets.rs`, `lib.rs` and `Cargo.toml` are not touched (AC-5.4 to AC-5.6).

## Interfaces and data
None. `loader::load(&Path) -> Result<Vec<Page>, MangoError>` keeps its signature. Only its documented order changes, from filesystem order to slug order. The public surface (seven re-exports, `PlannedOutput`) is unchanged. There are no new error variants or messages.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Sort only the pages in `load` | Does not change which load error is reported, because the loader stops at the first failing entry in filesystem order. Fails REQ-4. |
| Sort only the traversal by name | Pages would come in name order, not slug order (`a/b` before `a-c`, `m-n` before `m`). Fails AC-2.1. |
| Sort in `pipeline::plan` instead of the loader | AC-2.1 puts the order on the loader. It would also leave other loader callers (the future dev server, tests) with filesystem order. |
| Collect every entry recursively, then sort by the full relative path text (which would match slug order for errors too) | Rejected by the spec (Open question 2): errors are ordered segment by segment. It would also mean restructuring the walk, and a failing entry may have no valid slug. |
| Check every entry and report the minimum error | Changes the model from stopping at the first failure. AC-4.2 keeps stopping at the first failure, and the build would do more work. |
| `walkdir` with `sort_by_file_name` | A new dependency (AC-5.6). Its symlink and hidden-entry handling would also have to be re-proven against RISK-1, RISK-5 and RISK-10. |
| Sort by `entry.path()` instead of `file_name()` | Same order within one folder, but it compares the whole parent prefix every time. `file_name` says what is meant. |

## Risks
- **New tests could pass without the change by chance on Linux.** Filesystem listing order is unspecified (std `fs.rs:3137-3138`), so a test that should fail without the change could pass on a filesystem whose order happens to match. Mitigations:
  - Each new order test uses six to eight entries.
  - Files are created in an order that is neither the expected order nor its reverse.
  - Names are chosen so that name order, whether byte-wise or case-insensitive, differs from the expected order: `Z`/`_x` against lowercase letters, `m` against `m-n`, and the folder `a` against `a-c.md`.
  - The Developer runs the new tests against the unchanged loader on Linux before implementing (T-2 to T-4) and records which fail (Open question 5). The E2E test has exactly two bad files, as AC-6.5 requires, so on a hash-ordered filesystem it fails without the change only about half the time. The unit tests carry the proof for REQ-4.
- **Case-insensitive filesystems (Windows, and macOS by default).** No test creates two files whose names differ only by case in the same folder, so no test file overwrites another on NTFS.
- **Read errors while collecting a folder's entries** are now reported before any entry of that folder is checked (see Approach). This matches AC-4.3 and changes no error text.
- **Memory.** Each folder's entries are held in memory while it is walked. This is negligible for a content folder.

**Unverified assumptions:** None that the design relies on. The external claims it relies on are all backed by source:
- `OsString` ordering: std `os_str.rs:796-800, 1548-1552, 1055-1057`.
- `DirEntry::file_name` returning `OsString`: std `fs.rs:2521`.
- Unspecified `read_dir` order: std `fs.rs:3137-3138`.
- Tera `throw` for the render test: tera 1.20.1, see Test strategy.

One claim is **not verified and does not affect correctness**: that NTFS lists names in case-insensitive (uppercase) collation order. If true, T-2 to T-4 also fail without the change on Windows CI. It only decides on which CI platform the new tests are *shown* to fail without the change. No test or production code depends on it, and it cannot be checked by a test run against the new code, so it gets no safety-net task. The Linux result recorded in T-2 to T-4 is the evidence REQ-6 asks for.

## Test strategy
- **Suite command:** `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- **Safety net:**
  - **AC-1.6** (duplicate-slug collision text, `both page 'a' and page 'a'`) has no existing test: nothing in `src/` or `tests/` builds `a.md` next to `a.markdown`. **T-1** pins it before any change. It passes before and after, and is also the AC-2.4 test.
  - **Other baseline behavior of `traverse` that the change must keep** is already covered and must stay green unchanged:
    - Each load failure kind: `subdirectory_page_without_frontmatter_is_an_error_naming_the_file`, `subdirectory_malformed_frontmatter_is_an_error_naming_the_file`, `non_object_frontmatter_fails_the_load_even_for_drafts`, `unknown_key_in_draft_is_still_an_error`, `invalid_date_in_draft_is_still_an_error`, `invalid_file_name_is_an_error_naming_the_file_even_for_drafts`, `load_rejects_symlinked_directory`, `load_fails_on_dangling_symlink`, `load_fails_on_symlink_loop`.
    - Per-file precedence (AC-4.4): `frontmatter_error_wins_over_invalid_file_name`, `key_errors_win_over_date_tag_and_file_name_errors`, `non_object_frontmatter_wins_over_invalid_file_name`, `frontmatter_error_wins_over_backslash_in_name`.
    - Hidden entries (AC-4.5): `load_skips_hidden_files_and_folders`, `hidden_entries_are_not_checked`, `hidden_symlinks_are_not_resolved`.
    - Output independence (AC-1.6, AC-5.1): `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `planning_twice_gives_identical_outputs`.
  - **AC-1.1 to AC-1.5** are replaced by REQ-2 to REQ-4, not kept, so they get no safety net.
- **New and changed behavior:** the tests below. Every existing test in `tests/build.rs` and `tests/plan.rs` must pass with no edit (AC-6.6, AC-5.1).

| Test | File | Covers |
|---|---|---|
| `load_returns_pages_in_slug_order` | `src/content/loader.rs` | AC-2.1, AC-2.3, AC-6.1, AC-6.7 |
| `load_reports_first_bad_entry_by_name_in_a_folder` | `src/content/loader.rs` | AC-4.1, AC-4.2, AC-4.5, AC-6.4 |
| `load_reports_bad_entry_in_subfolder_at_the_subfolder_position` | `src/content/loader.rs` | AC-4.1, AC-6.4 |
| `load_reports_symlinked_folder_before_later_bad_entries` (unix) | `src/content/loader.rs` | AC-4.1, AC-4.3 |
| `plan_lists_content_pages_in_slug_order` | `tests/plan.rs` | AC-2.2, AC-2.3, AC-5.3, AC-6.2, AC-6.7 |
| `first_render_error_among_pages_is_for_the_first_slug` | `tests/plan.rs` | AC-3.1, AC-6.3 |
| `first_page_collision_is_for_the_first_slug` | `tests/plan.rs` | AC-3.2, AC-2.4 |
| `build_reports_first_bad_content_file_by_path_keeping_output` | `tests/build.rs` | AC-6.5, AC-4.1, AC-5.2 |

- **Tera `throw` (for AC-6.3).** In tera 1.20.1 (`Cargo.lock:782-783`):
  - `throw` is registered unconditionally (`tera-1.20.1/src/tera.rs:751`, not behind the `builtins` feature).
  - It fails with `Error::msg(<message>)` when `message` is a string (`src/builtins/functions.rs:128-139`, test `throw_errors_with_message` at `:278-286`).
  - The renderer wraps a function error as `Error::call_function(name, e)`, keeping `e` as the source (`src/renderer/processor.rs:497`, `src/errors.rs:160-165`).
  - mango's `MangoError::Template` display joins the whole source chain (`src/error.rs:41-50`, test `template_display_includes_source_chain`).
  - So `{{ throw(message=page.title) }}` produces a `Template` error whose text contains that page's title.

## Tasks
### T-1 Safety net: duplicate slug from two extensions collides with a fixed message
- Kind: safety-net
- Satisfies: AC-1.6, AC-2.4
- Files: `tests/plan.rs`
- Steps: add `same_slug_from_two_extensions_collides`, tagged `// AC-risk-8.1.6, AC-risk-8.2.4`. Write `a.md` and `a.markdown` (both valid, not drafts) at the top of `project(...)`'s site, then assert with `assert_eq!` that `plan` fails with `MangoError::General` and exactly `Mango Error: output path '{}' would be written by both page 'a' and page 'a'`, where the path is `p.out.join("a").join("index.html").display()`.
- Done when: the test passes on the unchanged code.

### T-2 Tests: loader page order and load-error order (written before T-5)
- Kind: test
- Satisfies: AC-2.1, AC-2.3, AC-4.1, AC-4.2, AC-4.3, AC-4.5, AC-6.1, AC-6.4, AC-6.7
- Files: `src/content/loader.rs` (the `tests` module only)
- Steps: add four tests. Bad files contain `"# no frontmatter\n"`. Paths used in assertions are built one segment per `join`.
  - `load_returns_pages_in_slug_order`, tagged `// AC-risk-8.2.1, AC-risk-8.6.1, AC-risk-8.2.3`.
    - Create valid pages in this order: `b.md`, `m-n.md`, `a/b.md`, `Z.md`, `m.md`, `_x.md`, `a-c.md`, plus a draft `c.md`.
    - Assert that the slugs returned by `load(&site)`, **not sorted by the test**, equal `["Z", "_x", "a-c", "a/b", "b", "m", "m-n"]`.
  - `load_reports_first_bad_entry_by_name_in_a_folder`, tagged `// AC-risk-8.4.1, AC-risk-8.4.2, AC-risk-8.4.5, AC-risk-8.6.4`.
    - Create bad files in this order: `m.md`, `a.md`, `z.md`, `_x.md`, `m-n.md`, `k.md`, `Q.md`, `b.md`, plus a bad hidden `.first.md`.
    - Assert that the error is `Frontmatter` and that its text starts with `Mango Frontmatter Error: {site.join("Q.md")}: missing frontmatter`.
  - `load_reports_bad_entry_in_subfolder_at_the_subfolder_position`, tagged `// AC-risk-8.4.1, AC-risk-8.6.4`. Two layouts, each in a fresh fixture folder:
    - (a) Bad top-level `d.md`, `a-c.md`, `c.md`, `a_b.md`, `b.md`, `a.md`; a valid `a/y.md`; a bad `a/z.md`. Expect `site/a/z.md`. This is the AC-4.1 example of `a-c.md` against `a/b.md`.
    - (b) Bad top-level `c.md`, `a.md`, `d.md`, `b.md`; a bad `_d/z.md`. Expect `site/_d/z.md`.
  - `#[cfg(unix)] load_reports_symlinked_folder_before_later_bad_entries`, tagged `// AC-risk-8.4.1, AC-risk-8.4.3`.
    - Create bad `r.md`, `posts-old.md` and `postsa.md`, a dangling symlink `q.md`, and `posts` as a symlink to a real folder outside the site.
    - Expect `MangoError::General` containing `content folder 'posts'`.
- Done when:
  - The tests compile.
  - The Developer has run them against the **unchanged** loader on Linux and recorded in Pipeline notes which of them fail. Expected: at least `load_returns_pages_in_slug_order` fails, since no filesystem order gives slug order for these names.
  - After T-5, all four pass.

### T-3 Tests: plan order, render-error choice and collision choice (written before T-5)
- Kind: test
- Satisfies: AC-2.2, AC-2.3, AC-2.4, AC-3.1, AC-3.2, AC-5.3, AC-6.2, AC-6.3, AC-6.7
- Files: `tests/plan.rs`
- Steps:
  - `plan_lists_content_pages_in_slug_order`, tagged `// AC-risk-8.2.2, AC-risk-8.6.2, AC-risk-8.2.3`.
    - Create pages with `frontmatter(...)` in the T-2 creation order (`b.md`, `m-n.md`, `a/b.md`, `Z.md`, `m.md`, `_x.md`, `a-c.md`). No config.
    - Assert that `paths(&plan)` equals `Z/index.html`, `_x/index.html`, `a-c/index.html`, `a/b/index.html`, `b/index.html`, `m/index.html`, `m-n/index.html`, `a/index.html`, `index.html`, `tags/index.html`, `assets/style.css`.
  - `first_render_error_among_pages_is_for_the_first_slug`, tagged `// AC-risk-8.3.1, AC-risk-8.6.3`.
    - Overwrite `page.html` with `{{ throw(message=page.title) }}`.
    - Use the same seven files, created in the same order. Each title is `boom <slug with / replaced by ~> end`, so every title is unique and none is a substring of another: `boom b end`, `boom m-n end`, `boom a~b end` (for `a/b.md`), `boom Z end`, `boom m end`, `boom _x end`, `boom a-c end`.
    - Assert that the error is `MangoError::Template`, that its text `contains` `boom Z end`, and that it does not contain (`!contains`) any of the other six titles.
  - `first_page_collision_is_for_the_first_slug`, tagged `// AC-risk-8.3.2, AC-risk-8.2.4`.
    - Create `m-n.md`, `m-n.markdown`, `m.md`, `m.markdown`.
    - Assert with `assert_eq!` the exact text `Mango Error: output path '<p.out/m/index.html>' would be written by both page 'm' and page 'm'`, with the path built as `p.out.join("m").join("index.html")`.
- Done when:
  - The tests compile.
  - The Developer has recorded which of them fail against the unchanged loader on Linux. Expected: `plan_lists_content_pages_in_slug_order` fails.
  - After T-5, all three pass.

### T-4 E2E: two bad content files, first by path reported, output kept (written before T-5)
- Kind: test
- Satisfies: AC-4.1, AC-5.2, AC-6.5
- Files: `tests/build.rs`
- Steps: add `build_reports_first_bad_content_file_by_path_keeping_output`, tagged `// AC-risk-8.6.5, AC-risk-8.4.1, AC-risk-8.5.2`, following the pattern of `build_fails_on_array_frontmatter_keeping_output`.
  1. Write a valid `posts/one.md`, build with `build_temp_site`, and check `assert_success`.
  2. Write a marker file and take a `snapshot`.
  3. Add a bad `a.md`, then a bad `_d/z.md`, and build again.
  4. Assert `assert_failure`, status `Some(1)`, and that stderr contains `format!("{}: missing frontmatter", site.join("_d").join("z.md").display())` and does not contain `site.join("a.md").display()`.
  5. Assert that the snapshot is unchanged.
- Done when: the test passes after T-5. Its result against the unchanged code on Linux is recorded (with two files it may pass by chance there, see Risks).

### T-5 Sort the traversal by name and the pages by slug
- Kind: implementation
- Satisfies: AC-1.1, AC-1.2, AC-1.3, AC-1.4, AC-1.5, AC-2.1, AC-2.2, AC-2.3, AC-2.4, AC-3.1, AC-3.2, AC-3.3, AC-4.1, AC-4.2, AC-4.3, AC-4.4, AC-4.5, AC-4.6, AC-5.1, AC-5.2, AC-5.3, AC-5.4, AC-5.5, AC-5.6
- Files: `src/content/loader.rs`, `src/build/pipeline.rs` (comments only)
- Steps:
  - In `traverse`, collect the `read_dir` entries with every error mapped through `MangoError::io_at(dir, e)`, sort them with `sort_by_key(|entry| entry.file_name())`, and keep the loop body unchanged. Add a comment saying why: filesystem listing order is unspecified (RISK-8).
  - In `load`, sort the non-draft pages with `sort_by(|a, b| a.slug.cmp(&b.slug))`.
  - Update `load`'s doc comment: pages are returned in slug order; the error reported is the first failing entry by path, with each folder's entries in byte-wise name order and a subfolder's contents at the subfolder's place.
  - In `pipeline.rs`, reword the comment above `content::build` and the `BuildPlan::outputs` doc to say "pages (by slug)".
  - No other file is touched: not `assets.rs`, not `lib.rs`, not `Cargo.toml`.
- Done when:
  - T-1 to T-4 pass.
  - Every existing test passes unchanged, including `build_generates_site_from_fixture`, the `fixture_*` tests and `fixture_build_is_deterministic`.
  - The gate passes.

### T-6 User-facing docs: changelog and README
- Kind: docs
- Satisfies: AC-7.1, AC-7.2
- Files: `CHANGELOG.md`, `README.md`
- Steps:
  - `CHANGELOG.md`: under `## [Unreleased]` → `### Changed`, add one line that is not marked **Breaking**. Proposed wording: "When several content files have errors, the build now reports the first one by path (each folder's files and subfolders in name order), the same on every system, instead of whichever the filesystem happened to list first."
  - `README.md`, Safe builds: add that a build stops at the first error it finds, and that when several content files have errors, the one reported is the first by path. Name order is byte-wise, so `Z.md` comes before `a.md`, and the folder `a/` before `a-c.md`.
- Done when: both lines are present, proofread, and say nothing about output changes (there are none).

### T-7 Developer docs and system overview
- Kind: docs
- Satisfies: AC-7.3, AC-7.4
- Files: `CLAUDE.md`, `specs/_system/overview.md`
- Steps:
  - `CLAUDE.md`: in the plan description ("build **one** `Vec<Output>` in a fixed order: pages, …"), say that pages are in slug order. In the module map entry for `loader.rs`, say that each folder's entries are walked in byte-wise name order, so the first failing entry is the first by path, compared segment by segment, and that `load` returns pages sorted by slug (RISK-8).
  - `overview.md`, `traverse` risk row: replace the "One thing remains open … (RISK-8, open) …" sentences with the new order (RISK-8, done) and cite the new loader tests.
  - `overview.md`, Determinism row: add the two loader sorts to the list of sorts. Add a Changelog row dated 2026-09-27 for this run.
- Done when: no text still says that RISK-8 is open or that pages are in `read_dir` order.

### T-8 Spec cross-references and backlog
- Kind: docs
- Satisfies: AC-7.5, AC-7.6
- Files: `specs/arch-2/requirements.md`, `specs/_system/backlog.md`
- Steps:
  - arch-2: after AC-2.4's unchanged wording, add a note that it is superseded by `specs/risk-8/requirements.md` (REQ-2). Add a Changelog row dated 2026-09-27 for run `20260927-191208-risk-8-sort-pages-by-slug`.
  - Backlog: set RISK-8's index row and section header to `done`. End the section with "Resolved by `specs/risk-8/`: …", saying that pages are sorted by slug and the walk is sorted by name, that load errors are chosen by path, and that the asset planner's choice among several bad assets still follows listing order (out of scope).
  - Do not record a commit hash.
- Done when: RISK-8 reads `done` in both places. The `(RISK-8)` suffix in the pull request title is handled at landing, not in this task.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-5 (replaced by REQ-2), T-2 |
| AC-1.2 | T-5 (replaced by REQ-4), T-2 |
| AC-1.3 | T-5 (replaced by AC-2.2), T-3 |
| AC-1.4 | T-5 (replaced by AC-3.1), T-3 |
| AC-1.5 | T-5 (replaced by AC-3.2), T-3 |
| AC-1.6 | T-1, T-5 |
| AC-2.1 | T-2, T-5 |
| AC-2.2 | T-3, T-5 |
| AC-2.3 | T-2, T-3, T-5 |
| AC-2.4 | T-1, T-3, T-5 |
| AC-3.1 | T-3, T-5 |
| AC-3.2 | T-3, T-5 |
| AC-3.3 | T-5 |
| AC-4.1 | T-2, T-4, T-5 |
| AC-4.2 | T-2, T-5 |
| AC-4.3 | T-2, T-5 |
| AC-4.4 | T-5 |
| AC-4.5 | T-2, T-5 |
| AC-4.6 | T-5 |
| AC-5.1 | T-5 |
| AC-5.2 | T-4, T-5 |
| AC-5.3 | T-3, T-5 |
| AC-5.4 | T-5 |
| AC-5.5 | T-5 |
| AC-5.6 | T-5 |
| AC-6.1 | T-2 |
| AC-6.2 | T-3 |
| AC-6.3 | T-3 |
| AC-6.4 | T-2 |
| AC-6.5 | T-4 |
| AC-6.6 | T-5 |
| AC-6.7 | T-2, T-3 |
| AC-7.1 | T-6 |
| AC-7.2 | T-6 |
| AC-7.3 | T-7 |
| AC-7.4 | T-7 |
| AC-7.5 | T-8 |
| AC-7.6 | T-8 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-27 | 20260927-191208-risk-8-sort-pages-by-slug | Initial design. `traverse` sorts each folder's entries by name, so the first load error is the first by path. `load` sorts pages by slug, so plan order, write order, render-error choice and collision choice follow slug order. One safety-net test (duplicate slug), new loader, plan and E2E tests, and docs. 8 tasks. |
| 2026-09-27 | 20260927-191208-risk-8-sort-pages-by-slug | Design gate: rewrote the garbled page-title sentence in T-3 (unique titles, none a substring of another) and proofread the document. No change to the approach or tasks. |
