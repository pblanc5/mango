<!-- Published by dev-pipeline run 20260925-234126-risk-6-backslash-file-names, stage spec v2, approved 2026-09-26T00:25:00Z. -->

# Backslashes in file names are rejected: Requirements

Spec ID: `risk-6`

## Summary
On Linux and macOS, a backslash is an ordinary character in a file name, but mango treats it as a folder separator. A single file named `a\b.md` is published at `/a/b/` and creates a section `a` that has no folder behind it, even though the file-name rule does not allow `\`. This change makes a backslash in any file or folder name under the site folder an invalid-file-name error, like any other character the rule does not allow. Nothing changes on Windows, where `\` is a real separator and cannot appear inside a name.

## Context
- **Who is affected:** site authors on Linux or macOS whose content has a file or folder name with a `\` in it. That is almost always an accident, such as a name typed or copied from a Windows path, or a file produced by a tool that escapes characters. Today the page is published somewhere the author didn't intend, and nothing reports it.
- **The rule it breaks:** the documented file-name rule (`README.md:76`, `CLAUDE.md` "File names are strict") allows only ASCII letters, digits, `-`, `_` and `.` in each segment. `\` is not in that list, but it never reaches the check, because it is rewritten to `/` first.
- **The "Strict inputs" convention** (`specs/constitution.md`, Conventions): bad file names fail the build with an error naming the file and the value, drafts included.
- **Backlog item:** RISK-6 in `specs/_system/backlog.md`. It offers two options: reject `\` where it is not a separator, or document the behavior. The maintainer chose rejection (Open question 1).
- **Related spec:** `specs/arch-3/requirements.md`. Its baseline AC-1.5 describes the backslash-to-`/` behavior, and its AC-7.7 required ARCH-3 to keep that behavior and pin it with a test tagged `// AC-arch-3.7.7`. ARCH-3 deferred the decision to this item (its Open question 2). This spec replaces both criteria (AC-7.6).
- **Related spec:** `specs/risk-4/requirements.md`. Its AC-5.3 and arch-3's AC-1.3 say frontmatter, key, date and tag errors are reported before a file-name error. This spec keeps that order.
- **Release:** the change ships in 0.2.0 together with RISK-4, which already makes that release breaking (AC-7.2).
- **Related items:** RISK-5 and RISK-7 are about symlinks and asset-copy errors, not file names. Asset file names have no character rules at all, and this spec does not add any (Out of scope).

## Current behavior

### REQ-1 [baseline] A page's slug is its site-relative path with `/` separators
Evidence: `src/content/slug.rs:28-37` (`Slug::from_content_path`), `src/content/slug.rs:40-62` (`Slug::parse`), `src/content/loader.rs:74-75`; tests `generate_slug_strips_site_prefix_and_extension`, `generate_slug_accepts_safe_names_and_rejects_others`, `generate_slug_error_message_is_exact`, `dot_only_file_names_are_rejected` (`src/content/slug.rs`), `build_fails_on_invalid_file_name_naming_file`, `build_fails_on_dot_only_file_name_keeping_output`, `build_generates_site_from_fixture` (`tests/build.rs`); `README.md:76`.
Status: inferred, confirm at approval
- AC-1.1 [baseline] WHEN a markdown file is loaded THE content loader SHALL derive the page's slug from the file's path relative to the site folder: its final extension removed, and its folders and file stem joined with `/` on every platform. For example, `site/posts/post_one.md` gives `posts/post_one` on Linux, macOS and Windows.
- AC-1.2 [baseline] IF a segment of the slug is empty or contains a character other than ASCII letters, digits, `-`, `_` or `.` THEN THE content loader SHALL fail the build with a `General` error reading `<file path>: invalid file name '<segment>': use only ASCII letters, digits, '-', '_' and '.'`. This applies to draft pages too, and the build fails before the output folder is cleaned.
- AC-1.3 [baseline] IF a non-empty segment of the slug consists only of `.` characters THEN THE content loader SHALL fail the build with a `General` error reading `<file path>: invalid file name '<segment>': a segment cannot consist only of dots`.
- AC-1.4 [baseline] WHEN a page has a frontmatter, key, date or tag error and also an invalid file name THE content loader SHALL report the frontmatter, key, date or tag error, not the file-name error.
- AC-1.5 [baseline] THE content loader SHALL check only the part of the path below the site folder. The site folder's own path is not part of any slug segment.

