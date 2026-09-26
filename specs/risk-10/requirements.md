<!-- Published by dev-pipeline run 20260926-163650-risk-10-skip-hidden-entries, stage spec v1, approved 2026-09-26T16:48:12Z. -->

# Hidden files and folders are skipped: Requirements

Spec ID: `risk-10`

## Summary
Today mango publishes every markdown file under the site folder, including hidden ones. `site/.notes.md` becomes `/.notes/`, and `site/.drafts/secret.md` becomes `/.drafts/secret/` with a section index at `/.drafts/`. Hidden files under the assets folder, such as `.DS_Store`, are copied into `dist/assets/`. Authors often keep scratch notes and tool folders under names that start with `.` and expect them to stay private, and most static site generators skip such names. With this change, mango skips any file or folder whose name starts with `.`, under both the site folder and the assets folder, without reading or checking it. Hidden entries then never reach the output and never fail the build. That also covers the Emacs lock files (`.#<name>`) that RISK-5 made fatal.

## Context
- **Who is affected:** site authors who keep hidden files or folders next to their content, whether on purpose (scratch notes, `.drafts/`) or through tools: editor lock and swap files, `.obsidian/`, `.git/` when the site folder is its own repository, and `.DS_Store` written by macOS Finder. The repository is public, and authors copying its layout get the same exposure.
- **Backlog item:** RISK-10 in `specs/_system/backlog.md`. It asks the spec to settle four choices: (1) skip hidden files, hidden folders, or both; (2) whether names starting with `_` are skipped too; (3) whether the rule also applies under `meta/assets/`; (4) whether a skipped entry is checked at all. This spec proposes an answer to each (REQ-3 to REQ-5). All four are repeated under Open questions for confirmation at approval.
- **Done when (from the backlog):** the rule is implemented and tested, `load_publishes_hidden_markdown_file` is replaced by a test of the new behavior, RISK-5's Emacs lock-file note is revisited, and `README.md` and `CHANGELOG.md` say which names are skipped.
- **Related specs:**
  - `specs/risk-5/requirements.md`. AC-2.3 ("no name is exempt", naming `.#post.md`) and AC-4.3 (hidden entries handled as before) are superseded for hidden names by this spec.
  - `specs/arch-3/requirements.md`. REQ-8 rejects file-name segments made only of dots, such as `...md`. Every such name starts with `.`, so under this spec those files are skipped before any file-name check (see AC-1.4, AC-3.5 and Open question 5).
- **"Strict inputs" convention** (`specs/constitution.md`, Conventions): bad input fails the build. This spec narrows what counts as input: hidden entries are no longer input at all.
- **Release:** pages that are published today stop being published, and their URLs disappear from the output, the feed and the sitemap. Under **Releasing** in the constitution that is a breaking change. Before 1.0 it bumps the minor version.

## Current behavior

### REQ-1 [baseline] Hidden entries under the site folder
Evidence: `src/content/loader.rs:25-87` (`traverse` walks every entry and applies no name filter), `src/content/slug.rs:30-60` (`.` is an allowed character in any segment, and only a segment made entirely of dots is rejected); tests `load_publishes_hidden_markdown_file`, `load_fails_on_dangling_symlink` (its `.#post.md` case), `dot_only_file_name_is_rejected_even_for_drafts` (`src/content/loader.rs`), `build_fails_on_dot_only_file_name_keeping_output` (`tests/build.rs`); backlog RISK-10.

Status:
- AC-1.1 and AC-1.2: confirmed with the binary on 2026-09-25 (recorded in backlog RISK-10). AC-1.1 is also pinned by `load_publishes_hidden_markdown_file`.
- AC-1.3: the symlink cases are pinned by `load_fails_on_dangling_symlink`. The rest is inferred, confirm at approval.
- AC-1.4: pinned by `dot_only_file_name_is_rejected_even_for_drafts` and `build_fails_on_dot_only_file_name_keeping_output`.
- AC-1.5: inferred, confirm at approval.

- AC-1.1 [baseline] WHEN a markdown file whose name starts with `.` is under the site folder and is not a draft THE content loader SHALL publish it like any other page. For example, `site/.notes.md` is published at `/.notes/`.
- AC-1.2 [baseline] WHEN a folder whose name starts with `.` is under the site folder THE content loader SHALL walk it like any other folder. For example, `site/.drafts/secret.md` is published at `/.drafts/secret/`, and the build writes a section index at `/.drafts/` that lists it.
- AC-1.3 [baseline] THE content loader SHALL validate a hidden entry exactly as it validates any other entry. A hidden markdown file with missing or malformed frontmatter, an unknown key, a bad date or tag, or an invalid file name fails the build, drafts included. A hidden symlink to a folder fails with the `content folder '<rel>' is a symlink to a directory` error. A hidden dangling symlink or looping chain, such as the Emacs lock file `.#post.md`, fails with the I/O error of RISK-5 AC-2.5.
- AC-1.4 [baseline] IF a markdown file's name is made only of dots before its extension (`...md`, `..md`) THEN THE content loader SHALL fail the build with the error `<file>: invalid file name '<segment>': a segment cannot consist only of dots` (arch-3 AC-8.1), drafts included, before the output folder is cleaned.
- AC-1.5 [baseline] WHEN a hidden page is published THE build SHALL treat it like any other page: it appears in its section's listing, in the home page's recent pages when dated, on the pages of its tags, in the feed and in the sitemap.

