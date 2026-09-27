<!-- Published by dev-pipeline run 20260927-191208-risk-8-sort-pages-by-slug, stage spec v2, approved 2026-09-27T19:26:34Z. -->

# Content page order does not depend on the filesystem: Requirements

Spec ID: `risk-8`

## Summary
Today mango's content loader returns pages in whatever order the operating system lists directory entries. That order is not sorted and differs between filesystems. Output bytes don't depend on it, but three other things do: the order in which the build plan lists and writes content pages, which page's error is reported when several page templates fail to render, and which collision is reported when several exist. Because the loader stops at the first bad file, the listing order also decides which error a site author sees when several content files are bad. With this change, pages are ordered by slug and bad content files are checked in a fixed path order, so the same site gives the same plan and the same error on every system.

## Context
- **Who is affected:** site authors with more than one bad file who build on different machines, or on CI and locally. Today each machine may report a different error first, and a fix-and-rebuild loop can jump between files unpredictably. Also affected: tests and future callers of the library seam, such as the dev server (FEAT-1), that enumerate `BuildPlan::outputs()`.
- **Backlog item:** RISK-8 in `specs/_system/backlog.md`. It proposes sorting pages by slug in the loader "so enumeration and error precedence are deterministic too". The backlog lists it as a `/spec-feature` change because it changes which error a user sees first.
- **Related specs:**
  - `specs/arch-2/requirements.md`: AC-2.4 records the unsorted page order as baseline and points to RISK-8 (its Open question 4). AC-2.1 item 1 ("content pages, in the order the content loader returns them"), AC-3.2, AC-3.3 and AC-5.2 define precedence by plan order, so they inherit the new order.
  - `specs/risk-5/requirements.md` (Out of scope) and `specs/design` of RISK-5 say that when a site has several problems, the one reported first depends on filesystem listing order, "which is RISK-8".
  - `specs/risk-10/requirements.md` (Out of scope) lists "the order in which pages are loaded (RISK-8)".
- **Release:** no output file changes and no build that succeeds today fails. Only the choice among errors that would fail the build anyway changes. Under **Releasing** in the constitution this is not breaking, but a site author can see it, so it gets a changelog line (REQ-7).

## Current behavior

### REQ-1 [baseline] Page order and what depends on it
Evidence: `src/content/loader.rs:17-27` (`load` returns pages in the order `traverse` pushed them), `src/content/loader.rs:29-96` (`traverse` iterates `fs::read_dir` unsorted, descends into each folder when it meets it, and returns the first error with `?`), `src/build/pipeline.rs:105,127,142-143`, `src/build/generate/content.rs:6-11`, `src/build/output.rs:141-180` (`check_collisions`: the first colliding pair in list order), `src/build/output.rs:185-202` (`render`: the first template error in list order), `src/build/index/section.rs:47`, `src/build/index/tag.rs:24`, `src/build/generate/home.rs:19`, `src/build/generate/sitemap.rs:30`, `src/build/generate/assets.rs:31` (every listing and the asset list are sorted); tests `first_render_error_in_output_order_is_reported`, `collision_labels_are_exact_for_reachable_kinds`, `plan_enumerates_every_kind_in_order` (`tests/plan.rs`, each with a single content page), `fixture_build_is_deterministic` (`tests/build.rs`). The loader unit tests sort slugs before comparing them (the `slugs` helper in `src/content/loader.rs`), so no test depends on page order. Recorded as arch-2 AC-2.4.
Status: inferred, confirm at approval

