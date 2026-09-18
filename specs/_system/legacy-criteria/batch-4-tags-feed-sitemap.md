# Batch 4 - tags, RSS feed and sitemap

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260914-212907-tasks-md/01-plan.v1.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260914-212907-tasks-md`
- Groups defined here: `AC-1`, `AC-2`, `AC-3`, `AC-4`, `AC-5`, `AC-6`, `AC-7`, `AC-8`, `AC-9`
- Criteria: 43

## Requirements

### REQ-1 Strict tag validation
Tags are their own slugs. Anything other than `^[a-z0-9]+(-[a-z0-9]+)*$` fails the build, the same way dates do. Duplicate tags are removed.
- AC-1.1 `page::validate_tags` accepts values that match the pattern, for example `blog`, `static-site`, `a1`, `2026`, `a-b-c`.
- AC-1.2 It rejects each of these with `MangoError::Frontmatter`, and the message contains `'<value>'`: `""`, `"Rust"`, `"static site"`, `"c++"`, `"-rust"`, `"rust-"`, `"a--b"`, `"café"`, `"ünï"`. The message is `invalid tag '<tag>': expected lowercase ASCII letters and digits separated by single hyphens`.
- AC-1.3 Duplicates are removed and the first occurrence keeps its place: `["b","a","b","c","a"]` becomes `["b","a","c"]`.
- AC-1.4 `Page::new` calls the validation, so `Page.tags` holds the validated, deduplicated list. A page with no tags still gets an empty list.
- AC-1.5 E2E: a temp site page with `"tags": ["Rust"]` fails the build. stderr names the file and contains `'Rust'`, and stdout is empty.
- AC-1.6 E2E: the same failure happens when that page is `"draft": true`.

### REQ-2 Tag index and tag pages
A tag index and one page per tag are built from the draft-filtered pages. Both use the existing render and collision pipeline.
- AC-2.1 `build_tag_index(pages)` returns a `BTreeMap<String, Vec<PageSummary>>` with one entry per tag used. Each list is sorted with `compare_summaries`: newest first, undated pages included and last, ties broken by title and then slug.
- AC-2.2 The tag index render item has slug `tags`, source `tag index` and template `tags.html`. Its context `tags` is a list of `{ name, url, page_count }` sorted by name, where `url` is `/tags/<name>/`. It also gets `config`.
- AC-2.3 Each tag gets a render item with slug `tags/<tag>`, source `tag page '<tag>'` and template `tag.html`. Its context `tag` is `{ name, url, pages }`, where `pages` are `PageSummary` objects (`title`, `date`, `slug`, `url`) in AC-2.1 order. It also gets `config`.
- AC-2.4 The tag index item is always produced, even when no page has tags. Its `tags` list is then empty. Tag render items come out as the index first, then tag pages in name order.
- AC-2.5 In `cli::build`, tag items join `check_collisions` and `output::render` after pages, sections and home. They are written after cleaning, together with the other files.
- AC-2.6 E2E: a site with `tags/one.md` (a `tags/` folder) fails. stderr names `<dist>/tags/index.html`, `section index 'tags'` and `tag index`. The output folder is untouched (the marker file is still the only file).
- AC-2.7 E2E: a site with a top-level `tags.md` fails. stderr names `page 'tags'` and `tag index`. The output is untouched.
- AC-2.8 E2E: a site whose pages have no tags succeeds and writes `tags/index.html`.
- AC-2.9 E2E: if `tags.html` is removed from a private template copy, the rebuild fails. stderr mentions `tags.html`, and the previous output is intact (the same pattern as `missing_home_template_keeps_previous_output`).

### REQ-3 Tag links in the page context
- AC-3.1 In the `page.html` context, `page.tags` is a list of `{ name, url }` objects in frontmatter order, after deduplication. `url` is `/tags/<name>/`.
- AC-3.2 Summaries (`section.pages`, `home.recent`, `tag.pages`) do not get a `tags` key.

### REQ-4 `base_url` validation
- AC-4.1 `config::load` accepts `base_url` values that start with `http://` or `https://`, for example `https://example.com`, `https://example.com/` and `http://localhost:8080`. It also accepts a config where `base_url` is not set. Templates still receive the value exactly as written.
- AC-4.2 `config::load` rejects `""`, `example.com`, `ftp://example.com`, `//example.com` and `HTTPS://example.com` (the prefix check is case-sensitive). The error is `MangoError::Config` and its message contains the config path and `'<value>'`. Message format: `'<path>': invalid base_url '<value>': must start with http:// or https://`.
- AC-4.3 E2E: build a site successfully, then rebuild with a config whose `base_url` is `example.com`. The rebuild fails, stderr contains `Mango Config Error`, the config path and `example.com`, and `snapshot(out)` is unchanged.

