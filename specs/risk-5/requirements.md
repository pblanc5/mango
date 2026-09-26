<!-- Published by dev-pipeline run 20260926-004309-risk-5-broken-symlinks-in-site, stage spec v5, approved 2026-09-26T02:18:00Z. -->

# Unresolvable symlinks in the site folder fail the build: Requirements

Spec ID: `risk-5`

## Summary
Today, mango silently skips a symlink under the site folder when it can't resolve the link's target. This covers a dangling link and a chain of links that loops (`a.md -> b.md -> a.md`). A page the author expects can then be missing from the site, and so can a whole section if the link was its only page, while the build reports success. The assets folder already treats both cases as build errors. With this change the site folder does the same: an unresolvable entry fails the build with an error naming it, before the output folder is touched.

## Context
- **Who is affected:** site authors who use symlinks in their content, for example to share a markdown file between sites. If the link breaks because its target moved or was renamed, the page disappears from the output and nothing reports it.
- **Backlog item:** RISK-5 in `specs/_system/backlog.md`. It asks two things. First, should a broken or looping link under `site/` fail the build, or is a warning enough? Second, should the loader be aligned with `assets::plan`, or should the reason the two differ be recorded? The maintainer chose to fail the build, with no warning, and to align the loader with the assets folder, reusing its error.
- **Origin:** RISK-1 (`specs/_system/legacy-criteria/risk-1-symlinked-content-folders.md`) rejected symlinked content folders. It kept unresolvable links silently ignored, as baseline AC-11.6 and AC-11.7, and deferred the stricter behavior to this item because the change is visible to users.
- **Precedent:** RISK-2 made `assets::plan` fail on a dangling link or a link loop before the output folder is cleaned (`README.md:116`).
- **"Strict inputs" convention** (`specs/constitution.md`, Conventions): bad input fails the build with an error naming the file, before the output folder is cleaned.
- **Release:** a site that builds today can fail after this change, so the change is breaking. It can ship in 0.2.0 alongside RISK-4 and RISK-6, which already make that release breaking.
- **Related items:** RISK-8 (page order depends on the filesystem) decides which error is reported first when a site has several problems. Hidden entries under `site/` are a separate follow-up, to be added to the backlog after RISK-5 lands (see Out of scope).

## Current behavior

### REQ-1 [baseline] Symlinks under the site folder, and under the assets folder for contrast
Evidence: `src/content/loader.rs:25-60` (`traverse`; the skip is at `:40-42`), `README.md:117`, `CLAUDE.md` module map (`src/content/` entry); tests `load_rejects_symlinked_directory`, `load_rejects_symlink_loop_to_ancestor`, `load_follows_symlinked_markdown_file`, `load_ignores_symlinked_non_markdown_file`, `load_accepts_symlinked_site_root`, `load_ignores_dangling_symlink`, `load_ignores_symlink_loop_chain` (`src/content/loader.rs`), `build_fails_on_symlinked_content_folder_keeping_output`, `build_reads_symlinked_markdown_file` (`tests/build.rs`); for the assets folder, `src/build/generate/assets.rs:63` and tests `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`; legacy criteria AC-11.1 to AC-11.8.

Manual check: on 2026-09-25 the orchestrator ran the current binary on Linux and confirmed AC-1.1 and AC-1.2 exactly. A dangling `.md` link, an `a.md` <-> `b.md` loop and a dangling link named like a folder were all skipped, and the build exited 0. The same dangling link under the assets folder failed with `Mango I/O Error at 'assets/x.css': No such file or directory (os error 2)`.

Status:
- AC-1.1 and AC-1.2: confirmed by the orchestrator's run of the current binary on Linux (2026-09-25).
- AC-1.3 to AC-1.8: inferred, confirm at approval.

