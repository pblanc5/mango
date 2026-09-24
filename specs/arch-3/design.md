<!-- Published by dev-pipeline run 20260923-220004-arch-3-slug-tag-newtypes, stage design v1, approved 2026-09-23T22:48:13-05:00. -->

# `Slug` and `Tag` newtypes: Design

Spec ID: `arch-3` · Requirements: `specs/arch-3/requirements.md`

## Overview
Two new crate-private types go in `src/content/`:
- **`Slug`** (`slug.rs`) wraps the `/`-joined slug text. Its field is private, so a value exists only after validation, as a derivation from another valid slug or tag, or as one of the two fixed locations (home, tag index). It owns all slug logic: URL, output path, parent, top-level test, segments, ordering, display and serialization.
- **`Tag`** (`tag.rs`) wraps a validated tag. It owns tag validation, de-duplication of a tag list, and the tag page's slug and URL.

`Page::new` now takes the content file's path and the site folder, and validates in this order: date, tags, slug. So a `Page` always has a valid slug, and a frontmatter, date or tag error is still reported before a file-name error (AC-1.3). Every consumer (render contexts, output paths, section and tag indexes, home, feed, sitemap) stops handling slug strings and calls these types.

The only change a site author can see: a non-empty dot-only segment is rejected with `a segment cannot consist only of dots`.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/slug.rs` (new) | `Slug` type: validation from a content path, `home()`, `tag_index()`, `tag_page(&Tag)`, `parent()`, `is_top_level()`, `segments()`, `url()`, `output_path(dist)`, `Display`, `Serialize`, derived `Ord`. Unit tests. |
| `src/content/tag.rs` (new) | `Tag` type: `parse`, `parse_list` (validate and de-duplicate), `slug()`, `url()`, `Display`, `Serialize`, derived `Ord`. Unit tests. |
| `src/content/mod.rs` | Declares `pub(crate) mod slug; pub(crate) mod tag;` |
| `src/content/page.rs` | `Page.slug: Slug`, `Page.tags: Vec<Tag>`. `Page::new` gains `path`/`site` and builds the slug. Removes `generate_slug`, `is_valid_slug_segment`, `slug_url`, `tag_slug`, `validate_tags`, `is_valid_tag`. Slug and tag tests move to the new modules. |
| `src/content/loader.rs` | One-step `Page::new(fm, markdown, PageType::General, path, site)`. The `generate_slug` call is removed. Test assertions compare via `to_string()`. |
| `src/content/summary.rs` | `PageSummary.slug: Slug`, and `url` comes from `page.slug.url()`. |
| `src/render/template.rs` | `TagLink`, `TagIndexEntry` and `TagTemplate` hold `name: Tag`. `SectionTemplate`, `SectionLink`, `HomeSection` and `RenderItem` hold `slug: Slug`. URLs come from `.url()`. Home uses `Slug::home()`, the tag index `Slug::tag_index()`, and a tag page `tag.slug()` / `tag.url()`. `render_tag_page` takes a `Tag`. |
| `src/build/output.rs` | `get_final_path` removed; `item.slug.output_path(dist)` replaces it. |
| `src/build/index/section.rs` | `SectionSlug` alias and `extract_parent_slug` removed. Keys are `Slug` (`BTreeMap<Slug, Section>`, `subsections: Vec<Slug>`), and `Slug::parent()` finds the parent. |
| `src/build/index/tag.rs` | `build_tag_index` returns `BTreeMap<Tag, Vec<PageSummary>>`. |
| `src/build/generate/home.rs` | `!slug.contains('/')` becomes `slug.is_top_level()`. |
| `src/build/generate/tag.rs` | Takes `BTreeMap<Tag, …>` and passes `Tag` values on. |
| `src/build/generate/section.rs` | Passes `Slug` values on (the code is otherwise unchanged). Test helper updated. |
| `src/build/generate/feed.rs` | `slug_url(&page.slug)` becomes `page.slug.url()`. The escape test uses a valid slug. |
| `src/build/generate/sitemap.rs` | `slug_url(&item.slug)` becomes `item.slug.url()`. The escape test uses a valid slug. |
| `tests/build.rs` | New E2E test for `site/...md`. No existing test changes. |
| `README.md` | Line 68: dot-only segments are rejected. |
| `CLAUDE.md` | "File names are strict", "Tags are strict", module map (`content/`, `build/output.rs`, `build/index/section.rs`). |
| `specs/_system/backlog.md` | ARCH-3 `done` with a "Landed" note. ARCH-6 note updated. New RISK-6 in the index and as its own section. |

## Approach

**Slug text and validation.** `Slug(String)` holds the `/`-joined text exactly as today; the home slug is `""`.

`Slug::from_content_path(path, site)` does what `generate_slug` did:
1. `strip_prefix(site)`, and on failure returns `General("unable to generate slug from path")` (AC-1.7).
2. `with_extension("")`, then `to_string_lossy()`, then `replace('\\', "/")`. This keeps AC-1.5, AC-1.6 and AC-7.7.
3. Passes the text to the private `Slug::parse(text, file)`.

`parse` walks the segments in order and fails on the first bad one:
- A segment that is empty or has a character outside `[A-Za-z0-9._-]` gets the AC-1.2 message: `<file>: invalid file name '<seg>': use only ASCII letters, digits, '-', '_' and '.'`.
- Otherwise, a segment made only of `.` gets the AC-8.1 message: `<file>: invalid file name '<seg>': a segment cannot consist only of dots`.

Both are `MangoError::General`. Because the empty and charset check runs first, an empty segment can never get the dot-only message (AC-5.2).

**Constructors (the AC-5.1 review list).** This is the complete list:
- `from_content_path` validates.
- `parse` is private and validates.
- `parent()` derives from an existing slug.
- `tag_page(&Tag)` derives from a tag.
- `home()` and `tag_index()` are the fixed locations.
- `#[cfg(test)] from_test_text(&str)` calls `parse` and panics on invalid input.

