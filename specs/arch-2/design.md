<!-- Published by dev-pipeline run 20260923-230744-arch-2-one-output-model, stage design v1, approved 2026-09-23T23:52:12-05:00. -->

# One output model instead of three: Design

Spec ID: `arch-2` · Requirements: `specs/arch-2/requirements.md`

## Overview
Every planned output becomes one value, `Output { kind: OutputKind, body: Body }`, defined in `src/build/output.rs`.
- **`OutputKind`** says what the output is: `Page { slug, date }`, `Section(slug)`, `Home`, `TagIndex`, `Tag(tag)`, `Feed`, `Sitemap` or `Asset { folder, rel }`. Three things are derived from it and nowhere else:
  - the output location (`Output::path(dist)`)
  - the collision label (`Display for OutputKind`)
  - the sitemap entry (an exhaustive `match` in `sitemap.rs`)
- **`Body`** says how the bytes are produced: `Template { name, context }`, `Text(String)` or `Copy(source)`.

The pipeline builds one `Vec<Output>` in the AC-2.1 order. `check_collisions` takes that list. `render` consumes it and returns one `Vec<RenderedOutput>`, in which each entry is either text to write or a file to copy. `BuildPlan` holds that one list. `outputs()` enumerates it, and `commit` writes it in a single loop that now also does the asset copies. `RenderItem`, `GeneratedFile`, `AssetFile`, `render_generated`, `assets::copy`, every `source`/`label` string and `page_date` are removed. Output bytes, messages, order, the public API and `README.md` do not change.

## Affected components
| Component / file | Change |
|---|---|
| `/home/roguestar/workspace/mango/src/build/output.rs` | **New:** `Output`, `OutputKind` (with `Display` and derived `Debug, PartialEq, Eq`), `Body`, `RenderedOutput`, `Contents`, `Output::path(dist)`, plus `#[cfg(test)]` accessors `context()`, `template_name()` and `text()`. `check_collisions(dist, &[Output])` replaces the three-input version. `render(tera, dist, Vec<Output>)` replaces `render` and `render_generated`. `write(&[RenderedOutput])` writes text and copies files. `GeneratedFile`, `RenderedFile` and `render_generated` are removed. Unit tests are rewritten, and new tests are added (T-2, T-4). |
| `/home/roguestar/workspace/mango/src/build/pipeline.rs` | Builds one `Vec<Output>` in AC-2.1 order, checks it, renders it, and stores `Vec<RenderedOutput>` in `BuildPlan` (field `outputs`, replacing `files` + `assets`). `outputs()` maps `Contents` to `PlannedOutput`. `commit` calls only `output::write`. `asset_dest` becomes `asset_folder`, the assets folder's name; its error text is unchanged. |
| `/home/roguestar/workspace/mango/src/render/template.rs` | `RenderItem` is removed. `render_page`, `render_section_page`, `render_home_page`, `render_tag_index` and `render_tag_page` keep their names and parameters but return `Output`, with the kind set and `Body::Template { name: "<x>.html", context }`. Template names stay hardcoded here. Tests are rewritten (see AC-9.5 table). |
| `/home/roguestar/workspace/mango/src/build/generate/content.rs` | `Result<Vec<Output>, MangoError>` |
| `/home/roguestar/workspace/mango/src/build/generate/section.rs` | Returns `Vec<Output>`. Test rewritten. |
| `/home/roguestar/workspace/mango/src/build/generate/home.rs` | `build` returns `Output`. Test helpers and one test rewritten. `recent_pages` is unchanged. |
| `/home/roguestar/workspace/mango/src/build/generate/tag.rs` | Returns `Vec<Output>`. Tests rewritten. |
| `/home/roguestar/workspace/mango/src/build/generate/feed.rs` | Returns `Option<Output>` (`Feed`, `Body::Text`). Tests use `.text()` and the kind. |
| `/home/roguestar/workspace/mango/src/build/generate/sitemap.rs` | `build(impl IntoIterator<Item=&Output>, config) -> Option<Output>`. Selection and `<lastmod>` come from a private exhaustive `match` on `OutputKind`. Tests rewritten, and the AC-8.3 test added. |
| `/home/roguestar/workspace/mango/src/build/generate/assets.rs` | `AssetFile` and `copy` are removed. `plan(assets_source, folder)` returns `Vec<Output>` (`Asset { folder, rel }`, `Body::Copy(source)`), sorted by `rel`. Tests go through `output::render` + `output::write`. Safety-net test added (T-2). |
| `/home/roguestar/workspace/mango/tests/plan.rs` | Safety-net in-process tests (T-1), including the AC-7.4 test. No existing test changes. |
| `/home/roguestar/workspace/mango/CLAUDE.md` | Pipeline, feed/sitemap and module-map text updated to describe the single model (AC-10.1). |
| `/home/roguestar/workspace/mango/specs/_system/backlog.md` | ARCH-2 marked `done` with a Landed note. ARCH-5 updated. RISK-7 and RISK-8 added (AC-10.2). |