### REQ-5 Generated non-HTML files in the pipeline
Files with an explicit relative path (the feed and the sitemap) go through the collision check. They are built in memory before cleaning and written after it.
- AC-5.1 `output::check_collisions` also checks a list of `GeneratedFile { path (relative to dist), source, contents }`. Render items are checked first, then generated files, using the same error format. A render item and a generated file that map to the same final path fail with both sources named.
- AC-5.2 Generated files become `RenderedFile`s at `<dist>/<path>` without touching the filesystem. They are written by the existing `output::write` after `clean_contents`.
- AC-5.3 Existing HTML path mapping, collision messages and render behavior do not change. All existing `output.rs` tests still pass after adjusting their call sites.

### REQ-6 RSS feed (`dist/feed.xml`)
- AC-6.1 When `base_url` is unset, `feed::build` returns `None`, so no feed is produced. The E2E build succeeds and writes no `feed.xml`.
- AC-6.2 When `base_url` is set, the result is `GeneratedFile { path: "feed.xml", source: "RSS feed", … }`. It is RSS 2.0:
  - `<channel>` holds `title` (`config.title`, or empty), `link` (`<base>/`) and `description` (`config.description`, or empty).
  - `<channel>` then holds one `<item>` per entry of the shared recent list, in that order.
- AC-6.3 Each `<item>` has these children, in order:
  - `title`
  - `link`: `<base><url>`
  - `<guid isPermaLink="true">` with the same value as `link`
  - `description`: the page's frontmatter description
  - `pubDate`: the date formatted as `%a, %d %b %Y 00:00:00 +0000`, for example `Sat, 24 Jan 2026 00:00:00 +0000`
- AC-6.4 `<base>` is `base_url` with trailing `/` characters removed. `https://example.com/` and `https://example.com` produce the same feed, and no URL contains `//` after the scheme.
- AC-6.5 Every text value, including links, goes through the XML escape helper. The helper maps `&` to `&amp;`, `<` to `&lt;`, `>` to `&gt;`, `"` to `&quot;` and `'` to `&apos;`, and leaves other text unchanged.
- AC-6.6 Undated pages never appear, and the number of items follows `recent_count` (including 0).
- AC-6.7 The recent list comes from one shared function that `home::build` and `feed::build` both call. `home.recent` behaves exactly as before, and existing home tests pass unchanged.
- AC-6.8 The output is deterministic. There is no `lastBuildDate` and no current time. For a known page set, the output equals one golden string exactly.

### REQ-7 Sitemap (`dist/sitemap.xml`)
- AC-7.1 When `base_url` is unset, no sitemap is produced. When it is set, the result is `GeneratedFile { path: "sitemap.xml", source: "sitemap", … }` using the sitemaps.org 0.9 `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">` format.
- AC-7.2 The sitemap has one `<url><loc>` per HTML render item passed in (home, pages, section indexes, tag index, tag pages), where `loc` is `<base>` plus `slug_url(item.slug)`. The list comes from the render items, not from walking the content again.
- AC-7.3 Entries are sorted by `loc`, compared as plain strings.
- AC-7.4 Only render items for dated content pages get `<lastmod>YYYY-MM-DD</lastmod>`. Undated pages, sections, home and tag items get none.
- AC-7.5 `loc` values are escaped, and a trailing slash on `base_url` does not produce `//`.