The type has no `Default`, `From`, `Deserialize`, public field or `FromStr`.

**Derived facts.**
- `url()` returns `"/"` for home, otherwise `"/<text>/"`.
- `segments()` yields nothing for home, otherwise `text.split('/')`.
- `output_path(dist)` is `dist` plus the segments plus `index.html`. This is identical to `get_final_path`, byte for byte.
- `parent()` is `rsplit_once('/')` on the text, so it returns `None` for top-level slugs and for home.
- `is_top_level()` is true when the text is non-empty and has no `/`. Home is not top-level. Home never appears among section keys, so `home.sections` does not change.

Ordering is the derived `Ord` on the inner `String`, which compares bytes (AC-5.4). `Display` and `#[serde(transparent)] Serialize` produce the plain text (AC-5.5), so template contexts and collision labels are unchanged.

**Tag.** `Tag(String)`, with `Tag::parse(String)` running today's `is_valid_tag` check and today's exact `Frontmatter` message (AC-4.1, AC-6.2). `Tag::parse_list(Vec<String>)` parses each value in order and keeps the first occurrence (AC-4.2, AC-6.3). `Tag::slug()` returns `Slug::tag_page(self)` (`tags/<tag>`), and `Tag::url()` returns `self.slug().url()` (AC-6.4). It derives `Ord`, `Display` and transparent `Serialize` (AC-6.5).

**One-step `Page` (AC-5.7, Open question 3).** `Page::new(fm, content, kind, path, site)` runs `parse_date`, then `Tag::parse_list`, then `Slug::from_content_path`. Doing the slug last keeps AC-1.3's precedence. The loader's `with_path` rewrites only `Frontmatter` errors, and the slug errors are `General` and already name the file, so every message stays the same. The alternatives would add a placeholder slug or an `Option<Slug>` (see below). This also closes ARCH-6's two-phase bullet. ARCH-6 keeps its other points (`PageType`, `compare_summaries`, `recent_pages`).