Not changed: `README.md` (AC-9.7), `tests/build.rs`, `example/`, `src/content/*` (including `Slug::output_path(dist)`, which arch-3 AC-5.3 requires), `src/lib.rs`, `Cargo.toml` and `specs/_system/overview.md` (DOC-1).

## Approach

**1. The kind is the identity (AC-6.4, AC-7.1).**
```rust
pub enum OutputKind {
    Page { slug: Slug, date: Option<NaiveDate> },
    Section(Slug),
    Home,
    TagIndex,
    Tag(Tag),
    Feed,
    Sitemap,
    Asset { folder: PathBuf, rel: PathBuf },
}
```
In `Asset`, `folder` is the assets folder's name, i.e. the folder under the output folder that assets are copied into. `rel` is the path relative to the assets folder. Every field belongs to the variant it describes, so no field is "empty for the other kinds". The page date lives only on `Page`. Nothing on `Output` exists only for the sitemap (AC-8.1).

**2. Location comes from the kind (AC-1.2 to AC-1.4, AC-6.1).** `Output::path(&self, dist)` is one exhaustive `match`:
| Kind | Location |
|---|---|
| `Page { slug, .. }`, `Section(slug)` | `slug.output_path(dist)` |
| `Home` | `Slug::home().output_path(dist)` |
| `TagIndex` | `Slug::tag_index().output_path(dist)` |
| `Tag(t)` | `t.slug().output_path(dist)` |
| `Feed` | `dist.join("feed.xml")` |
| `Sitemap` | `dist.join("sitemap.xml")` |
| `Asset { folder, rel }` | `dist.join(folder).join(rel)` |

These are exactly today's computations (`asset_dest = dist.join(name)`, then `asset_dest.join(&rel)`), so every path, and every path shown in a message, is byte-identical. Producers no longer need `dist`. A feed can no longer be placed anywhere except `feed.xml`, which removes states that are impossible in production anyway (see Risks).

**3. Labels come from the kind (AC-3.1, AC-7.1, AC-7.2).** `impl Display for OutputKind` prints:
- `page '{slug}'`
- `section index '{slug}'`
- `home page`
- `tag index`
- `tag page '{tag}'`
- `RSS feed`
- `sitemap`
- `asset '{rel}'`, where `rel` is printed via `to_string_lossy().replace('\\', "/")`, the same expression `assets::collect` uses today

`check_collisions` keeps its algorithm unchanged but uses `HashMap<PathBuf, &OutputKind>` and `Vec<(PathBuf, &OutputKind)>`, and formats labels with `{}`. The two `format!` strings are not touched, so the messages, the `General` variant, the first-pair rule and the ancestor walk stay identical.

**4. Sitemap selection by kind (AC-8.1, AC-8.2).** A private `fn entry(kind: &OutputKind) -> Option<(String, Option<NaiveDate>)>` returns the URL and date:
- `Page` → `(slug.url(), date)`
- `Section` → `(slug.url(), None)`
- `Home` → `Slug::home().url()`
- `TagIndex` → `Slug::tag_index().url()`
- `Tag` → `t.url()`
- `Feed | Sitemap | Asset{..}` → `None`

The match lists every variant and has no `_` arm, so adding a kind forces a decision here. The rest of `build` (base URL, sort, XML) is unchanged, so the bytes are unchanged (AC-8.4). The pipeline passes the list after the feed has been appended, so production also relies on the kind filter.

**5. One list, AC-2.1 order (AC-6.3).** `plan` does the following:
1. Loading and `assets::plan(assets, &asset_folder)` stay where they are, so the failure order is unchanged.
2. Build `outputs = content::build(..)?`.
3. Build the section index. Build home from it, then extend with sections and push home.
4. Extend with tag outputs, then extend with `feed::build(..)`.
5. `let sitemap = sitemap::build(&outputs, ..); outputs.extend(sitemap);`
6. `outputs.extend(asset_outputs)`.
7. `check_collisions(dist, &outputs)?`, then `render(&tera, dist, outputs)?`.

The collision check still runs before any template is rendered (AC-3.5).