### REQ-2 [baseline] Hidden entries under the assets folder
Evidence: `src/build/generate/assets.rs:41-80` (`collect` walks every entry and applies no name filter); tests `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`, `plan_rejects_symlinked_directory` (none of them uses a hidden name).
Status: inferred, confirm at approval.

- AC-2.1 [baseline] WHEN a file whose name starts with `.` is under the assets folder THE asset planner SHALL copy it to the same relative path under the assets output folder. For example, `meta/assets/.DS_Store` is copied to `dist/assets/.DS_Store`.
- AC-2.2 [baseline] WHEN a folder whose name starts with `.` is under the assets folder THE asset planner SHALL walk it and copy its files like any other folder's.
- AC-2.3 [baseline] IF a hidden entry under the assets folder is a dangling symlink, part of a looping chain or a symlink to a folder THEN THE asset planner SHALL fail the build exactly as it does for any other name.

## User stories
- As a site author, I want files and folders whose names start with `.` to stay out of my published site, so that scratch notes and tool folders are never made public by accident.
- As a site author, I want editor lock files, swap files and OS clutter (`.#post.md`, `.DS_Store`) to be ignored, so that the build neither fails on them nor copies them into my output.
- As a site author, I want one rule for content and assets, so that I don't have to remember two.
- As a site author, I want to keep building from a site or assets folder whose own path has a hidden name (`--site .content`, or a project under `~/.local/`), so that where I keep my project doesn't matter.

## Requirements

### REQ-3 What counts as hidden
A single rule, the same for the site folder and the assets folder and on every platform (proposed; Open questions 1 and 2).
- AC-3.1 THE content loader and THE asset planner SHALL treat an entry as hidden when its own name, as listed in its parent folder, starts with `.`. Examples: `.notes.md`, `.drafts`, `.git`, `.DS_Store`, `.#post.md`.
- AC-3.2 THE content loader and THE asset planner SHALL decide whether an entry is hidden from its own name only. The extension, the contents, whether the entry is a file, folder or symlink, and a symlink's target play no part. Neither do operating-system attributes such as the Windows "hidden" attribute. A hidden symlink to a visible file is hidden. A visible symlink (`notes.md`) to a hidden file (`.secret.md`) is not hidden, and is handled as it is today.
- AC-3.3 THE content loader and THE asset planner SHALL NOT treat a name that starts with any other character as hidden, `_` included. For example, `site/_notes.md` is still published at `/_notes/` (proposed; Open question 2).
- AC-3.4 THE content loader SHALL NOT skip the site folder given with `--site`, and THE asset planner SHALL NOT skip the assets folder given with `--assets`, whatever their own names or the names of the folders above them. The rule applies only to entries found inside them. For example, `--site .content`, `--site .` and a site under `~/.local/share/blog/site` build as they do today.
- AC-3.5 THE content loader SHALL treat a markdown file whose name is made only of dots before its extension (`...md`, `..md`) as hidden, since its name starts with `.`. Such a file is skipped under REQ-4 and no longer fails the build with the error of AC-1.4 (Open question 5).

### REQ-4 Hidden entries under the site folder are skipped
Replaces AC-1.1 to AC-1.5 for hidden entries.
- AC-4.1 WHEN a hidden file is under the site folder, at any depth, THE content loader SHALL skip it. It produces no page, and it does not appear in any section listing, the home page, any tag page, the tag index, the feed or the sitemap.
- AC-4.2 WHEN a hidden folder is under the site folder, at any depth, THE content loader SHALL skip the folder and everything inside it, at any depth, including entries whose own names are not hidden. The folder gets no section index. For example, `site/.drafts/secret.md` publishes nothing, and no `/.drafts/` section exists.
- AC-4.3 THE content loader SHALL NOT inspect a skipped entry at all. It does not read it, resolve it if it is a symlink, parse its frontmatter or check its date, tags, keys or file name. So a hidden entry SHALL NOT fail the build. The following all build successfully: a hidden dangling symlink (the Emacs lock file `.#post.md`), a hidden looping chain, a hidden symlink to a folder, a hidden markdown file with no or malformed frontmatter, and a hidden folder containing `my notes.md` (proposed; Open question 3).
- AC-4.4 WHEN the site folder contains hidden entries THE build output SHALL be byte-identical to the output of the same site with those entries removed. This includes the case where a folder holds only hidden entries: with them removed it is an empty folder, so it gets no section today and gets none here either.
- AC-4.5 THE build SHALL NOT report skipped entries. A build whose only unusual input is hidden entries exits with status 0 and writes nothing to stderr.