- AC-1.1 [baseline] IF an entry under the site folder is a symlink whose target does not exist THEN THE content loader SHALL skip it: no page, no error, and the other pages load as usual. This holds whatever the link's name, whether it ends in `.md` or not. If the link was the only file in its folder, that folder's section doesn't appear in the output either, and nothing is reported.
- AC-1.2 [baseline] IF an entry under the site folder is part of a symlink chain that loops (`a.md -> b.md`, `b.md -> a.md`) THEN THE content loader SHALL skip it: no page, no error, and the load terminates.
- AC-1.3 [baseline] IF resolving an entry under the site folder fails for any other reason (for example, the entry is removed between listing the folder and inspecting it) THEN THE content loader SHALL skip it in the same way.
- AC-1.4 [baseline] IF an entry under the site folder is a symlink that resolves to a folder THEN THE content loader SHALL fail the build with a `General` error containing `content folder '<path relative to the site folder>'` and `symlink to a directory`, before the output folder is cleaned.
- AC-1.5 [baseline] WHEN an entry under the site folder is a symlink that resolves to a markdown file THE content loader SHALL read it as a page, with its URL taken from the link's path.
- AC-1.6 [baseline] WHEN an entry under the site folder is a symlink that resolves to a file that is not markdown THE content loader SHALL ignore it, as it ignores any other file that is not markdown.
- AC-1.7 [baseline] WHEN the site folder passed with `--site` is itself a symlink to a folder THE content loader SHALL load the pages inside it.
- AC-1.8 [baseline] IF an entry under the assets folder is a dangling symlink or part of a looping chain THEN THE asset planner SHALL fail the build with an I/O error naming the entry's path and the operating system's cause, before the output folder is cleaned.

## User stories
- As a site author, I want a broken symlink in my content to fail the build, so that a page never silently disappears from my site.
- As a site author, I want the error to name the broken link, so that I can find it and fix or remove it.
- As a site author, I want content and assets to follow one rule for broken links, so that I don't have to remember two.
- As a site author, I want symlinks that work today (to markdown files, to other files, and a symlinked `--site` folder) to keep working unchanged.

## Requirements

### REQ-2 An unresolvable entry under the site folder fails the build
Every entry the content loader finds under the site folder, at any depth, must resolve. The build fails; a warning is not enough (confirmed by the maintainer). This replaces AC-1.1, AC-1.2 and AC-1.3.
- AC-2.1 IF an entry under the site folder, at any depth, is a symlink whose target does not exist THEN THE content loader SHALL fail the build.
- AC-2.2 IF an entry under the site folder, at any depth, is part of a symlink chain that loops THEN THE content loader SHALL fail the build, and the build SHALL terminate.
- AC-2.3 THE content loader SHALL apply AC-2.1 and AC-2.2 whatever the entry's name. A link fails the same way whether it is named like a markdown file (`broken.md`), like any other file (`broken.txt`), has no extension, or is hidden (a name starting with `.`, such as an Emacs lock file `.#post.md`). No name is exempt, just as in the assets folder.
- AC-2.4 IF resolving an entry under the site folder fails for any other reason THEN THE content loader SHALL fail the build rather than skip the entry.
- AC-2.5 WHEN the build fails because of AC-2.1, AC-2.2 or AC-2.4 THE error SHALL be the same kind of I/O error that the asset planner reports in AC-1.8, of the form `Mango I/O Error at '<path>': <OS cause>`, and SHALL give the operating system's cause. The path SHALL be the path at which the walk reached the entry, starting from the site folder as given with `--site`, and SHALL name the link itself, not its target. The path is not shortened to a form relative to the site folder; that form stays specific to the folder-rejection error in AC-1.4.
- AC-2.6 WHEN the build fails because of AC-2.1, AC-2.2 or AC-2.4 THE CLI SHALL print the error to stderr and exit with status 1.

### REQ-3 Failure is safe
- AC-3.1 IF the build fails because of REQ-2 THEN THE build SHALL leave the previous contents of the output folder untouched.

### REQ-4 Nothing else changes
- AC-4.1 THE content loader SHALL keep AC-1.4, AC-1.5, AC-1.6 and AC-1.7 unchanged: a symlinked folder is still rejected with the same message, a symlinked markdown file is still read, a symlink to any other file is still ignored, and a symlinked `--site` folder still works.
- AC-4.2 THE asset planner SHALL keep AC-1.8 unchanged, including its error.
- AC-4.3 WHEN an entry under the site folder whose name starts with `.` resolves THE content loader SHALL handle it exactly as it does today. For example, a hidden markdown file such as `site/.notes.md` is still published at `/.notes/`.
- AC-4.4 WHEN the example site (`example/site`) is built THE build SHALL succeed, and its output SHALL be byte-identical to the output before this change.
- AC-4.5 THE change SHALL add no dependency.

