<!-- Published by dev-pipeline run 20260925-234126-risk-6-backslash-file-names, stage design v1, approved 2026-09-26T00:45:00Z. -->

# Backslashes in file names are rejected: Design

Spec ID: `risk-6` · Requirements: `specs/risk-6/requirements.md`

## Overview
`Slug::from_content_path` currently rewrites every `\` in the site-relative path to `/` before validating it. The fix makes it rewrite only the characters that are path separators on the current platform: `std::path::is_separator` is `/` on Unix and macOS, and `/` or `\` on Windows. On Linux and macOS a `\` inside a name then stays inside its segment, and the unchanged `Slug::parse` rejects it with the existing charset error, naming the segment exactly as it is on disk. On Windows the result is identical to today. This is a one-expression change in production code. The rest of the work is tests (unit and E2E, mostly `#[cfg(unix)]`, plus one `#[cfg(windows)]` unit test), docs, and spec bookkeeping.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/slug.rs` | `from_content_path`: `.replace('\\', "/")` becomes `.replace(std::path::is_separator, "/")`. The doc comment says "the platform's path separators written as `/`". Remove the test `backslash_in_stem_becomes_separator` (`// AC-arch-3.7.7`). Add a `#[cfg(windows)]` separator test, `#[cfg(unix)]` exact-message tests, and a site-prefix safety-net test. |
| `src/content/loader.rs` (tests only) | Safety net: a site folder with `\` in its own path loads (`#[cfg(unix)]`). New `#[cfg(unix)]` tests: a draft with `\` in a folder name fails; a frontmatter error wins over a `\` name. |
| `tests/build.rs` (tests only) | New `#[cfg(unix)]` E2E tests: rejection with exit 1, exact stderr, and the previous output kept (`snapshot`) for a top-level file, a folder and a draft; file-name error instead of a collision; a site folder path containing `\` builds. |
| `README.md` | The file-name rule (line 76) says `\` is not allowed, and that on Windows it is the folder separator. |
| `CHANGELOG.md` | A `**Breaking:**` bullet under `## [Unreleased]` / `### Changed`. |
| `CLAUDE.md` | "File names are strict": replace "On Unix a `\` in a file name becomes a `/` (RISK-6)." with the rejection rule. |
| `specs/_system/overview.md` | The `src/content/slug.rs` row (line 28) no longer cites `backslash_in_stem_becomes_separator`. |
| `specs/_system/backlog.md` | RISK-6 marked `done` in the table (line 33) and the section (line 323), with a resolution note. |
| `specs/arch-3/requirements.md` | AC-7.7 (line 135) marked withdrawn; AC-1.5 (line 45) annotated as no longer current. |

## Approach
**The change.** In `src/content/slug.rs`, `from_content_path` becomes:

```rust
let text = path
    .strip_prefix(site)
    .map_err(|_e| MangoError::General("unable to generate slug from path".into()))?
    .with_extension("")
    .to_string_lossy()
    .replace(is_separator, "/");
```

with `use std::path::{Path, PathBuf, is_separator}` (or the fully qualified `std::path::is_separator`; either is fine, `cargo fmt` decides the import order).