**Replaced call sites (AC-5.6).**
| Before | After |
|---|---|
| `page.rs` `generate_slug` + `is_valid_slug_segment` | `Slug::from_content_path` / `Slug::parse` |
| `template.rs` `slug_url(&tag_slug(&name))` ×2, `tag_slug(&name)` + `slug_url(&slug)` in `render_tag_page` | `tag.url()`, `tag.slug()` |
| `template.rs` `slug_url` in `PageTemplate::from`, `SectionLink::new`, `HomeSection::new`, `render_section_page` | `slug.url()` |
| `template.rs` literal `String::new()` (home) and `"tags".into()` | `Slug::home()`, `Slug::tag_index()` |
| `summary.rs:21`, `feed.rs:36`, `sitemap.rs:23` `slug_url(...)` | `.url()` |
| `output.rs` `get_final_path` | `Slug::output_path` |
| `section.rs` `extract_parent_slug` | `Slug::parent` |
| `home.rs:32` `!slug.contains('/')` | `Slug::is_top_level` |
| `page.rs` `validate_tags` / `is_valid_tag` | `Tag::parse_list` / `Tag::parse` |

Error messages and collision labels still print a slug or tag through `Display` (Open question 4). `loader::site_relative` still rewrites `\` to `/` when it labels the *path* in the symlinked-folder error. That label is a path, not a slug, and it is out of scope.

## Interfaces and data
```rust
// src/content/slug.rs
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Slug(String);
impl Slug {
    pub fn from_content_path(path: &Path, site: &Path) -> Result<Slug, MangoError>;
    fn parse(text: String, file: &Path) -> Result<Slug, MangoError>;
    pub fn home() -> Slug;                 // ""
    pub fn tag_index() -> Slug;            // "tags"
    pub fn tag_page(tag: &Tag) -> Slug;    // "tags/<tag>", called only by Tag::slug
    pub fn parent(&self) -> Option<Slug>;
    pub fn is_top_level(&self) -> bool;
    pub fn segments(&self) -> impl Iterator<Item = &str>;
    pub fn url(&self) -> String;
    pub fn output_path(&self, dist: &Path) -> PathBuf;
    #[cfg(test)] pub(crate) fn from_test_text(text: &str) -> Slug;
}
impl fmt::Display for Slug { /* the text */ }

// src/content/tag.rs
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Tag(String);
impl Tag {
    pub fn parse(value: String) -> Result<Tag, MangoError>;
    pub fn parse_list(values: Vec<String>) -> Result<Vec<Tag>, MangoError>;
    pub fn slug(&self) -> Slug;
    pub fn url(&self) -> String;
}
impl fmt::Display for Tag { /* the text */ }

// src/content/page.rs
pub struct Page { /* … */ pub slug: Slug, pub tags: Vec<Tag>, /* … */ }
pub fn new(fm: MangoFrontmatter, content: String, kind: PageType, path: &Path, site: &Path) -> Result<Page, MangoError>;

// render/template.rs field types
TagLink { name: Tag, url: String }          TagIndexEntry { name: Tag, url: String, page_count: usize }
TagTemplate { name: Tag, url, pages }       SectionTemplate { slug: Slug, url, pages, subsections }
SectionLink { slug: Slug, url }             HomeSection { slug: Slug, url, page_count }
RenderItem { slug: Slug, source, template, context, page_date }
pub fn render_tag_page(tag: Tag, pages: Vec<PageSummary>, config: &SiteConfig) -> RenderItem;