**6. Rendered form and commit (AC-5.1 to AC-5.4, AC-6.5).**
- `render` consumes the list: `Template` → `Contents::Text(tera.render(name, &context)?)`, `Text(s)` → `Contents::Text(s)` with no clone, `Copy(src)` → `Contents::Copy(src)`. `collect` stops at the first template error in list order (AC-5.2). The path is computed before the body is moved.
- `write` loops once:
  - `create_dir_all(parent)`, mapping an error to `io_at(parent)`
  - `Text` → `fs::write(path)`, mapping an error to `io_at(path)`
  - `Copy(src)` → `fs::copy(src, path)`, mapping an error to `io_at(src)`. A comment points to RISK-7.
- Because assets are always last in the list, this single loop writes every file and then copies every asset, as today (AC-2.3). The T-1 order test pins that assets come last.
- `commit` remains the only caller of `write` and `clean_contents`.

**7. Output construction stays in `render/template.rs`.** AC-10.2 says ARCH-5's "item construction" stays open, so the five constructors stay where they are. They now build `build::output::Output`. `build` no longer imports its core type from `render`, which closes that ARCH-5 bullet. `render/template.rs` now imports `Output` from `build`. That reverse edge is the one ARCH-5 removes when it moves construction into `build/generate/*`, and the ARCH-5 backlog text says so.

**8. Assets (AC-1.4).** `collect` gathers `(rel, source)` pairs. `plan` sorts them by `rel`, which gives the same order as today's sort by `dest`, since `dest = asset_dest.join(rel)` and the prefix components are shared. Each pair becomes an `Output`. The symlink and outside-folder errors are unchanged.

**9. Test names are kept.** Legacy numeric tags resolve by test name (constitution), so every rewritten test keeps its name and its `// AC-…` tag. This includes `render_generated_maps_paths_without_touching_fs` and `generated_file_collides_with_render_item`, whose bodies now use the new model.

## Interfaces and data
```rust
// src/build/output.rs (module is private; types are crate-internal like every other `pub` item under `build`)
#[derive(Debug)]
pub struct Output { pub kind: OutputKind, pub body: Body }
#[derive(Debug, PartialEq, Eq)]
pub enum OutputKind { Page { slug: Slug, date: Option<NaiveDate> }, Section(Slug), Home, TagIndex, Tag(Tag), Feed, Sitemap, Asset { folder: PathBuf, rel: PathBuf } }
#[derive(Debug)]
pub enum Body { Template { name: &'static str, context: tera::Context }, Text(String), Copy(PathBuf) }
#[derive(Debug)]
pub struct RenderedOutput { pub path: PathBuf, pub contents: Contents }
#[derive(Debug)]
pub enum Contents { Text(String), Copy(PathBuf) }

impl Output { pub fn path(&self, dist: &Path) -> PathBuf; }
impl fmt::Display for OutputKind { /* AC-3.1 labels */ }
#[cfg(test)] impl Output { pub(crate) fn context(&self) -> &tera::Context; pub(crate) fn template_name(&self) -> &str; pub(crate) fn text(&self) -> &str; } // panic on the wrong body

pub fn check_collisions(dist: &Path, outputs: &[Output]) -> Result<(), MangoError>;
pub fn render(tera: &Tera, dist: &Path, outputs: Vec<Output>) -> Result<Vec<RenderedOutput>, MangoError>;
pub fn write(outputs: &[RenderedOutput]) -> Result<(), MangoError>;

// producers
template::render_page(&Page, &SiteConfig) -> Result<Output, MangoError>
template::render_section_page(Slug, Vec<PageSummary>, Vec<SectionLink>, &SiteConfig) -> Output
template::render_home_page(Vec<PageSummary>, Vec<HomeSection>, &SiteConfig) -> Output
template::render_tag_index(Vec<TagIndexEntry>, &SiteConfig) -> Output
template::render_tag_page(Tag, Vec<PageSummary>, &SiteConfig) -> Output
content::build(&[Page], &SiteConfig) -> Result<Vec<Output>, MangoError>
section::build(SectionIndex, &SiteConfig) -> Vec<Output>
home::build(&[Page], &SectionIndex, &SiteConfig) -> Output
tag::build(BTreeMap<Tag, Vec<PageSummary>>, &SiteConfig) -> Vec<Output>
feed::build(&[Page], &SiteConfig) -> Option<Output>
sitemap::build<'a>(impl IntoIterator<Item = &'a Output>, &SiteConfig) -> Option<Output>
assets::plan(assets_source: &Path, folder: &Path) -> Result<Vec<Output>, MangoError>

// pipeline.rs
pub struct BuildPlan { output_dir: PathBuf, protected: Vec<PathBuf>, outputs: Vec<RenderedOutput> }
```
Public surface: unchanged. `PlannedOutput::{File, Copy}` and their fields are untouched (AC-9.4). There are no schema, format or file changes.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Store a `path` on every `Output`, as in the backlog sketch (`path: OutputPath`) | The slug would be duplicated in both path and kind, and the two could disagree. It would also mean threading `dist` into every producer, or adding a relative-path API to `Slug`, which arch-3 AC-5.3 pins as `output_path(dist)`. Deriving the path from the kind gives one source of truth. |
| Move `Output` construction into `build/generate/*` now, with `render` returning only contexts | That is ARCH-5's "item construction" work, which AC-10.2 says stays open. Doing it here would make the required backlog text false. |
| Keep the three types and add a shared trait for label and path | Fails AC-6.2 and AC-6.3, because there would still be three shapes and three inputs. |
| Reuse `Body` as the rendered form (`Template` never present after render) | `commit` would need an unreachable arm, which means a panic in production code, and that is forbidden. `Contents` rules the state out by type. |
| Keep `assets::copy` as a separate commit step | `commit` would still handle one output shape separately (AC-6.3). |
| Let the caller filter what the sitemap sees | Decided against at the gate (Open question 5). |
| Store the asset `rel` as a pre-formatted `String` | Keeping `PathBuf` makes `path()` a plain join, and the label formatting stays the exact expression used today. |