Why this satisfies every criterion:
- **Unix and macOS (AC-3.1 to AC-3.3).** `is_separator` is true only for `/`, so the call replaces `/` with `/` and changes nothing. A name `a\b` stays one segment, `a\b`. `Slug::parse` already rejects any segment with a byte outside `[A-Za-z0-9._-]`, so the error is the existing AC-1.2 text, naming `'a\b'` (AC-3.2). The charset check runs **before** the dot-only check in `Slug::parse` (lines 42-58). A segment containing `\` is non-empty and has a disallowed byte, so `\` and `..\x` get the charset error and never the dot-only one (AC-3.3). The stems are `\` and `..\x` because `with_extension("")` only strips the last `.ext`; see Evidence.
- **Drafts and precedence (AC-3.4, AC-3.5).** Unchanged by construction. `Page::new` validates the date, then the tags, then the slug (`src/content/page.rs:41-43`), and `loader::load` filters drafts only after `traverse` succeeds (`loader.rs:20-22`).
- **Collisions (AC-3.6).** `plan` loads the pages first, and the collision check runs later over the output list (CLAUDE.md pipeline order). So a failing load returns before any collision is checked, whatever order `read_dir` returns the entries in.
- **Site-folder path (AC-3.7, AC-1.5).** `strip_prefix(site)` removes the site path by components before any text is produced, so a `\` in the site path never reaches a segment. This already works today and is pinned first by T-1.
- **Windows (AC-5.1).** `is_separator` is true for both `/` and `\`. Today's `.replace('\\', "/")` leaves `/` as `/` and turns `\` into `/`, and the new call does the same, so the output is byte-for-byte the same function on Windows.
- **Error texts (AC-5.3).** No message, variant or check order changes. For a name without `\`, Unix output is identical to before, because the old replacement had nothing to replace.
- **Output (AC-5.2).** The fixture has no `\` in any name, so the slug text of every fixture page is unchanged on both platforms.

**Only the content slug changes.** `loader::site_relative` (the symlinked-folder label) and asset labels keep their `\` to `/` rewrite. Both are explicitly out of scope.

**Evidence for external behavior** (Rust 1.92.0 source, `~/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/`, the version pinned in `rust-toolchain.toml`):
- `std::path::is_separator(c)` is `c.is_ascii() && is_sep_byte(c as u8)`: `std/src/path.rs:255-257`.
- `is_sep_byte` is `b == b'/'` on Unix (`std/src/sys/path/unix.rs:6-8`) and `b == b'/' || b == b'\\'` on Windows (`std/src/sys/path/windows.rs:50-52`). The choice is made in `std/src/sys/path/mod.rs:1-28`: `target_os = "windows"` selects `windows`, and the `_` arm (Linux, macOS and the other Unixes) selects `unix`.
- `str::replace<P: Pattern>` (`alloc/src/str.rs:268`) accepts any `F: FnMut(char) -> bool` as a pattern (`core/src/str/pattern.rs:942-944`), so the `fn(char) -> bool` item `is_separator` can be passed directly.
- `with_extension("")` strips only the final extension: `_with_extension` (`std/src/path.rs:3030-3053`) keeps the path up to the extension, and `_set_extension` (`std/src/path.rs:1621-1645`) truncates after the file stem. The stem comes from `rsplit_file_at_dot` (`std/src/path.rs:308-329`), which splits at the last `.`. So `\.md` gives `\`, `..\x.md` gives `..\x`, and `a\b.md` gives `a\b`. On Unix the file name is the whole `a\b.md`, because `\` is not a separator byte there (`unix.rs:6-8`). This code path is unchanged by the design; the T-2 exact-message tests pin the results.
- `Path::display` formats the `OsStr` as lossy UTF-8 with no escaping (`std/src/path.rs:3575-3579`), so `\` appears as-is in the message. T-2 and T-4 assert exact text.

## Interfaces and data
None. No public or crate-internal signatures change, and no error variant, message, template context or output path changes for valid input.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Build the text from `Path::components()`, joined with `/` | `components()` normalizes: it drops interior `.` components and returns `..` as `Component::ParentDir`, so extra mapping code is needed to keep AC-1.3's dot-only errors exactly as they are (AC-5.3). That is more code and more risk than a one-expression change. |
| `#[cfg(windows)]` around the existing `.replace('\\', "/")` | Correct, but it puts the platform rule in a `cfg` instead of asking std what a separator is, and it creates two code paths, one of which clippy on Linux never sees. `is_separator` is a single path, checked by clippy on both CI platforms. |
| `.replace(std::path::MAIN_SEPARATOR, "/")` | Correct for paths from `read_dir`, but on Windows it would miss a `/` inside a relative path. That is harmless today, but `is_separator` states the rule exactly: any separator of this platform. |
| Pre-check the raw file and folder names for `\` with a dedicated error | Adds a new message, which the requirements rule out (AC-5.3, Open question 3). |
| Document the behavior instead | Rejected by the maintainer (Open question 1). |

## Risks
- **Windows-only test is only compiled on Windows.** The `#[cfg(windows)]` unit test (T-2) is not compiled, linted or run by the local gate on Linux. Only the Windows CI job checks it. Keep it trivially simple (it reuses the existing `slug()` helper and `assert_eq!`), and treat a red Windows CI job as a blocker before merge.
- **Cygwin** treats `\` as a separator (`std/src/sys/path/cygwin.rs:9-11`), so it behaves like Windows. It is not a CI or release target; this is noted only for completeness.
- **Users with `\` names on Linux or macOS** now get a build failure. This is intended and recorded as breaking (AC-7.2).
- **Test path construction.** The new tests are `#[cfg(unix)]`, where `site.join("a\\b.md")` is one segment. Expected paths must still be built one segment per `join` (`site.join("x\\y").join("p.md")`), following the CLAUDE.md convention.