// build/index
SectionIndex { sections: BTreeMap<Slug, Section> }   Section { pages, subsections: Vec<Slug> }
pub fn build_tag_index(pages: &[Page]) -> BTreeMap<Tag, Vec<PageSummary>>;
```
Other points:
- The library's public surface does not change: `src/lib.rs` is untouched, and the types stay unreachable from outside the crate because the `content` module is private.
- There is no schema, template context, URL or output-path change.
- There is one new error text (AC-8.1).

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Keep two-phase construction and hold a placeholder `Slug::home()` until the path is known | A `Page` would claim the home location until the slug is fixed. That meets the letter of AC-5.7 but hides the bug it guards against. |
| `slug: Option<Slug>` filled in later | Every consumer would need to handle `None`, and `unwrap`/`expect` is banned in production code. |
| Validate the slug in the loader first and pass a `Slug` into `Page::new` | A file-name error would then win over a frontmatter, date or tag error, which breaks AC-1.3 and AC-7.2. |
| Store the slug as `Vec<String>` segments | Derived ordering would compare segment by segment (`a/b` before `a-c`), which breaks AC-3.1 and AC-5.4. It would also need a custom `Ord` and a join for every URL. |
| Put `Slug` and `Tag` in one module so `Tag::slug` can build the slug directly | This works, but two files keep each type's rules separate. The single cross-module constructor, `Slug::tag_page(&Tag)`, only derives from a valid tag, so it passes the AC-5.1 review. |
| `impl Borrow<str>` / `PartialEq<&str>` so tests can write `sections["posts"]` | Production code does not need them. Tests use `to_string()` or `Slug::from_test_text`, which keeps the types' surface minimal. |
| Keep the AC-1.2 message for dot-only segments | Rejected by the approver (Open question 5): the message would list `.` as allowed. |

## Risks
- **Byte-identical output (AC-7.1).** `output_path`, `url` and `Serialize` must reproduce `get_final_path`, `slug_url` and the plain strings exactly. `build_generates_site_from_fixture`, `fixture_build_is_deterministic` and `fixture_internal_links_resolve` guard this. The unit tests of `url()`/`output_path()` pin the home case.
- **Unit-test churn can weaken coverage.** About ten test helpers assign `page.slug = …`, and two escape tests use slugs that are now invalid (`feed::values_are_escaped` uses `posts/a&b`, `sitemap::escapes_loc` uses `x<y`). Those two must switch to valid slugs (for example `posts/a-b` and `x-y`) and keep the escaping assertions through `base_url`, which still contains `&` and `'`. That keeps legacy batch-4 AC-6.5 and AC-7.5 verified. No other expected value may change beyond the type adaptations.
- **Legacy look-up by test name.** `specs/_system/legacy-criteria/` finds tests by name. Rewritten tests that legacy tables name keep their names and `// AC-…` tags, even after they move modules: `slug_url_is_root_relative_with_trailing_slash`, `validate_tags_accepts_valid_values`, `validate_tags_rejects_invalid_values_naming_value`, `validate_tags_removes_duplicates_keeping_order`, `new_rejects_invalid_tag`, `new_defaults_missing_date_and_tags` and `home_slug_maps_to_root_index`.
- **Intermediate dead-code warnings.** T-3 adds the types before anything uses them, so clippy reports them as unused until T-4 lands. The gate is required only after T-4. No `#[allow]`.
- **Dot-only rejection breaks an existing site.** A site that has a dot-only file name today will fail to build. This is intended (REQ-8), the message says why, and it happens before the output folder is cleaned.
- **Windows.** The backslash rewrite is kept unchanged. The AC-7.7 test gives `a/b` on both platforms, because `\` is a separator on Windows anyway.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- **Safety net.** These `[baseline]` criteria have no test pinning their exact current behavior:
  - AC-1.2: only part of the message is asserted.
  - AC-1.3: no precedence test exists.
  - AC-1.5 / AC-7.7: the backslash behavior is not tested.
  - AC-1.6: non-UTF-8 file names are not tested.
  - AC-1.7: the message text is not pinned.
  - AC-3.1: nothing pins section order by byte-wise comparison of the full text (`a-c` before `a/b`).
  - AC-4.3: `section.slug` in the section context is not asserted.

  T-1 and T-2 add these against the current code, before any change. The other baseline criteria are already covered: AC-1.1, AC-2.1–2.7, AC-3.2, AC-3.3, AC-4.1 and AC-4.2 (tests listed in the requirements' Evidence lines). AC-1.4 is replaced by REQ-8, so it gets no safety net.
- **New and changed behavior:**
  - `slug.rs` tests:
    - `from_content_path` behavior (AC-1.1, AC-1.2, AC-1.5 to AC-1.7, AC-7.7), moved from T-1.
    - `parse` rejection: `a//b`, `""`, `/a` and `a b` get the AC-1.2 message. `.`, `..`, `...`, `posts/..` and `a/./b` get the AC-8.1 message (AC-5.2, AC-7.4, AC-8.1, AC-8.2).
    - `url`/`output_path`/`parent`/`is_top_level`/`segments`, including home (AC-5.3). `slug_url_is_root_relative_with_trailing_slash` keeps its tags.
    - `a-c` < `a/b` (AC-5.4).
    - `Display` and `serde_json::to_value` give plain strings (AC-5.5).
  - `tag.rs` tests: the three `validate_tags_*` tests rewritten on `Tag::parse_list`/`Tag::parse` (AC-4.1, AC-4.2, AC-6.2, AC-7.4); slug and URL (AC-6.4, from `tag_slug_is_under_tags`); ordering and serialization (AC-6.5).
  - `page.rs`: `new_rejects_invalid_tag` (AC-6.3), plus `new_derives_slug_from_path` (AC-5.7).
  - `loader.rs`: a dot-only draft is rejected (AC-8.1).
  - `tests/build.rs`: `build_fails_on_dot_only_file_name_keeping_output` (AC-8.2).
  - All existing fixture, E2E and plan tests pass unedited (AC-7.1, AC-7.2).
  - AC-5.1, AC-5.6, AC-6.1 and AC-7.5 are verified by review against the constructor list and the call-site table above.