- AC-1.1 [baseline] THE content loader SHALL return pages in the order it meets them. Within each folder it meets entries in the order the operating system lists them, which is not sorted, and it walks a subfolder's contents at the point where it meets the subfolder. So page order can differ between filesystems and between two copies of the same site.
- AC-1.2 [baseline] IF more than one entry under the site folder would fail the load THEN THE content loader SHALL stop at the first failing entry it meets in the order of AC-1.1 and report only that entry's error. Which error is reported can therefore differ between filesystems. This covers every load failure: missing or malformed frontmatter, non-object frontmatter, key errors, type errors, bad dates, tags or file names, a symlinked folder, an unresolvable entry, and a folder that cannot be read. Drafts are included, and hidden entries are skipped (RISK-10).
- AC-1.3 [baseline] THE planning phase SHALL place content pages first in the plan, in the loader's order (arch-2 AC-2.1 item 1). `BuildPlan::outputs()` and the order in which the commit phase writes content pages follow that order.
- AC-1.4 [baseline] IF templates fail to render for more than one content page THEN THE planning phase SHALL report the error for the first of those pages in the loader's order (arch-2 AC-5.2).
- AC-1.5 [baseline] IF the plan has more than one output collision THEN THE collision check SHALL report the first one in plan order (arch-2 AC-3.2, AC-3.3). When content pages take part in more than one collision, which collision is reported can therefore depend on the loader's order.
- AC-1.6 [baseline] THE build output SHALL NOT depend on page order. Section listings, the home page's recent pages, tag pages, the tag index, the feed and the sitemap are all sorted. Two content files with the same slug (such as `a.md` and `a.markdown`) fail the collision check with a message that is the same whichever file is met first (`both page 'a' and page 'a'`).

## User stories
- As a site author, I want the same bad site to report the same error on my laptop, on CI and on Windows, so that I can reproduce and fix build failures one at a time.
- As a site author with several broken files, I want errors to come in a predictable order, so that fixing one file and rebuilding moves me forward instead of jumping around.
- As a developer of mango, or of a tool that calls the build plan, I want the plan to list outputs in the same order on every system, so that tests and callers can rely on it.

## Requirements

### REQ-2 Content pages are ordered by slug
Replaces AC-1.1 and AC-1.3 for the order of pages. Resolves arch-2 AC-2.4.
- AC-2.1 THE content loader SHALL return pages sorted by slug, in the byte-wise order of the slug's `/`-joined text. This is the order section indexes already use: `a-c` comes before `a/b`, and `B` before `a`. The order does not depend on the order in which the operating system lists entries.
- AC-2.2 THE planning phase SHALL place content pages first in the plan in the order of AC-2.1, followed by the other outputs in the existing order of arch-2 AC-2.1 items 2 to 8. `BuildPlan::outputs()` SHALL list content pages in that order, and the commit phase SHALL write them in that order.
- AC-2.3 WHEN the same site is planned from two copies whose folders list their entries in different orders THE build plan SHALL enumerate the same outputs, with the same contents, in the same order.
- AC-2.4 IF two content files have the same slug (such as `a.md` and `a.markdown`) THEN THE build SHALL still fail with the existing collision error, whose text SHALL be the same whichever file the filesystem lists first.

### REQ-3 Errors among content pages are chosen by slug
- AC-3.1 IF templates fail to render for more than one content page THEN THE planning phase SHALL report the render error for the page whose slug comes first in the order of AC-2.1.
- AC-3.2 IF the plan has more than one output collision THEN THE collision check SHALL report the first one in plan order, as arch-2 AC-3.2 and AC-3.3 already specify. With content pages in the order of AC-2.1, the collision reported SHALL be the same on every filesystem.
- AC-3.3 THE planning phase SHALL keep the existing order of stages and the existing precedence between them. Content loading comes before the config, templates and assets, and the collision check comes before rendering (arch-2 AC-3.5). Only the choice among errors of the same stage changes.

### REQ-4 Load errors are chosen by path
Replaces AC-1.2 for which error is reported (proposed; Open questions 1 and 2).
- AC-4.1 IF more than one entry under the site folder would fail the load THEN THE content loader SHALL report the error for the entry whose path relative to the site folder comes first. Paths are compared segment by segment, and each segment (a file or folder name) is compared byte-wise. A path that is a prefix of another comes first. So within a folder, entries are considered in byte-wise name order, and a subfolder's contents take the subfolder's place. Examples:
  - `b.md` and `a/z.md` are both bad: `a/z.md` is reported.
  - `a-c.md` and `a/b.md` are both bad: `a/b.md` is reported, because the folder name `a` comes before `a-c.md`. This differs from slug order, where `a-c` comes before `a/b` (Open question 2).
  - `posts/` is a symlink to a folder and `posts-old.md` has bad frontmatter: the `posts` symlink error is reported.