**Unverified assumptions:** That macOS filesystems (APFS) accept `\` in a file name. std treats macOS as Unix (verified above, `sys/path/mod.rs` `_` arm), but neither CI nor the local gate runs on macOS, so the `#[cfg(unix)]` tests have never run there. The claim that a Linux filesystem accepts `\` in a name is proven first by T-1's on-disk loader test, which creates a folder named `my\site`.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` (constitution, Commands).
- Safety net:
  - Baseline criteria in the changed code that existing tests already cover:
    - AC-1.1: `generate_slug_strips_site_prefix_and_extension`, `loads_pages_nested_two_levels_deep`, `build_generates_site_from_fixture` (on Linux and Windows CI).
    - AC-1.2: `generate_slug_accepts_safe_names_and_rejects_others`, `generate_slug_error_message_is_exact`, `empty_and_bad_segments_get_the_charset_message`, `invalid_file_name_is_an_error_naming_the_file_even_for_drafts`, `build_fails_on_invalid_file_name_naming_file`.
    - AC-1.3: `dot_only_segments_get_their_own_message`, `dot_only_file_names_are_rejected`, `build_fails_on_dot_only_file_name_keeping_output`.
    - AC-1.4: `frontmatter_error_wins_over_invalid_file_name`, `key_errors_win_over_date_tag_and_file_name_errors`.
  - AC-1.5 is covered only implicitly, because every loader test uses an absolute site path. **T-1** pins it directly, including a site path containing `\` (AC-3.7), and passes on the unchanged code.
  - The baseline criteria AC-2.1 to AC-2.4 are replaced by REQ-3, so they get no safety net. Their pinning test is removed in T-2 (AC-6.4).
- New and changed behavior:
  - `src/content/slug.rs`:
    - `backslash_in_name_is_an_invalid_file_name` (unix): AC-3.1, AC-3.2, AC-3.3, AC-6.2.
    - `backslash_separates_folders_on_windows` (windows): AC-5.1, AC-6.3.
    - `site_folder_path_is_not_part_of_any_segment` (T-1): AC-1.5, AC-3.7.
  - `src/content/loader.rs`:
    - `backslash_in_folder_name_is_rejected_even_for_drafts` (unix): AC-3.4.
    - `frontmatter_error_wins_over_backslash_in_name` (unix): AC-3.5.
    - `load_accepts_backslash_in_site_folder_path` (unix, T-1): AC-3.7.
  - `tests/build.rs`:
    - `build_fails_on_backslash_in_file_name_keeping_output` (unix): AC-3.1, AC-3.2, AC-3.4, AC-3.8, AC-4.1, AC-6.1.
    - `backslash_name_fails_before_collision_check` (unix): AC-3.6.
    - `build_accepts_backslash_in_site_folder_path` (unix): AC-3.7, AC-6.5.
  - AC-5.2, AC-5.3 and AC-5.4 are covered by every existing test passing **unchanged**: the fixture manifest, `fixture_build_is_deterministic`, `fixture_internal_links_resolve`, and all file-name tests. No fixture file and no `Cargo.toml`/`Cargo.lock` change. The Tester should also build `example/` at `branch_point` and at HEAD and compare the two `dist` trees byte for byte (AC-5.2).

## Tasks
### T-1 Pin that the site folder's own path is never validated
- Kind: safety-net
- Satisfies: AC-1.5, AC-3.7
- Files: `src/content/slug.rs`, `src/content/loader.rs`
- Done when:
  - `slug.rs` gains `site_folder_path_is_not_part_of_any_segment`, tagged `// AC-risk-6.1.5, AC-risk-6.3.7`. `Slug::from_content_path(&Path::new("my site").join("a+b").join("posts").join("one.md"), &Path::new("my site").join("a+b"))` gives `posts/one` on all platforms. A `#[cfg(unix)]` block in the same test does the same with the site `my\site`: `Path::new("my\\site")` and file `Path::new("my\\site").join("posts").join("one.md")` give `posts/one`.
  - `loader.rs` gains `#[cfg(unix)] load_accepts_backslash_in_site_folder_path`, tagged `// AC-risk-6.3.7`. It uses `fixture_dir(..).join("my\\site")`, writes `posts/one.md`, and asserts that `load` returns exactly the slug `posts/one`. This also proves that the Linux filesystem accepts `\` in a folder name.
  - Both tests pass on the **unchanged** code, and `cargo test` is green.

### T-2 Split slugs only on the platform's separators
- Kind: implementation
- Satisfies: AC-3.1, AC-3.2, AC-3.3, AC-5.1, AC-5.2, AC-5.3, AC-5.4, AC-6.2, AC-6.3, AC-6.4
- Files: `src/content/slug.rs`
- Done when:
  - `from_content_path` uses `.replace(is_separator, "/")` (std's `std::path::is_separator`), and its doc comment says the platform's path separators are written as `/`, with a `\` inside a name on Unix being an invalid character.
  - `backslash_in_stem_becomes_separator` and its `// AC-arch-3.7.7` tag are gone.
  - New `#[cfg(unix)] backslash_in_name_is_an_invalid_file_name`, tagged `// AC-risk-6.3.1, AC-risk-6.3.2, AC-risk-6.3.3`, asserts `MangoError::General` and the exact `err.to_string()` for each case below, and that no message contains `DOTS_MESSAGE`:
    - `site/a\b.md` gives `Mango Error: site/a\b.md: invalid file name 'a\b': use only ASCII letters, digits, '-', '_' and '.'`.
    - `site/x\y/p.md` gives the segment `'x\y'`.
    - `site/\.md` gives the segment `'\'`.
    - `site/..\x.md` gives the segment `'..\x'`.

    Rust literals use `\\`. Paths go through the existing `slug()` helper; the file part of the message is `Path::new(path).display()`.
  - New `#[cfg(windows)] backslash_separates_folders_on_windows`, tagged `// AC-risk-6.5.1`: `slug("site\\posts\\post_one.md")` gives `posts/post_one`, and `slug("site\\a\\b.md")` gives `a/b`.
  - The unix test fails if the old `.replace('\\', "/")` is restored.
  - Every pre-existing test passes unmodified.
  - `Cargo.toml` and `Cargo.lock` are untouched.
  - The gate is green.