## Risks
- **Module cycle `render` ↔ `build`.** `render/template.rs` imports `build::output`, and `build` imports `render`. Rust allows this, and it is the edge ARCH-5 removes. The ARCH-5 backlog text records it (T-6).
- **One legacy assertion can't be built anymore.** `generated_file_collides_with_render_item` put a feed at `posts/one/index.html`. With derived locations, an HTML output and the feed or sitemap can never share a path. That was already impossible in production, because HTML outputs end in `index.html` and the generated files sit at the root. Its replacement assertions are:
  - an exact cross-kind collision where the HTML output is named first (page vs asset)
  - an exact duplicate of a generated output (`both RSS feed and RSS feed`)
  - the existing file-vs-folder test with `RSS feed`

  Legacy batch-4 AC-5.1 stays verified by the same-named test. Listed in the AC-9.5 table.
- **Assets must come last in the list.** This holds `commit`'s single loop to "files, then copies" (AC-2.3) and the enumeration split (AC-9.3). The pipeline appends them last, and T-1 pins it.
- **Path equality and display.** Every path is built by the same join sequence as today, so `Display` in messages and `HashMap` equality are unchanged. The T-1 exact-message tests, `tests/plan.rs` and the E2E tests check it.
- **`Debug` derives.** `Output: Debug` needs `tera::Context: Debug`. Tera derives it, and `expect_err` on `assets::plan` results needs it. If it is ever missing, drop the derive and change those tests to `match`.
- **Diff size.** Many unit tests change mechanically, and there is a risk of silently weakening an assertion. The AC-9.5 table gives the replacement for each one, and the Reviewer should check it line by line.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- **Safety net.** Baseline criteria in code being changed that no existing test pins exactly:
  - **AC-2.1 / AC-2.2:** full kind order with more than one section, tag and asset. Nothing asserts it. → T-1 `plan_enumerates_every_kind_in_order`
  - **AC-3.1 / AC-3.2 / AC-3.3:** exact labels reachable from real input that are only checked by substring today (`section index 'tags'` vs `tag index`, `tag page`, `asset`, `sitemap`). → T-1 `collision_labels_are_exact_for_reachable_kinds` and `page_and_asset_collision_names_the_page_first` (also AC-7.4)
  - **AC-3.5:** collision reported before a render error. No test. → T-1 `collision_is_reported_before_a_render_error`
  - **AC-5.2:** first failing template in order. Not pinned. → T-1 `first_render_error_in_output_order_is_reported`
  - **AC-5.3:** write failure naming the path or parent. No test. → T-2 `write_failure_names_the_path` (`src/build/output.rs`)
  - **AC-5.4:** copy failure names the **source**. The existing test only checks `style.css`, which both paths contain. → T-2 `copy_failure_names_the_source_path` (`src/build/generate/assets.rs`)
  - **Already covered:** AC-1.1 to AC-1.4 (`plan_holds_every_output_fully_rendered`, `build_generates_site_from_fixture`, asset unit tests), AC-3.4 (`index_md_is_not_a_collision`, `build_allows_index_md_inside_section`), AC-4.1 to AC-4.3 (sitemap unit tests, `fixture_feed_and_sitemap`), AC-5.1 (`plan_holds_every_output_fully_rendered`).
  - **No test needed:** AC-2.3 (write order is not observable apart from the enumeration, which is pinned). AC-2.4 (loader not touched). AC-4.4 (the internal field that REQ-8 replaces).