## Tasks
### T-1 Pin current slug-building behavior
- Kind: safety-net
- Satisfies: AC-1.2, AC-1.3, AC-1.5, AC-1.6, AC-1.7, AC-7.7
- Files: `src/content/page.rs`, `src/content/loader.rs`
- Done when: these new tests pass against the unchanged code:
  - In `page.rs`:
    - `generate_slug_error_message_is_exact` (`// AC-arch-3.1.2`): `site/a+b.md` gives `MangoError::General` with the full text `Mango Error: site/a+b.md: invalid file name 'a+b': use only ASCII letters, digits, '-', '_' and '.'`.
    - `generate_slug_outside_site_message` (`// AC-arch-3.1.7`): exact text `unable to generate slug from path`.
    - `backslash_in_stem_becomes_separator` (`// AC-arch-3.7.7`): `Path::new("site/a\\b.md")` gives slug `a/b`.
    - `#[cfg(unix)] non_utf8_file_name_is_rejected_lossily` (`// AC-arch-3.1.6`): `OsStr::from_bytes(b"site/\xff.md")` gives the AC-1.2 error naming segment `'\u{FFFD}'`.
  - In `loader.rs`, `frontmatter_error_wins_over_invalid_file_name` (`// AC-arch-3.1.3`): `my posts/x.md` with tag `Rust` gives a `Frontmatter` error containing `'Rust'` and not `invalid file name`. The same holds with date `2026-02-30`.

### T-2 Pin listing order and the section context's slug
- Kind: safety-net
- Satisfies: AC-3.1, AC-4.3
- Files: `src/build/index/section.rs`, `src/render/template.rs`
- Done when: these new tests pass against the unchanged code:
  - `sections_order_bytewise_not_by_segment` (`// AC-arch-3.3.1`): pages `a/b/x` and `a-c/x` give section keys `["a", "a-c", "a/b"]`.
  - `section_context_slug_is_plain_string` (`// AC-arch-3.4.3`): `section["slug"] == json!("a")` and `section["subsections"][0]["slug"] == json!("a/b")`.