### REQ-2 [baseline] A backslash inside a name becomes a folder separator
Evidence: `src/content/slug.rs:34` (`.replace('\\', "/")`, applied before validation); test `backslash_in_stem_becomes_separator` (`src/content/slug.rs:199-203`, tagged `// AC-arch-3.7.7`); `specs/arch-3/requirements.md` AC-1.5 and AC-7.7; `specs/_system/backlog.md` RISK-6. Every example below was reproduced by running the current binary on Linux on 2026-09-25 (run 20260925-234126-risk-6-backslash-file-names).
Status: confirmed by running the current binary on Linux
All of REQ-2 is replaced by REQ-3.
- AC-2.1 [baseline] WHERE `\` is not a path separator (Linux, macOS) THE content loader SHALL replace every `\` in a site-relative file or folder name with `/` before the file-name check. As a result, a single file named `a\b.md` at the top of the site folder gets the slug `a/b`. It is published at `/a/b/`, and a section `a` is generated for it although no folder `a` exists. This behavior is replaced by AC-3.1.
- AC-2.2 [baseline] WHERE `\` is not a path separator THE content loader SHALL apply the same replacement to folder names. A file `x\y/p.md` gets the slug `x/y/p`, and sections `x` and `x/y` are generated. This behavior is replaced by AC-3.1 and AC-3.2.
- AC-2.3 [baseline] WHERE `\` is not a path separator, IF a file named `a\b.md` sits next to a file `a/b.md` THEN THE build SHALL fail with a collision error naming `page 'a/b'` twice, not with a file-name error. The same applies to `a\b.md` next to a top-level `a.md`, which collides as `page 'a'` against `section index 'a'`. This behavior is replaced by AC-3.6.
- AC-2.4 [baseline] WHERE `\` is not a path separator, IF the replacement produces an empty or dot-only segment THEN THE content loader SHALL reject the name with the AC-1.2 or AC-1.3 error, naming the segment after replacement. For example, a file named `\.md` is reported with the segment `''`, and a file named `..\x.md` with the segment `'..'`. Neither error names the actual file or folder name as it appears on disk. This behavior is replaced by AC-3.3.

## User stories
- As a site author on Linux or macOS, I want a file name with a backslash in it to fail the build, so that a page is never published at a URL and in a section I didn't create.
- As a site author, I want the error to show the name exactly as it is on disk, so that I can find the file and rename it.
- As a site author on Windows, I want nothing to change, so that my nested folders keep building as before.

## Requirements

### REQ-3 A backslash in a file or folder name is an invalid file name
A backslash inside a single file or folder name is treated like any other character the file-name rule does not allow. It is never a separator. On Windows, `\` separates folders and cannot appear inside a name, so this requirement has no effect there.
- AC-3.1 IF the name of a markdown file under the site folder, or of any folder between the site folder and that file, contains `\` THEN THE content loader SHALL fail the build with the `General` error of AC-1.2. It SHALL NOT treat the `\` as a folder separator.
- AC-3.2 WHEN AC-3.1 applies THE error SHALL name the file's path and the offending name exactly as it appears on disk, backslash included, without its extension if it is the file name. For example, a top-level file `a\b.md` gives `<file path>: invalid file name 'a\b': use only ASCII letters, digits, '-', '_' and '.'`, where `<file path>` ends in `a\b.md`, and a file `x\y/p.md` names the segment `'x\y'`.
- AC-3.3 WHEN a name contains `\` THE content loader SHALL report the AC-1.2 error for that name even if the rest of the name is empty or made only of dots. For example, `\.md` is reported with the segment `'\'` and `..\x.md` with the segment `'..\x'`. Neither gets the dot-only error of AC-1.3.
- AC-3.4 IF the page with the backslash in its name has `"draft": true` THEN THE content loader SHALL still fail the build.
- AC-3.5 WHEN a page has a frontmatter, key, date or tag error and also a backslash in its name THE content loader SHALL report the frontmatter, key, date or tag error, as in AC-1.4.
- AC-3.6 IF a file named `a\b.md` sits next to `a/b.md` or next to a top-level `a.md` THEN THE build SHALL fail with the file-name error of AC-3.1, not with a collision error.
- AC-3.7 THE content loader SHALL NOT reject a site whose site-folder path itself contains `\` (for example `--site 'my\site'` on Linux), as long as every name below the site folder is valid (AC-1.5).
- AC-3.8 WHEN the build fails because of AC-3.1 THE CLI SHALL print the error to stderr and exit with status 1.

### REQ-4 Failure is safe
- AC-4.1 IF the build fails because of AC-3.1 THEN THE build SHALL leave the previous contents of the output folder untouched.

### REQ-5 Nothing else changes
- AC-5.1 THE content loader SHALL keep AC-1.1: on every platform, including Windows, a page in nested folders gets a `/`-joined slug. On Windows, `site\posts\post_one.md` still gives `posts/post_one`.
- AC-5.2 WHEN the example site (`example/site`) is built THE build SHALL succeed, and its output SHALL be byte-identical to the output before this change, on Linux and on Windows.
- AC-5.3 THE content loader SHALL keep the error texts, variants and precedence of AC-1.2, AC-1.3 and AC-1.4 for every name that contains no `\`. This change adds no new error message.
- AC-5.4 THE change SHALL add no dependency.

### REQ-6 Tests
- AC-6.1 WHERE `\` is not a path separator THE test suite SHALL include a temp-site E2E test in which a site containing a file named `a\b.md` fails with exit status 1, stderr naming the file and `'a\b'`, and the previous output left intact (a `snapshot` test). It SHALL also cover a folder name containing `\` and a draft page.
- AC-6.2 WHERE `\` is not a path separator THE test suite SHALL include unit tests for AC-3.2 and AC-3.3 that check the exact message, including a top-level file, a folder name and a name made of `\` and dots.
- AC-6.3 WHERE `\` is a path separator (Windows) THE test suite SHALL include a unit test showing that a site-relative path written with `\` separators gives a `/`-joined slug (AC-5.1). This keeps the Windows half of the check that `backslash_in_stem_becomes_separator` makes today, which runs on both platforms.
- AC-6.4 THE test suite SHALL no longer contain a test pinning AC-2.1. The `// AC-arch-3.7.7` tag SHALL be removed with it, because that criterion is withdrawn (AC-7.6). New tests SHALL be tagged `// AC-risk-6.<req>.<m>`.
- AC-6.5 WHERE `\` is not a path separator THE test suite SHALL include a test showing that a site whose site-folder path contains `\`, and whose names below the site folder are all valid, builds successfully (AC-3.7).

### REQ-7 Documentation and backlog
- AC-7.1 THE user guide (`README.md`) file-name rule SHALL state that a backslash is not allowed in a file or folder name. It MAY add that on Windows `\` is simply the folder separator.
- AC-7.2 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]`, in the `### Changed` group, as an entry starting with `**Breaking:**`, worded for site authors: on Linux and macOS a file or folder name containing `\` now fails the build instead of being split into folders. The change ships in 0.2.0 together with RISK-4, so it needs no version bump beyond the one RISK-4 already requires.
- AC-7.3 THE developer guide (`CLAUDE.md`) SHALL no longer say that on Unix a `\` in a file name becomes a `/` (the "File names are strict" paragraph), and SHALL describe the rejection instead.
- AC-7.4 THE system overview (`specs/_system/overview.md`) SHALL no longer cite a removed test.
- AC-7.5 THE backlog SHALL mark RISK-6 `done`, in its summary table row and its section, with a note on how it was resolved, in the change that lands it. The pull request title SHALL end with `(RISK-6)`.
- AC-7.6 THE arch-3 requirements (`specs/arch-3/requirements.md`) SHALL mark AC-7.7 as withdrawn, with the date and `superseded by specs/risk-6/requirements.md`, and SHALL note on baseline AC-1.5 that it no longer describes current behavior. Their IDs and wording SHALL otherwise stay as they are.

## Out of scope
- Documenting the backslash behavior instead of rejecting it. That was the other option in the backlog item; the maintainer chose rejection (Open question 1).
- A dedicated error message or hint for `\` (Open question 3).
- Character rules for asset file names. Assets are copied under their own names and have no file-name rule. An asset named `a\b.css` on Linux is copied as `a\b.css`, but a collision error labels it `asset 'a/b.css'`. That label quirk is left as it is.
- The `site_relative` label in the symlinked-content-folder error, which also shows `\` as `/`. It is a label only and affects no output.
- Any other character in file names, and the error texts of AC-1.2 and AC-1.3.
- Collecting file-name errors across several pages. The build still stops at the first failing page.
- Windows reserved names (`CON`, `NUL`) and Windows case-insensitive collisions.

## Open questions
1. **Reject or document?** The backlog item offers both options. **Decided: reject the backslash (REQ-3).**
2. **Changelog wording.** A site with a `\` in a name builds today and would fail after this change. **Decided: the `Changed` group with a `**Breaking:**` prefix, shipping in 0.2.0 with RISK-4 (AC-7.2).**
3. **Error wording.** **Decided: reuse the AC-1.2 message unchanged; no new message (AC-5.3).**

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260925-234126-risk-6-backslash-file-names | New spec from backlog RISK-6. Records the current slug derivation and the backslash-to-`/` behavior as baseline. A `\` in a content file or folder name becomes an invalid-file-name error with the existing message, reported with the name exactly as on disk, drafts included and before cleaning; recorded as a breaking change for 0.2.0. Windows is unchanged. Supersedes `specs/arch-3/requirements.md` AC-1.5 and AC-7.7. |