### REQ-5 Tests
AC-2.4 gets no dedicated test. It is covered only through the dangling-link and loop tests below, since AC-2.1 and AC-2.2 are instances of it.
- AC-5.1 WHERE the platform supports symlinks in tests (Unix) THE test suite SHALL include unit tests showing that a dangling link fails the load with an error naming the link, both for a name ending in `.md` and for one that does not (AC-2.1, AC-2.3, AC-2.5).
- AC-5.2 WHERE the platform supports symlinks in tests THE test suite SHALL include a unit test showing that a looping chain fails the load with an error naming one of the links in the chain (AC-2.2).
- AC-5.3 WHERE the platform supports symlinks in tests THE test suite SHALL include a unit test showing that a dangling link in a nested folder fails the load (AC-2.1, "at any depth").
- AC-5.4 WHERE the platform supports symlinks in tests THE test suite SHALL include a temp-site E2E test showing that a build of a site containing a dangling link exits with status 1, prints an error naming the link to stderr, and leaves the previous output intact (a `snapshot` test) (AC-2.6, AC-3.1).
- AC-5.5 THE test suite SHALL no longer contain the tests pinning AC-1.1 and AC-1.2 (`load_ignores_dangling_symlink`, `load_ignores_symlink_loop_chain`), and their `// AC-11.6` and `// AC-11.7` tags SHALL be removed with them, since those criteria are superseded. New tests SHALL be tagged `// AC-risk-5.<req>.<m>`.

### REQ-6 Documentation and backlog
- AC-6.1 THE user guide (`README.md`) SHALL say that a broken link or a link loop under `site/` is a build error, as it is under `meta/assets/`, and SHALL no longer say that it is ignored.
- AC-6.2 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]`, in the `### Changed` group, as an entry that starts with `**Breaking:**` and is worded for site authors: a broken symlink or a symlink loop under the site folder now fails the build instead of being skipped silently.
- AC-6.3 THE developer guide (`CLAUDE.md`) SHALL describe the new behavior. It SHALL no longer say that the loader skips an entry whose target can't be resolved, or that this deliberately differs from `assets::plan`.
- AC-6.4 THE system overview (`specs/_system/overview.md`) SHALL no longer list RISK-5 as open, and SHALL no longer cite a removed test.
- AC-6.5 In the change that lands RISK-5, THE backlog SHALL mark RISK-5 `done`, in both its summary table row and its section, with a note on how it was resolved. The backlog's Workflow value for RISK-5 SHALL read `/spec-feature`, in both the summary table row and the section's header line, since this change is validation strictness per the constitution and was run through `/spec-feature`. The pull request title SHALL end with `(RISK-5)`.
- AC-6.6 THE legacy criteria file for RISK-1 (`specs/_system/legacy-criteria/risk-1-symlinked-content-folders.md`) SHALL note that AC-11.6 and AC-11.7 are superseded by `specs/risk-5/requirements.md`. Their recovered wording SHALL stay as it is.

## Out of scope
- A warning instead of an error. The maintainer chose to fail the build. mango has no warning output today: stderr is used only for the error that fails the build.
- Exempting some names from REQ-2, such as editor lock files (`.#<name>`) or other hidden names. The maintainer chose no exemption, as in the assets folder (AC-2.3).
- Hidden entries under `site/` (names starting with `.`). Today a hidden markdown file such as `site/.notes.md` is published at `/.notes/` (the orchestrator confirmed this with the current binary). Skipping hidden entries would also avoid Emacs `.#<name>` lock files. That is a separate follow-up, to be added to the backlog after RISK-5 lands. This spec doesn't change how hidden files are handled (AC-4.3).
- A dedicated error message for broken links, such as one that says "broken symlink" or "symlink loop". The maintainer chose to reuse the asset planner's I/O error (AC-2.5). A clearer message would also change the assets error, so it would be a separate change.
- Collecting several errors in one report. The build still stops at the first failing entry. When a site has several problems, the one reported first depends on the order in which the filesystem lists entries, which is RISK-8.
- Cycles among real folders (bind mounts, Windows junctions), which RISK-1 already left out of scope.
- The `clean` command, which does not read the site folder.
- A broken symlink passed as `--site` itself. It already fails with the existing "is not a directory" error (for example `./brokensite is not a directory`).

## Open questions
None

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260926-004309-risk-5-broken-symlinks-in-site | New spec from backlog RISK-5. Records the current symlink handling under the site folder as baseline. From now on, a dangling link, a looping chain or any other entry that can't be resolved under the site folder fails the build before cleaning. The error is the same I/O error the assets folder gives, and it names the link. Names starting with `.` get no exemption, and hidden-file handling is unchanged. The change is recorded as breaking. Supersedes legacy AC-11.6 and AC-11.7. |