- **New and changed behavior:**
  - AC-7.3 → T-4 `labels_derive_from_kind`
  - AC-6.1 / AC-1.2 to AC-1.4 → T-4 `paths_derive_from_kind`
  - AC-8.3 → T-4 `lists_only_html_kinds_and_dates_only_content_pages`
  - AC-7.4 → T-1 `page_and_asset_collision_names_the_page_first`
  - AC-9.1 to AC-9.3 → every existing test in `tests/build.rs` and `tests/plan.rs` passes unedited, including `fixture_build_is_deterministic`, `build_generates_site_from_fixture` and `planning_twice_gives_identical_outputs`
  - AC-8.4 → the sitemap and feed tests keep their expected XML verbatim
- **AC-9.5 affected unit tests.** Every test keeps its name and tag. "Kind" means comparing `OutputKind` values; "label" means `kind.to_string()`.

| Test (file) | Removed thing it used | Replacement assertion |
|---|---|---|
| `collision_names_path_and_both_sources`, `index_md_is_not_a_collision`, `distinct_paths_pass`, `file_vs_directory_conflict_is_detected`, `asset_conflicts_are_detected` (`output.rs`) | `RenderItem`/`GeneratedFile`/`AssetFile` helpers, three-input `check_collisions` | Same inputs built as `Output`s (`page`, `section`, `feed`, `asset` helpers; asset = `Asset { folder: "assets", rel }`), one slice; identical asserted strings |
| `home_slug_maps_to_root_index` (`output.rs`) | `home.slug.output_path`, free-text `"other root"` | `home().path(dist) == dist/index.html`; two `Home` outputs collide; message has the path and `home page` |
| `generated_file_collides_with_render_item` (`output.rs`) | `GeneratedFile` at an arbitrary path | (1) page, tag index, feed and sitemap pass. (2) Exact: `output path 'dist/assets/index.html' would be written by both page 'assets' and asset 'index.html'`, i.e. HTML output vs non-template output, HTML named first. (3) Two `Feed` outputs: exact `… 'dist/feed.xml' would be written by both RSS feed and RSS feed` |
| `render_generated_maps_paths_without_touching_fs` (`output.rs`) | `render_generated`, `GeneratedFile` | `render(&Tera::default(), &dist, vec![Feed Text("<RSS feed/>"), Sitemap Text("<sitemap/>")])`: paths `dist/feed.xml`, `dist/sitemap.xml`, `Contents::Text` equal, `!dist.exists()` |
| `render_returns_paths_and_html_without_touching_fs`, `render_error_returns_template_error`, `write_creates_parent_dirs_and_files` (`output.rs`) | `RenderItem`, `RenderedFile.html` | Same with `Output`/`RenderedOutput { contents: Contents::Text }` |
| `page_context_date_is_formatted_or_empty` (`template.rs`) | `item.source` | label `== "page 'posts/dated'"`; kind `== Page { slug, date: Some(2026-01-24) }` |
| `section_context_includes_config_url_and_subsections` | `item.slug`, `item.source` | kind `== Section(a)`; label `== "section index 'a'"` |
| `home_item_fields_and_context` | `item.slug`, `.source`, `.template` | kind `== Home`; label `== "home page"`; `template_name() == "home.html"` |
| `page_context_tags_are_links` | `item.page_date` | kind `== Page { slug: page.slug, date: page.date }` |
| `tag_index_context_and_source` | `.slug`, `.source`, `.template`, `.page_date` | kind `== TagIndex` (no date by type); label `== "tag index"`; `template_name() == "tags.html"` |
| `tag_page_context_and_source` | `.slug`, `.source`, `.template`, `.page_date` | kind `== Tag(blog)`; `path(dist) == dist/tags/blog/index.html`; label `== "tag page 'blog'"`; `template_name() == "tag.html"` |
| `page_context_includes_description`, `page_context_includes_url`, `page_context_includes_config`, `section_context_slug_is_plain_string`, `null_config_title_uses_default_filter` | `item.context` field | `item.context()` |
| `render_items_in_section_slug_order` (`section.rs`) | `.slug`, `.source` | kinds `== [Section(a), Section(a/b), Section(posts), Section(projects)]`; `items[2]` label `== "section index 'posts'"` |
| `home_item_has_empty_slug_and_source` + helpers (`home.rs`) | `.slug`, `.source`, `.template`, `RenderItem` | kind `== Home`; label `== "home page"`; `template_name() == "home.html"`; `context()` |
| `index_first_then_tags_in_name_order`, `index_generated_without_tags` (`tag.rs`) | `.slug`, `.source`, `.template` | kinds `== [TagIndex, Tag(blog), Tag(rust)]`; labels `== ["tag index", "tag page 'blog'", "tag page 'rust'"]`; `template_name() == "tags.html"` |
| `golden_feed` + all feed tests (`feed.rs`) | `feed.path`, `.source`, `.contents` | kind `== Feed`; label `== "RSS feed"`; `path(dist) == dist/feed.xml`; `.text()` compared to the same golden string |
| `covers_every_item_kind_sorted` + all sitemap tests (`sitemap.rs`) | `sitemap.path`, `.source`, `.contents`, `RenderItem` | kind `== Sitemap`; label `== "sitemap"`; `path(dist) == dist/sitemap.xml`; `.text()`; same locs and XML |
| `copies_all_files_in_all_nested_directories`, `copy_failure_is_an_error_naming_the_path`, `plan_and_copy_follow_symlinked_files` (`assets.rs`) | `assets::copy` | `output::write(&output::render(&Tera::default(), &dir, plan(&src, Path::new("dest"))?)?)`, same file and content assertions |
| `plan_is_sorted_labelled_and_touches_nothing`, `plan_and_copy_follow_symlinked_files` (`assets.rs`) | `f.label`, `f.dest` | labels via `kind.to_string()` (same strings); `files[0].path(&dir) == dir/dest/a/b.css`; `!dir.join("dest").exists()` |
| `plan_rejects_symlinked_directory`, `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`, `plan_fails_on_missing_assets_folder` | `dest` argument meaning | Pass `Path::new("dest")`; `!dir.join("dest").exists()`; same assertions |