### REQ-5 Hidden entries under the assets folder are skipped
The same rule as REQ-4 (proposed; Open question 4). Replaces AC-2.1 to AC-2.3 for hidden entries.
- AC-5.1 WHEN a hidden file is under the assets folder, at any depth, THE asset planner SHALL skip it, and it SHALL NOT be copied into the output. For example, `meta/assets/.DS_Store` does not produce `dist/assets/.DS_Store`.
- AC-5.2 WHEN a hidden folder is under the assets folder, at any depth, THE asset planner SHALL skip the folder and everything inside it, at any depth, including entries whose own names are not hidden.
- AC-5.3 THE asset planner SHALL NOT inspect a skipped entry at all. A hidden dangling symlink, a hidden looping chain or a hidden symlink to a folder under the assets folder SHALL NOT fail the build.
- AC-5.4 WHEN the assets folder contains hidden entries THE build output SHALL be byte-identical to the output with those entries removed.

### REQ-6 Nothing else changes
- AC-6.1 THE content loader and THE asset planner SHALL handle every entry that is not hidden exactly as before. That includes the RISK-5 failure on a visible dangling or looping symlink, the RISK-1 and RISK-2 rejection of a visible symlink to a folder, file-name validation and draft handling.
- AC-6.2 IF the build fails because of an entry that is not hidden THEN THE build SHALL still leave the previous contents of the output folder untouched.
- AC-6.3 WHEN the example site (`example/site`, `example/meta`, `example/mango.json`) is built THE build SHALL succeed, and its output SHALL be byte-identical to the output before this change, including after the hidden fixture entries of AC-7.6 are added.
- AC-6.4 THE change SHALL add no dependency.

### REQ-7 Tests
New tests are tagged `// AC-risk-10.<req>.<m>`.
- AC-7.1 THE test suite SHALL include unit tests showing that the content loader skips a hidden markdown file at the top level and in a nested folder, and skips a hidden folder with all its contents, including a file inside it with a visible name (AC-4.1, AC-4.2). They SHALL also show that a markdown file whose name starts with `_` is still loaded (AC-3.3).
- AC-7.2 THE test suite SHALL include unit tests showing that a hidden entry under the site folder that would fail the load if it were visible does not fail it: a hidden markdown file with malformed frontmatter, a hidden folder containing a file with an invalid name, and a `...md` file (AC-4.3, AC-3.5). WHERE the platform supports symlinks in tests (Unix), they SHALL also cover a hidden dangling symlink named `.#post.md`, a hidden looping chain and a hidden symlink to a folder.
- AC-7.3 THE test suite SHALL include unit tests showing that the asset planner does not plan a hidden file or the contents of a hidden folder (AC-5.1, AC-5.2). WHERE the platform supports symlinks in tests, they SHALL also show that a hidden dangling symlink under the assets folder does not fail the plan (AC-5.3).
- AC-7.4 THE test suite SHALL include a test showing that a site folder and an assets folder whose own names start with `.` are loaded and planned normally (AC-3.4).
- AC-7.5 THE test suite SHALL include a temp-site E2E test in `tests/build.rs` in which a site with a hidden file, a hidden folder and a hidden asset (and, where the platform supports symlinks in tests, a dangling `.#post.md` symlink) builds with exit status 0 and empty stderr. Its output SHALL be byte-identical to the build of the same site without those entries (AC-4.4, AC-4.5, AC-5.4).
- AC-7.6 THE example site SHALL contain at least one hidden markdown file and one hidden folder holding a markdown file under `example/site`, and at least one hidden file under `example/meta/assets`, all realistic and all valid. The manifest in `build_generates_site_from_fixture` SHALL stay unchanged, and a `fixture_*` test SHALL assert that no output path has a segment starting with `.`.
- AC-7.7 THE test suite SHALL no longer contain `load_publishes_hidden_markdown_file`, and its `// AC-risk-5.4.3` tag SHALL be removed with it. `load_fails_on_dangling_symlink` SHALL no longer include the `.#post.md` case, and its visible cases SHALL stay. The loader and E2E tests that expect `...md` to fail (`dot_only_file_name_is_rejected_even_for_drafts`, `build_fails_on_dot_only_file_name_keeping_output`) SHALL be replaced by tests of AC-3.5. The unit tests of the slug type's own dot-only check (arch-3 AC-7.4) SHALL stay.