- AC-4.2 THE content loader SHALL still stop at the first failing entry in the order of AC-4.1 and report exactly one error. Errors are not collected or combined.
- AC-4.3 THE rule of AC-4.1 SHALL apply to every load failure listed in AC-1.2, drafts included. A folder that cannot be read takes the folder's own position in the order.
- AC-4.4 WHEN one file has more than one problem THE content loader SHALL keep the existing precedence between that file's own errors (for example, frontmatter errors before file-name errors, arch-3 AC-1.3, pinned by the test `frontmatter_error_wins_over_invalid_file_name`; and risk-9 REQ-4).
- AC-4.5 THE content loader SHALL keep skipping hidden entries without resolving, reading or checking them (RISK-10). A hidden entry SHALL NOT be reported, whatever its position in the order.
- AC-4.6 THE order of AC-4.1 SHALL be exact for names made only of ASCII characters, which covers every valid file name. For other names, which fail the load with the invalid-file-name error, the order SHALL still be the same on every run on a given platform.

### REQ-5 Nothing else changes
- AC-5.1 WHEN the example site (`example/site`, `example/meta`, `example/mango.json`) is built THE build SHALL succeed and produce output byte-identical to the output before this change. The manifest in `build_generates_site_from_fixture` and every `fixture_*` test SHALL pass with no change to their expectations.
- AC-5.2 THE build SHALL keep every error text, error variant and exit status. A failing build SHALL still leave the previous contents of the output folder untouched.
- AC-5.3 THE order of every output other than content pages SHALL stay as it is: section indexes, home, tag index, tag pages, feed, sitemap, then asset copies (arch-2 AC-2.1 items 2 to 8). The contents of every section listing, the home page, tag pages, the feed and the sitemap SHALL be unchanged.
- AC-5.4 THE asset planner SHALL behave as before, including which asset error is reported when several assets are bad (Out of scope; Open question 3).
- AC-5.5 THE library's public surface SHALL stay exactly the seven re-exports, and `PlannedOutput` SHALL keep its variants and fields.
- AC-5.6 THE change SHALL add no dependency.

### REQ-6 Tests
New tests are tagged `// AC-risk-8.<req>.<m>`. Each test of new behavior SHALL fail without the change on at least one filesystem that CI uses (Open question 5).
- AC-6.1 THE test suite SHALL include a unit test showing that the content loader returns pages in the exact order of AC-2.1, for a site whose slug order differs from the order the files were created in and from the segment-by-segment order of AC-4.1 (for example, pages `a-c.md`, `a/b.md` and `b.md`). The test SHALL compare the loaded order without sorting it first.
- AC-6.2 THE test suite SHALL include an in-process test in `tests/plan.rs` showing that `BuildPlan::outputs()` lists several content pages in the order of AC-2.1, ahead of the section indexes (AC-2.2).
- AC-6.3 THE test suite SHALL include a test in which a page template fails with an error that differs per page, and the reported error is the one for the page whose slug comes first (AC-3.1).
- AC-6.4 THE test suite SHALL include unit tests showing that, when several entries under the site folder would fail the load, the error reported is for the first in the order of AC-4.1. They SHALL cover at least two bad files in one folder, and a bad file inside a subfolder competing with a bad file whose name sorts after the subfolder's name (AC-4.1).
- AC-6.5 THE test suite SHALL include a temp-site E2E test in `tests/build.rs` in which two content files are bad. The build SHALL exit with status 1, its stderr SHALL name the file chosen by AC-4.1 and SHALL NOT name the other, and a `snapshot` check SHALL show that the previous output is untouched.
- AC-6.6 Every existing test in `tests/build.rs` and `tests/plan.rs` SHALL pass with no change to an expected string, path, order or status.
- AC-6.7 The tests of AC-6.1 and AC-6.2 SHALL also serve as the tests for AC-2.3 and SHALL carry the tag `// AC-risk-8.2.3` as well as their own. Each asserts one exact order for a site whose files were created in an order different from it, so the enumeration they check does not depend on the order in which the folders list their entries.