## Tasks
### T-1 Pin plan order, exact collision labels and failure precedence in-process
- Kind: safety-net
- Satisfies: AC-1.1, AC-1.2, AC-1.3, AC-1.4, AC-2.1, AC-2.2, AC-3.1, AC-3.2, AC-3.3, AC-3.5, AC-5.2, AC-7.2, AC-7.4, AC-9.2
- Files: `tests/plan.rs`
- Add these tests, written against the **current** code. They must pass before any other change.
  - `plan_enumerates_every_kind_in_order`, tagged `// AC-arch-2.2.1; AC-arch-2.2.2`. Setup: one page `a/b/c.md` (dated, tags `["rust","blog"]`), `base_url` set, and assets `style.css`, `img/logo.svg`, `z.txt`. Expected paths, exactly:
    - `a/b/c/index.html`, `a/index.html`, `a/b/index.html`, `index.html`, `tags/index.html`, `tags/blog/index.html`, `tags/rust/index.html`, `feed.xml`, `sitemap.xml`, `assets/img/logo.svg`, `assets/style.css`, `assets/z.txt`
    - the first 9 are `File` and the last 3 are `Copy`
  - `collision_labels_are_exact_for_reachable_kinds`, tagged `// AC-arch-2.3.1; AC-arch-2.3.2; AC-arch-2.3.3`. Full-message `assert_eq!` for each case:
    - (a) `tags/one.md` → `… both section index 'tags' and tag index`
    - (b) a page tagged `blog` with `opts.assets` = a folder named `tags` containing `blog/index.html` → `… '<out>/tags/blog/index.html' would be written by both tag page 'blog' and asset 'blog/index.html'`
    - (c) `sitemap.xml.md` with `base_url` → `… '<out>/sitemap.xml' would be written as a file by sitemap, but page 'sitemap.xml' needs it to be a directory for '<out>/sitemap.xml/index.html'`
  - `page_and_asset_collision_names_the_page_first`, tagged `// AC-arch-2.7.4`. Setup: `site/assets/x.md` and asset `x/index.html`. Expected: `General`, exactly `Mango Error: output path '<out>/assets/x/index.html' would be written by both page 'assets/x' and asset 'x/index.html'`.
  - `collision_is_reported_before_a_render_error`, tagged `// AC-arch-2.3.5`. Setup: `posts.md` + `posts/one.md` with `page.html` = `{{ missing.value }}`. Expected: a `General` collision error, not `Template`.
  - `first_render_error_in_output_order_is_reported`, tagged `// AC-arch-2.5.2`. Setup: broken `section.html` and `tag.html`, and a page tagged `rust`. Expected: `Template`, the message contains `section.html` and not `tag.html`.