### T-3 Add the `Slug` and `Tag` types
- Kind: implementation
- Satisfies: AC-5.1, AC-5.2, AC-5.3, AC-5.4, AC-5.5, AC-6.1, AC-6.2, AC-6.4, AC-6.5, AC-7.4, AC-8.1, AC-8.2
- Files: `src/content/slug.rs`, `src/content/tag.rs`, `src/content/mod.rs`
- Done when:
  - The types exist exactly as in Interfaces, with the constructor list of Approach and no other constructors.
  - Unit tests tagged `// AC-arch-3.<req>.<m>` pass:
    - empty segments (`a//b`, `""`) get the AC-1.2 message and not the AC-8.1 message
    - `.`, `..`, `posts/..` and `a/./b` get the AC-8.1 message
    - `url`, `output_path`, `parent`, `is_top_level` and `segments`, including home and `tags/<tag>`
    - `a-c` < `a/b`
    - `Display` and serialization give the plain string
    - tag validation and de-duplication give the exact existing message
    - `Tag::slug()` is `tags/blog` and `Tag::url()` is `/tags/blog/`
    - tag ordering is byte-wise
  - `cargo test content::` passes. Unused-item warnings are expected until T-4.

### T-4 Move the crate onto `Slug` and `Tag`, with one-step `Page`
- Kind: implementation
- Satisfies: AC-1.1, AC-1.2, AC-1.3, AC-1.5, AC-1.6, AC-1.7, AC-2.1, AC-2.2, AC-2.3, AC-2.4, AC-2.5, AC-2.6, AC-2.7, AC-3.1, AC-3.2, AC-3.3, AC-4.1, AC-4.2, AC-4.3, AC-5.6, AC-5.7, AC-6.3, AC-6.4, AC-7.1, AC-7.2, AC-7.3, AC-7.5, AC-7.6, AC-7.7, AC-8.1
- Files: `src/content/page.rs`, `src/content/loader.rs`, `src/content/summary.rs`, `src/content/slug.rs`, `src/content/tag.rs`, `src/render/template.rs`, `src/build/output.rs`, `src/build/index/section.rs`, `src/build/index/tag.rs`, `src/build/generate/home.rs`, `src/build/generate/tag.rs`, `src/build/generate/section.rs`, `src/build/generate/feed.rs`, `src/build/generate/sitemap.rs`
- Done when:
  - Every row of the call-site table is done, and `generate_slug`, `is_valid_slug_segment`, `slug_url`, `tag_slug`, `validate_tags`, `is_valid_tag`, `get_final_path`, `extract_parent_slug` and `SectionSlug` no longer exist (`grep` finds none of them in `src/`).
  - `Page::new` takes `path`/`site` and validates in the order date, tags, slug.
  - The slug tests from T-1 and `generate_slug_*` have moved to `slug.rs`, and the tag tests to `tag.rs`. Legacy-referenced test names and `// AC-…` tags are kept (see Risks).
  - `extract_parent_slug_uses_last_separator` and `tag_slug_is_under_tags` are rewritten as equivalent `Slug::parent` and `Tag::slug`/`url` tests.
  - `new_derives_slug_from_path` (`// AC-arch-3.5.7`) is added in `page.rs`.
  - `dot_only_file_name_is_rejected_even_for_drafts` (`// AC-arch-3.8.1`) is added in `loader.rs`.
  - Test helpers build slugs only through `Page::new` with a path, `Slug::from_test_text`, `Slug::home()`/`tag_index()` or `Tag::parse`.
  - The feed and sitemap escape tests use valid slugs.
  - `src/lib.rs`, `Cargo.toml`, `tests/build.rs` and `tests/plan.rs` are unchanged.
  - The full gate passes.

### T-5 E2E test for a dot-only file name
- Kind: test
- Satisfies: AC-8.1, AC-8.2
- Files: `tests/build.rs`
- Done when: `build_fails_on_dot_only_file_name_keeping_output` (`// AC-arch-3.8.2`) passes:
  1. Build a temp site with `posts/one.md`, then write `marker.txt` into the output and take a `snapshot`.
  2. Add `site/...md` and rebuild. The rebuild fails (`assert_failure`), and stderr contains `...md`, `'..'` and `a segment cannot consist only of dots`.
  3. The snapshot is unchanged.