### T-3 Loader-level draft and precedence tests
- Kind: test
- Satisfies: AC-3.4, AC-3.5
- Files: `src/content/loader.rs`
- Done when:
  - `#[cfg(unix)] backslash_in_folder_name_is_rejected_even_for_drafts`, tagged `// AC-risk-6.3.4`, uses `site.join("x\\y").join("p.md")` with `page("Draft", true)`. `load` fails with `MangoError::General`, and the message contains `file.display()` and `invalid file name 'x\y'`.
  - `#[cfg(unix)] frontmatter_error_wins_over_backslash_in_name`, tagged `// AC-risk-6.3.5`, loops over `"tags": ["Rust"]` and `"date": "2026-02-30"` on a file `a\b.md`, as `frontmatter_error_wins_over_invalid_file_name` does. Each case gives `MangoError::Frontmatter`, and the message contains `'Rust'` or `'2026-02-30'` and not `invalid file name`.
  - `cargo test` is green.

### T-4 End-to-end tests through the binary
- Kind: test
- Satisfies: AC-3.1, AC-3.2, AC-3.4, AC-3.6, AC-3.7, AC-3.8, AC-4.1, AC-6.1, AC-6.5
- Files: `tests/build.rs`
- Done when:
  - `#[cfg(unix)] build_fails_on_backslash_in_file_name_keeping_output`, tagged `// AC-risk-6.3.1, AC-risk-6.3.2, AC-risk-6.3.4, AC-risk-6.3.8, AC-risk-6.4.1, AC-risk-6.6.1`. It covers three cases, each in its own `temp_dir`:
    - top-level `a\b.md`, not a draft, segment `a\b`;
    - folder `x\y/p.md`, not a draft, segment `x\y`, file built as `site.join("x\\y").join("p.md")`;
    - top-level `d\e.md`, a draft, segment `d\e`.

    For each case: build a valid site (`posts/one.md`), write `marker.txt`, take a `snapshot`, add the bad file and rebuild. The rebuild asserts `assert_failure`, `output.status.code() == Some(1)`, and that stderr contains `format!("{}: invalid file name '{segment}': use only ASCII letters, digits, '-', '_' and '.'", file.display())`. The snapshot must be unchanged.
  - `#[cfg(unix)] backslash_name_fails_before_collision_check`, tagged `// AC-risk-6.3.6`, covers two sites: `a\b.md` next to `a/b.md`, and `a\b.md` next to `a.md`. Each fails with exit status 1. Stderr contains `invalid file name 'a\b'` and does not contain `would be written`. `dist` does not exist afterwards.
  - `#[cfg(unix)] build_accepts_backslash_in_site_folder_path`, tagged `// AC-risk-6.3.7, AC-risk-6.6.5`: with the site at `dir.join("my\\site")` holding `posts/one.md`, `build_temp_site` succeeds, and `dist/posts/one/index.html` is a file.
  - The first two tests fail on the branch-point code, and the gate is green.