- Done when: the new tests pass on the unchanged code and the gate is green.

### T-2 Pin commit-time I/O error paths
- Kind: safety-net
- Satisfies: AC-5.3, AC-5.4
- Files: `src/build/output.rs`, `src/build/generate/assets.rs`
- Add these tests against the current API:
  - `write_failure_names_the_path` (`// AC-arch-2.5.3`):
    - (a) the target path is an existing directory → `IoPath` with `path ==` that file path
    - (b) the parent is an existing file → `IoPath` with `path ==` the parent
  - `copy_failure_names_the_source_path` (`// AC-arch-2.5.4`): the destination is an existing directory → `IoPath` whose `path` equals `src/style.css`, not the destination
- Done when: both pass on the unchanged code and the gate is green.

### T-3 Replace the three output shapes with one `Output` model
- Kind: implementation
- Satisfies: AC-1.1, AC-1.2, AC-1.3, AC-1.4, AC-2.1, AC-2.2, AC-2.3, AC-2.4, AC-3.1, AC-3.2, AC-3.3, AC-3.4, AC-3.5, AC-4.1, AC-4.2, AC-4.3, AC-4.4, AC-5.1, AC-5.2, AC-5.3, AC-5.4, AC-6.1, AC-6.2, AC-6.3, AC-6.4, AC-6.5, AC-7.1, AC-7.2, AC-8.1, AC-8.2, AC-8.4, AC-9.1, AC-9.2, AC-9.3, AC-9.4, AC-9.5, AC-9.6
- Files: `src/build/output.rs`, `src/build/pipeline.rs`, `src/render/template.rs`, `src/build/generate/content.rs`, `src/build/generate/section.rs`, `src/build/generate/home.rs`, `src/build/generate/tag.rs`, `src/build/generate/feed.rs`, `src/build/generate/sitemap.rs`, `src/build/generate/assets.rs`
- Implement the Approach and the Interfaces above. This is one compile unit, so do it in a single task. Rewrite every affected unit test per the AC-9.5 table, keeping each test's name and tag. T-1/T-2 tests change only at their call sites; their assertions stay the same.
- Done when:
  - the gate passes
  - `grep -rn "RenderItem\|GeneratedFile\|AssetFile\|render_generated\|page_date\|RenderedFile" src/` returns nothing
  - `src/build/generate/assets.rs` has no `copy` fn
  - `git diff <branch_point> -- tests/build.rs` is empty
  - in `tests/plan.rs`, only T-1 additions differ
  - `src/lib.rs` and `Cargo.toml` are unchanged
  - no `#[allow` was added

### T-4 Unit tests for derived labels, paths and sitemap selection
- Kind: test
- Satisfies: AC-7.3, AC-8.3, AC-6.1, AC-6.4, AC-1.2, AC-1.3, AC-1.4, AC-3.1, AC-4.1, AC-4.3
- Files: `src/build/output.rs`, `src/build/generate/sitemap.rs`
- Add these tests:
  - `labels_derive_from_kind` (`// AC-arch-2.7.3`): exact label for each of the 8 kinds, with an undated and a dated page giving the same label. The asset is `Asset { folder: "assets", rel: Path::new("css").join("main.css") }` → `asset 'css/main.css'`.
  - `paths_derive_from_kind` (`// AC-arch-2.6.1`): `path(dist)` for each of the 8 kinds, per the Approach table.
  - `lists_only_html_kinds_and_dates_only_content_pages` (`// AC-arch-2.8.3`):
    - input: every HTML kind from `all_items`, plus a `Feed` output, a `Sitemap` output and an `Asset` output
    - expected: the locs equal exactly the 10 HTML locs of `covers_every_item_kind_sorted`
    - no loc contains `feed.xml`, `sitemap.xml` or `/assets/`
    - exactly one `<lastmod>`, and it is on `/posts/one/`
- Done when: the tests pass and the gate is green.