### REQ-8 Fixtures and fixture E2E
- AC-8.1 `test/mango.json` gains `"base_url": "https://example.com"`.
- AC-8.2 New `tag.html` and `tags.html` templates extend `base.html`. `page.html` lists tags as links. `base.html` gains a `/tags/` nav link and, only `{% if config.base_url %}`, `<link rel="alternate" type="application/rss+xml" href="/feed.xml">`.
- AC-8.3 `build_generates_site_from_fixture` also asserts that:
  - `tags/index.html`, `tags/blog/index.html`, `feed.xml` and `sitemap.xml` exist
  - `tags/blog/index.html` contains `href="/posts/post_one/"` and `href="/posts/post_two/"`
  - `posts/post_one/index.html` contains `href="/tags/blog/"`
  - `feed.xml` contains `<link>https://example.com/projects/mango/</link>`, and `Mango Task Tracker` appears before `Post One`
  - `sitemap.xml` contains `<loc>https://example.com/</loc>` and `<loc>https://example.com/tags/blog/</loc>`
- AC-8.4 E2E: a temp site built without a config writes neither `feed.xml` nor `sitemap.xml`.
- AC-8.5 E2E: a temp site built with a config that has a `base_url` writes both files. After `base_url` is removed from that config, a rebuild succeeds and both files are gone.

### REQ-9 Documentation and quality gates
- AC-9.1 `CLAUDE.md` documents:
  - the tag rules, tag URLs and the reserved `/tags/` path, including its collisions
  - the `tag` and `tags` contexts and the new shape of `page.tags`
  - `tag.html` and `tags.html` as required templates
  - the feed and sitemap: when they are written, what they contain, that they are generated in Rust, and their source labels
  - the `base_url` validation and trailing-slash trimming
  - the updated `build` pipeline order and the new modules in the module map
- AC-9.2 `cargo build`, `cargo test`, `cargo clippy --all-targets` and `cargo fmt --check` all pass with no new warnings. `Cargo.toml` dependencies do not change.

## Verified by