### REQ-8 Documentation and backlog
- AC-8.1 THE user guide (`README.md`) SHALL say, in its Content section, that files and folders whose names start with `.` are skipped under `site/` and `meta/assets/` at any depth, together with everything inside a hidden folder, and are not checked. It SHALL also say that names starting with `_` are not skipped. It SHALL no longer say that a name made only of dots, such as `...md`, is rejected. Its Known limitations lines on symlinks SHALL no longer say "whatever the link's name" without noting that hidden links are skipped.
- AC-8.2 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]` in the `### Changed` group, as an entry that starts with `**Breaking:**` and is written for site authors. It SHALL say that hidden files and folders under the site folder are no longer published, that hidden files under the assets folder are no longer copied, that a hidden broken symlink such as an editor lock file no longer fails the build, and that a page must be renamed if it should stay published.
- AC-8.3 THE developer guide (`CLAUDE.md`) SHALL describe the rule where it describes the pipeline, file-name rules, the loader and the asset planner. It SHALL no longer say that an unresolvable entry fails the load "whatever its name" without the hidden-name exception.
- AC-8.4 THE system overview (`specs/_system/overview.md`) SHALL describe the hidden-entry rule in its `traverse` and assets risk rows and SHALL NOT cite a removed test.
- AC-8.5 THE RISK-5 requirements (`specs/risk-5/requirements.md`) SHALL note that AC-2.3 (for hidden names) and AC-4.3 are superseded by `specs/risk-10/requirements.md`. The arch-3 requirements (`specs/arch-3/requirements.md`) SHALL note that AC-8.1 and AC-8.2 no longer apply to files found by the content loader, since dot-only names are hidden. Both notes SHALL keep the original criteria's wording, and each spec SHALL get a Changelog row.
- AC-8.6 In the change that lands RISK-10, THE backlog SHALL mark RISK-10 `done`, in both its summary table row and its section, with a note on how it was resolved. The pull request title SHALL end with `(RISK-10)`.

## Out of scope
- The templates folder. Which files the template loader picks up is not changed or examined here.
- A warning or a list of skipped entries. mango has no warning output, and the skip is silent (AC-4.5).
- A way to opt back in to publishing or copying a hidden entry, such as an allow-list or a config field.
- Ignore files or patterns (for example a `.mangoignore`), and skipping any other kind of name, such as `~` backup files or `#name#` autosave files, which do not start with `.`.
- Files whose names start with `.` that a site author might want served from the site root, such as `.well-known/` or `.htaccess`. mango has no way to place a file at the root of the output today, and this spec doesn't add one.
- The `clean` command, which reads neither folder.
- The order in which pages are loaded (RISK-8).

## Open questions
1. **Hidden files, hidden folders, or both?** Proposed: both, with a hidden folder skipped together with everything inside it, whatever those entries are named (AC-4.2, AC-5.2). Skipping only files would still publish `site/.drafts/secret.md`, which the backlog names as an exposure.
2. **Are names starting with `_` skipped too?** Proposed: no (AC-3.3). `_` is an allowed file-name character, and a page such as `site/_notes.md` publishes at `/_notes/` today. Skipping it would silently remove pages authors may rely on, and some other generators give `_` a different meaning (for example, a section's own page) rather than "private".
3. **Is a skipped entry checked at all?** Proposed: no (AC-4.3, AC-5.3). A hidden entry is not read, resolved or validated, so a hidden broken symlink, such as the Emacs lock file `.#post.md`, no longer fails the build. This reverses RISK-5 AC-2.3 for hidden names. The alternative is to keep resolving hidden entries and fail on broken ones, which keeps the Emacs lock-file failure.
4. **Does the rule also apply under the assets folder?** Proposed: yes (REQ-5), so that `.DS_Store` and editor files are not copied. Consequence: a hidden asset that an author does want copied (for example `meta/assets/.htaccess` or `meta/assets/.well-known/`) stops being copied. Such files land under `/assets/` today, not at the site root, so they rarely do their usual job anyway.
5. **Dot-only file names such as `...md` (suspected side effect).** These names start with `.`, so under AC-3.1 they are hidden and skipped. They no longer get the arch-3 AC-8.1 error; they are simply not published. The hazard arch-3 guarded against, output written outside the page's folder, cannot occur for a skipped file. Proposed: accept this and note it in the arch-3 spec (AC-3.5, AC-8.5). The alternative is to keep rejecting a dot-only name before the hidden-name rule applies.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-26 | 20260926-163650-risk-10-skip-hidden-entries | New spec from backlog RISK-10. Records the current handling of hidden entries under the site and assets folders as baseline. From now on, files and folders whose names start with `.` are skipped under both folders, at any depth, without being read or checked. They are no longer published or copied, and they can no longer fail the build. Names starting with `_` are not affected. Supersedes RISK-5 AC-2.3 (for hidden names) and AC-4.3, and takes dot-only file names out of arch-3 REQ-8's reach. Breaking. |