### T-5 User and developer docs
- Kind: docs
- Satisfies: AC-7.1, AC-7.2, AC-7.3
- Files: `README.md`, `CHANGELOG.md`, `CLAUDE.md`
- Done when:
  - `README.md:76` adds that a backslash (`\`) is not allowed in a file or folder name: on Linux and macOS a name such as `a\b.md` fails the build instead of being split into folders, and on Windows `\` is simply the folder separator.
  - `CHANGELOG.md` gains a second bullet under `## [Unreleased]` / `### Changed`, after the RISK-4 one, starting with `**Breaking:**` and written for site authors. It says that on Linux and macOS a file or folder name under the site folder that contains `\` now fails the build with an invalid-file-name error, drafts included, instead of being split into folders (`a\b.md` used to be published at `/a/b/`); that such files must be renamed; and that nothing changes on Windows. No version bump.
  - `CLAUDE.md`, "File names are strict": the sentence "On Unix a `\` in a file name becomes a `/` (RISK-6)." is replaced by the rule. Only the platform's path separators (`std::path::is_separator`: `/` on Unix, `/` and `\` on Windows) split segments. On Linux and macOS a `\` inside a name is an ordinary disallowed character and gets the charset error naming the name as on disk (`'a\b'`), drafts included (RISK-6).
  - All three are proofread; the repository is public.

### T-6 Spec and backlog bookkeeping
- Kind: docs
- Satisfies: AC-7.4, AC-7.5, AC-7.6
- Files: `specs/_system/overview.md`, `specs/_system/backlog.md`, `specs/arch-3/requirements.md`
- Done when:
  - `overview.md` line 28 cites `backslash_in_name_is_an_invalid_file_name` instead of `backslash_in_stem_becomes_separator`, and no removed test is cited anywhere in the file.
  - `backlog.md` shows RISK-6 as `done` in the summary row (line 33) and in its section heading line (line 323). The section gains a resolution note: resolved by `specs/risk-6/`, `from_content_path` splits only on `std::path::is_separator`, a `\` in a name on Linux or macOS gets the existing invalid-file-name error, breaking for 0.2.0, proven by `tests/build.rs::build_fails_on_backslash_in_file_name_keeping_output`. The note also replaces the stale "is pinned by `…backslash_in_stem_becomes_separator`" wording.
  - `specs/arch-3/requirements.md`: AC-7.7 ends with `**Withdrawn <YYYY-MM-DD of the edit>: superseded by specs/risk-6/requirements.md.**`, and AC-1.5 ends with a note that it no longer describes current behavior (superseded by `specs/risk-6/requirements.md`). No other ID or wording changes.
  - The pull request title ending in `(RISK-6)` is the orchestrator's job when landing, not a file change.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-2 (existing tests unchanged; Windows test) |
| AC-1.2 | T-2 (existing tests unchanged) |
| AC-1.3 | T-2 (existing tests unchanged) |
| AC-1.4 | T-2, T-3 |
| AC-1.5 | T-1 |
| AC-2.1 | replaced by AC-3.1: T-2 |
| AC-2.2 | replaced by AC-3.1, AC-3.2: T-2 |
| AC-2.3 | replaced by AC-3.6: T-4 |
| AC-2.4 | replaced by AC-3.3: T-2 |
| AC-3.1 | T-2, T-4 |
| AC-3.2 | T-2, T-4 |
| AC-3.3 | T-2 |
| AC-3.4 | T-3, T-4 |
| AC-3.5 | T-3 |
| AC-3.6 | T-4 |
| AC-3.7 | T-1, T-4 |
| AC-3.8 | T-4 |
| AC-4.1 | T-4 |
| AC-5.1 | T-2 |
| AC-5.2 | T-2 |
| AC-5.3 | T-2 |
| AC-5.4 | T-2 |
| AC-6.1 | T-4 |
| AC-6.2 | T-2 |
| AC-6.3 | T-2 |
| AC-6.4 | T-2 |
| AC-6.5 | T-4 |
| AC-7.1 | T-5 |
| AC-7.2 | T-5 |
| AC-7.3 | T-5 |
| AC-7.4 | T-6 |
| AC-7.5 | T-6 |
| AC-7.6 | T-6 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260925-234126-risk-6-backslash-file-names | Initial design: `from_content_path` replaces only `std::path::is_separator` characters with `/`, so on Unix a `\` stays in its segment and gets the existing charset error. Windows is unchanged. 6 tasks: a safety net for the site-prefix rule, the one-line change with Unix and Windows unit tests, loader and E2E tests, and docs and spec bookkeeping. |