From the run's test report (`03-test.v1.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-1.1 | new | `content::page::tests::validate_tags_accepts_valid_values` (all 5 spec values) | pass |
| AC-1.2 | new | `validate_tags_rejects_invalid_values_naming_value` (all 9 values, `Frontmatter` variant, exact message) | pass |
| AC-1.3 | new | `validate_tags_removes_duplicates_keeping_order` (`["b","a","b","c","a"]` → `["b","a","c"]`) | pass |
| AC-1.4 | new | `new_rejects_invalid_tag` (also checks dedup through `Page::new`), `new_defaults_missing_date_and_tags` | pass |
| AC-1.5 | new | `build_fails_on_invalid_tag_naming_file_and_value` (file name, `'Rust'`; `assert_failure` asserts stdout is empty) | pass |
| AC-1.6 | new | `build_fails_on_invalid_tag_in_draft` | pass |
| AC-2.1 | new | `index::tag::tests::groups_pages_by_tag_sorted`, `undated_pages_sort_last` (tie on title broken by slug) | pass |
| AC-2.2 | new | `template::tests::tag_index_context_and_source` (slug, source, template, exact `tags` JSON, config) | pass |
| AC-2.3 | new | `template::tests::tag_page_context_and_source` (slug, source, template, `tag` shape, config) | pass |
| AC-2.4 | new | `generate::tag::tests::index_first_then_tags_in_name_order`, `index_generated_without_tags` | pass |
| AC-2.5 | new | `src/cli.rs` diff (tag items after home in the collision check, render and write after clean); `build_generates_site_from_fixture` | pass |
| AC-2.6 | new | `build_fails_on_tags_folder_collision` (path, both sources, output folder holds only `marker.txt`) | pass |
| AC-2.7 | new | `build_fails_on_top_level_tags_page_collision` | pass |
| AC-2.8 | new | `build_writes_tag_index_without_tags` | pass |
| AC-2.9 | new | `missing_tags_template_keeps_previous_output` (stderr mentions `tags.html`, snapshot unchanged) | pass |
| AC-3.1 | changed | `template::tests::page_context_tags_are_links` (order kept after dedup, exact JSON) | pass |
| AC-3.2 | new | `index::tag::tests::summaries_have_no_tags_key`, `tag_page_context_and_source` | pass |
| AC-4.1 | new | `config::tests::base_url_accepts_http_and_https` (3 values kept as written, plus unset) | pass |
| AC-4.2 | new | `config::tests::base_url_rejects_other_values_naming_file_and_value` (5 values, exact message) | pass |
| AC-4.3 | new | `build_fails_on_invalid_base_url_keeping_output` | pass |
| AC-5.1 | new | `output::tests::generated_file_collides_with_render_item` (item vs file, file vs file, exact message) | pass |
| AC-5.2 | new | `output::tests::render_generated_maps_paths_without_touching_fs`, `feed_and_sitemap_only_with_base_url` | pass |
| AC-5.3 | baseline | existing `output::tests` pass; diff shows only `&[]` and `page_date: None` call-site edits | pass |
| AC-6.1 | new | `feed::tests::no_feed_without_base_url`, `feed_and_sitemap_only_with_base_url` | pass |
| AC-6.2 | new | `feed::tests::golden_feed`, `unset_title_and_description_are_empty` | pass |
| AC-6.3 | new | `feed::tests::golden_feed`; generated fixture `feed.xml` inspected | pass |
| AC-6.4 | new | `feed::tests::trailing_slash_base_url_has_no_double_slash`, `config::tests::base_url_root_trims_trailing_slashes` | pass |
| AC-6.5 | new | `xml::tests::escapes_special_characters`, `leaves_other_text_unchanged`, `feed::tests::values_are_escaped` (links included) | pass |
| AC-6.6 | new | `feed::tests::undated_pages_excluded`, `item_count_follows_recent_count` (0, 1, 2, 10) | pass |
| AC-6.7 | changed / baseline | `home::tests::recent_is_dated_sorted_and_truncated` (unchanged), `recent_pages_matches_home_recent`; `feed.rs` calls `home::recent_pages` | pass |
| AC-6.8 | new | `feed::tests::golden_feed` (exact string, no `lastBuildDate`) | pass |
| AC-7.1 | new | `sitemap::tests::no_sitemap_without_base_url`, `covers_every_item_kind_sorted` (path, source, xmlns header) | pass |
| AC-7.2 | new | `sitemap::tests::covers_every_item_kind_sorted` (home, pages, sections, tag index, tag pages; count equals items) | pass |
| AC-7.3 | new | `covers_every_item_kind_sorted`, `sorted_by_loc_as_plain_strings` | pass |
| AC-7.4 | new | `sitemap::tests::lastmod_only_on_dated_pages`; fixture `sitemap.xml` has `lastmod` only on the 4 dated pages | pass |
| AC-7.5 | new | `sitemap::tests::escapes_loc`, `trailing_slash_base_url` | pass |
| AC-8.1 | new | `test/mango.json` diff; `build_generates_site_from_fixture` | pass |
| AC-8.2 | new | template diffs and new `tag.html`/`tags.html` (both extend `base.html`); fixture test asserts the RSS `<link rel="alternate">` and tag links | pass |
| AC-8.3 | new | `build_generates_site_from_fixture` (all listed assertions present; `page` is `posts/post_one/index.html`) | pass |
| AC-8.4 | new | `feed_and_sitemap_only_with_base_url` (build without config) | pass |
| AC-8.5 | new | `feed_and_sitemap_only_with_base_url` (with `base_url`, then removed: both files gone) | pass |
| AC-9.1 | new | `CLAUDE.md` inspection: tag rules and reserved `/tags/` with collisions, `tag`/`tags` contexts, new `page.tags`, required templates, feed and sitemap (conditions, contents, generated in Rust, source labels), `base_url` validation and trimming, pipeline order, module map | pass |
| AC-9.2 | new | the gate commands above (all exit 0, no warnings, no dependency diff) | pass |