### T-5 Update CLAUDE.md for the single output model
- Kind: docs
- Satisfies: AC-10.1, AC-9.7
- Files: `CLAUDE.md`
- Edits:
  - Feed/sitemap bullets: "collision source" becomes "collision label". The sitemap's entries are "derived from the outputs' kinds (not the content)". `<lastmod>` comes from the date on the content-page kind (`OutputKind::Page`).
  - Pipeline paragraph: one `Vec<Output>` in the listed order. Collision check over that list. `render` to `RenderedOutput`s. `commit` writes text and copies assets in one pass.
  - Collisions bullet: "both labels".
  - Module map:
    - `render/template.rs` constructs the template `Output`s. The template names still live there.
    - `build/output.rs` holds `Output`/`OutputKind`/`Body`, `path`, `Display` labels, `check_collisions`, `render` and `write`.
    - `assets.rs` has `plan` only.
    - `feed.rs`/`sitemap.rs` return `Option<Output>`.
- `README.md` is not edited.
- Done when: `grep -n "RenderItem\|GeneratedFile\|AssetFile\|render_generated\|page_date\|collision source" CLAUDE.md` returns nothing, and `README.md` is unchanged.

### T-6 Backlog: ARCH-2 done, ARCH-5 updated, RISK-7 and RISK-8 added
- Kind: docs
- Satisfies: AC-10.2, AC-2.4, AC-5.4
- Files: `specs/_system/backlog.md`
- Edits:
  - ARCH-2: index status `done`, section header `· done`, and a **Landed.** note summarizing this design.
  - ARCH-5: record that the "`build` no longer imports its core type from `render`" bullet landed with ARCH-2 (`Output` lives in `build/output.rs`). Note that `render/template.rs` now imports `Output` from `build` to construct items, and that moving construction into `build/generate/*` removes that edge. Remove "the output type moved into `build/` (or replaced by ARCH-2's `Output`)" from the Proposal. View models, Tera loading, item construction and `add_content` stay open.
  - Add RISK-7, "Asset-copy errors name the source, not the destination" (low, S, `/ship-feature`, open). Describe AC-5.4 and why `copy_failure_is_an_error_naming_the_path` hides it. Propose naming the destination, or both paths.
  - Add RISK-8, "Content page order depends on the filesystem" (low, S, `/spec-feature`, open). Describe AC-2.4: enumeration order, write order and which page's render error comes first vary by filesystem, while output bytes do not. Propose sorting pages by slug in the loader.
  - If RISK-7 or RISK-8 are already taken, use the next free numbers.
- Done when: all four entries appear in the index and as their own sections, and the ARCH-2 row reads `done`.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-1, T-3 |
| AC-1.2 | T-1, T-3, T-4 |
| AC-1.3 | T-1, T-3, T-4 |
| AC-1.4 | T-1, T-3, T-4 |
| AC-2.1 | T-1, T-3 |
| AC-2.2 | T-1, T-3 |
| AC-2.3 | T-3 |
| AC-2.4 | T-3, T-6 |
| AC-3.1 | T-1, T-3, T-4 |
| AC-3.2 | T-1, T-3 |
| AC-3.3 | T-1, T-3 |
| AC-3.4 | T-3 |
| AC-3.5 | T-1, T-3 |
| AC-4.1 | T-3, T-4 |
| AC-4.2 | T-3 |
| AC-4.3 | T-3, T-4 |
| AC-4.4 | T-3 (superseded by AC-8.1: the date moves to the page kind) |
| AC-5.1 | T-3 |
| AC-5.2 | T-1, T-3 |
| AC-5.3 | T-2, T-3 |
| AC-5.4 | T-2, T-3, T-6 |
| AC-6.1 | T-3, T-4 |
| AC-6.2 | T-3 |
| AC-6.3 | T-3 |
| AC-6.4 | T-3, T-4 |
| AC-6.5 | T-3 |
| AC-7.1 | T-3 |
| AC-7.2 | T-1, T-3 |
| AC-7.3 | T-4 |
| AC-7.4 | T-1 |
| AC-8.1 | T-3 |
| AC-8.2 | T-3 |
| AC-8.3 | T-4 |
| AC-8.4 | T-3 |
| AC-9.1 | T-3 |
| AC-9.2 | T-1, T-3 |
| AC-9.3 | T-3 |
| AC-9.4 | T-3 |
| AC-9.5 | T-3 |
| AC-9.6 | T-3 |
| AC-9.7 | T-5 |
| AC-10.1 | T-5 |
| AC-10.2 | T-6 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-23 | 20260923-230744-arch-2-one-output-model | Initial design. One `Output { kind, body }` in `build/output.rs`. Location, label and sitemap entry derived from `OutputKind`. One list through collision check, render, enumeration and commit, with asset copies folded into `output::write`. Output construction stays in `render/template.rs` (ARCH-5). 6 tasks, T-1/T-2 safety-net. |