### T-6 Update user and developer docs
- Kind: docs
- Satisfies: AC-8.3, AC-9.1, AC-9.3
- Files: `README.md`, `CLAUDE.md`
- Done when:
  - `README.md:68` says a folder or file name made only of dots (such as `...md`) is rejected. Nothing else in `README.md` changes.
  - `CLAUDE.md`:
    - "File names are strict" names `content::slug::Slug` and gives the dot-only rule and message.
    - "Tags are strict" names `content::tag::Tag` (`Tag::parse`/`parse_list` from `Page::new`, `Tag::slug`/`url`).
    - The module map lists `slug.rs` and `tag.rs`, and describes `page.rs` as one-step `Page::new` plus `parse_date`.
    - The `build/output.rs` entry refers to `Slug::output_path`, and the `index/section.rs` entry to `Slug::parent`.
  - No removed function is named as current code.

### T-7 Update the backlog
- Kind: docs
- Satisfies: AC-9.2, AC-9.4
- Files: `specs/_system/backlog.md`
- Done when:
  - ARCH-3 is `done` in the index and in its header line, and has a "Landed." paragraph.
  - ARCH-6's two-phase-construction bullet, proposal and done-when say that one-step construction landed with ARCH-3, and its other points stay open.
  - ARCH-2 still reads correctly. `Tag(Tag)` now refers to `content::tag::Tag`, and a note is added if needed.
  - RISK-6 "Backslashes in file names become folder separators" (low, S, `/spec-feature`, open) is added to the index and as its own section. It describes the `a\b.md` case (published at `/a/b/`, creates a section `a`, even though `\` is not an allowed character), names the pinning test `backslash_in_stem_becomes_separator`, and proposes rejecting the backslash or documenting it.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-4 |
| AC-1.2 | T-1, T-4 |
| AC-1.3 | T-1, T-4 |
| AC-1.4 | replaced by AC-8.1: T-3, T-4, T-5 |
| AC-1.5 | T-1, T-4 |
| AC-1.6 | T-1, T-4 |
| AC-1.7 | T-1, T-4 |
| AC-2.1 | T-4 |
| AC-2.2 | T-4 |
| AC-2.3 | T-4 |
| AC-2.4 | T-4 |
| AC-2.5 | T-4 |
| AC-2.6 | T-4 |
| AC-2.7 | T-4 |
| AC-3.1 | T-2, T-4 |
| AC-3.2 | T-4 |
| AC-3.3 | T-4 |
| AC-4.1 | T-4 |
| AC-4.2 | T-4 |
| AC-4.3 | T-2, T-4 |
| AC-5.1 | T-3 |
| AC-5.2 | T-3 |
| AC-5.3 | T-3 |
| AC-5.4 | T-3 |
| AC-5.5 | T-3 |
| AC-5.6 | T-4 |
| AC-5.7 | T-4 |
| AC-6.1 | T-3 |
| AC-6.2 | T-3 |
| AC-6.3 | T-4 |
| AC-6.4 | T-3, T-4 |
| AC-6.5 | T-3 |
| AC-7.1 | T-4 |
| AC-7.2 | T-4 |
| AC-7.3 | T-4 |
| AC-7.4 | T-3 |
| AC-7.5 | T-4 |
| AC-7.6 | T-4 |
| AC-7.7 | T-1, T-4 |
| AC-8.1 | T-3, T-4, T-5 |
| AC-8.2 | T-3, T-5 |
| AC-8.3 | T-6 |
| AC-9.1 | T-6 |
| AC-9.2 | T-7 |
| AC-9.3 | T-6 |
| AC-9.4 | T-7 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-23 | 20260923-220004-arch-3-slug-tag-newtypes | Initial design: crate-private `Slug` (`content/slug.rs`) and `Tag` (`content/tag.rs`) that own all slug and tag-URL rules, one-step `Page::new(fm, content, kind, path, site)` validating date, tags, then slug, dot-only segments rejected with their own message, 2 safety-net tasks before the migration, docs and backlog (ARCH-3 done, ARCH-6 note, RISK-6). 7 tasks. |