### REQ-7 Documentation and backlog
- AC-7.1 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]` in the `### Changed` group, as one line for site authors that is not marked breaking. It SHALL say that when several content files have errors, the one reported is now chosen by path, the same on every system (Open question 4).
- AC-7.2 THE user guide (`README.md`) SHALL say, in its Safe builds section, that a build stops at the first error, and that when several content files have errors the one reported is the first by path.
- AC-7.3 THE developer guide (`CLAUDE.md`) SHALL describe the page order (by slug) and the order of load errors (AC-4.1) where it describes the loader and the plan's output order.
- AC-7.4 THE system overview (`specs/_system/overview.md`) SHALL no longer list RISK-8 as open in its `traverse` risk row, and SHALL describe the new order there.
- AC-7.5 THE arch-2 requirements (`specs/arch-2/requirements.md`) SHALL note that AC-2.4 is superseded by `specs/risk-8/requirements.md`, keeping AC-2.4's original wording, with a Changelog row.
- AC-7.6 In the change that lands RISK-8, THE backlog SHALL mark RISK-8 `done`, in both its index row and its section, with a note on how it was resolved. The pull request's title SHALL end with `(RISK-8)`.

## Out of scope
- Reporting several errors at once. The build still stops at the first error (AC-4.2).
- The order in which the asset planner reports errors when several assets are bad. It also depends on listing order today (`assets::collect` fails before the list is sorted). See Open question 3.
- The order in which templates are loaded from the templates folder.
- Any change to the order or contents of section listings, the home page, tag pages, the tag index, the feed or the sitemap. These are already sorted.
- The `clean` command.

## Open questions
1. **Is load-error order in scope?** The backlog item names page order, plan enumeration and render-error precedence. Sorting pages after loading doesn't change which *load* error is reported, because the loader stops at the first bad file it meets. RISK-5 and RISK-10 both defer that question to RISK-8. Proposed: in scope (REQ-4), since for site authors it's the most visible part of the problem. The alternative is to drop REQ-4 and leave load-error order as in AC-1.2.
2. **Two orders or one?** Pages are ordered by slug text (AC-2.1, matching section indexes), and load errors by path segment by segment (AC-4.1). The two orders disagree only when a name contains a character that sorts before `/`, such as `-` or `.`, next to a folder with a shorter name: `a-c.md` against `a/b.md`. Proposed: accept the difference. Segment-by-segment order is "each folder's entries in name order, subfolders in place", which is easy to explain, and a failing entry may have no valid slug. The alternative is to order load errors by the full relative path as text, which would match slug order.
3. **Asset errors too?** Which asset error is reported when several assets are bad (a symlinked folder, a dangling link) also depends on listing order. Proposed: out of scope here, with a note in the backlog, since the item is about content. The alternative is to apply the same rule as AC-4.1 to the assets folder in this change.
4. **Changelog and README lines.** Nothing that builds today changes, but a site author can see which error is reported. Proposed: a non-breaking `Changed` line (AC-7.1) and one sentence in Safe builds (AC-7.2). The alternative is to treat it as internal, with no changelog entry.
5. **Tests that fail without the change.** The Definition of done asks for tests that fail without the change. The old order is the filesystem's, which on some filesystems happens to be sorted or creation order. Proposed: AC-6.1 and AC-6.4 choose names and creation orders so that neither creation order, reverse order nor per-folder name order matches the expected result where that is possible, and the design records which CI filesystems (ext4 on Linux, NTFS on Windows) are shown to fail without the change. NTFS lists names sorted, so on Windows some of these tests may pass without the change.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-27 | 20260927-191208-risk-8-sort-pages-by-slug | New spec from backlog RISK-8. Records the unsorted page order and its effects as baseline. Content pages are now ordered by slug, so plan enumeration, write order, render-error precedence and collision precedence are the same on every filesystem. When several content files are bad, the error reported is for the first by path, compared segment by segment. Supersedes arch-2 AC-2.4. Not breaking. |
