<!-- Published by dev-pipeline run 20260926-004309-risk-5-broken-symlinks-in-site, stage design v1, approved 2026-09-26T02:35:00Z. -->

# Unresolvable symlinks in the site folder fail the build: Design

Spec ID: `risk-5` · Requirements: `specs/risk-5/requirements.md`

## Overview
`loader::traverse` already classifies every entry with `fs::metadata`, which follows symlinks. When that call fails (a dangling link, a looping chain, or an entry that vanished), the loader currently skips the entry with `let Ok(target) = … else { continue; }`. The fix changes one statement: the error goes to `MangoError::io_at(path, e)` and is returned with `?`, exactly as `assets::collect` does (`src/build/generate/assets.rs`, `let target = fs::metadata(&source).map_err(|e| MangoError::io_at(&source, e))?;`).

The metadata call already runs before any name check (`is_markdown`), so every name fails the same way, hidden names included. The load runs inside `plan`, before anything is cleaned, so the previous output survives.

The rest of the work is tests (unit and E2E, all `#[cfg(unix)]`), docs and spec bookkeeping.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/loader.rs` | `traverse`: the skip on a `fs::metadata` error becomes `map_err(io_at)?`, and the comment above it is rewritten. Tests: the safety-net test for hidden markdown files is added. `load_ignores_dangling_symlink` (`// AC-11.6`) and `load_ignores_symlink_loop_chain` (`// AC-11.7`) are removed. Three `#[cfg(unix)]` failure tests are added. |
| `tests/build.rs` (tests only) | New `#[cfg(unix)]` E2E test: a dangling link under `site/` exits 1, names the link on stderr and leaves the previous output intact. |
| `README.md` | Known limitations (line 117): a broken link or a link loop under `site/` is a build error, as under `meta/assets/`. |
| `CHANGELOG.md` | A third `**Breaking:**` bullet under `## [Unreleased]` / `### Changed`. |
| `CLAUDE.md` | Module map, `src/content/` entry: replace the "skipped silently — deliberately unlike `assets::plan` … see RISK-5" clause with the new rule. |
| `specs/_system/overview.md` | The `src/content/loader.rs` component row (line 25) and the Risky areas `traverse` row (line 108): no RISK-5 as open, no removed test cited. |
| `specs/_system/backlog.md` | RISK-5 marked `done`, with Workflow `/spec-feature`, in the table row (line 32) and the section header (line 318), plus a resolution note. |
| `specs/_system/legacy-criteria/risk-1-symlinked-content-folders.md` | A supersession note for AC-11.6 and AC-11.7 under REQ-3. The recovered wording is unchanged. |

## Approach
**The change.** In `traverse` (`src/content/loader.rs`), this:

```rust
let Ok(target) = fs::metadata(path) else {
    continue;
};
```

becomes:

```rust
let target = fs::metadata(path).map_err(|e| MangoError::io_at(path, e))?;
```

The comment above it keeps the explanation of `metadata` (follows the link) versus `is_symlink` (does not), and why a symlinked folder is rejected. It replaces the "skipped … tracked as RISK-5" part with: an unresolvable entry (a dangling link, an `ELOOP` chain, an entry removed after `read_dir`) fails the load with an I/O error naming the link, as in `assets::plan`, and it does so before any name check, so no name is exempt.

Why this satisfies each criterion:
- **AC-2.1, AC-2.2 and AC-2.4 (any depth, any failure).** Every entry at every depth goes through this one statement, because `traverse` recurses into real folders. Every `Err` from `fs::metadata` now fails the load, whatever its cause. A loop terminates: `stat` fails once with `ELOOP` on that entry, and nothing recurses. Existing tests show that `fs::metadata` returns `Err` for a dangling link and for a loop (see Evidence).
- **AC-2.3 (no name exempt).** The statement comes before `if !target.is_file() || !is_markdown(path)`, so names never reach a check. `.#post.md`, `broken.txt` and `broken` fail exactly like `broken.md`.
- **AC-2.5 (the error).** `MangoError::IoPath` displays as `Mango I/O Error at '<path>': <source>` (`src/error.rs`, the `#[error(...)]` on `IoPath`). This is the variant and helper that `assets::collect` uses. `path` is `entry.path()`, which is the `read_dir` argument joined with the entry's file name (see Evidence). The walk starts at `site` exactly as passed to `load`, so the path starts from the `--site` value, is not shortened, and names the link, not its target. The source is the OS error, which displays as `<detail> (os error <code>)`.
- **AC-2.6.** `main` already prints any `MangoError` to stderr and exits 1 (CLAUDE.md, module map `src/main.rs`). The E2E test in T-3 checks this.
- **AC-3.1.** `loader::load` is the first step of `plan`, and `commit` only accepts a `BuildPlan` (CLAUDE.md, plan/commit seam). T-3 checks this with `snapshot`.
- **AC-4.1 and AC-4.3.** The folder, markdown and non-markdown branches and the root `is_dir()` guard are untouched. Their existing tests run unchanged. Hidden names that resolve go through the same unchanged branches. T-1 pins this before the change.
- **AC-4.2, AC-4.4 and AC-4.5.** `assets.rs`, the fixture, `Cargo.toml` and `Cargo.lock` are not touched. The fixture has no symlinks, so every one of its `metadata` calls already succeeds and its output is unchanged.

**Evidence for external behavior** (Rust 1.92.0, pinned in `rust-toolchain.toml`, source at `~/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/`):
- `fs::metadata` "will traverse symbolic links to query information about the destination file", and on Unix it corresponds to `stat` (`fs.rs:2587-2593`). It errors when "`path` does not exist" (`fs.rs:2604`).
- An existing test shows that `fs::metadata` returns `Err` for a dangling link and for a two-link loop on Linux: `plan_fails_on_dangling_symlink` and `plan_fails_on_symlink_loop` (`src/build/generate/assets.rs`) assert `MangoError::IoPath` coming from exactly this `fs::metadata(..).map_err(io_at)?` call. The requirements also record a manual run on 2026-09-25 that gave `Mango I/O Error at 'assets/x.css': No such file or directory (os error 2)`.
- `DirEntry::path` is "created by joining the original path to `read_dir` with the filename of this entry" (`fs.rs:2389-2391`). So the reported path is the walked link path, built from the `--site` value.
- `io::Error`'s `Display` for an OS error is `"{detail} (os error {code})"` (`io/error.rs:1039-1045`).
- On Unix, `ENOENT` maps to `ErrorKind::NotFound` (`sys/pal/unix/mod.rs:254`). `ELOOP` maps to `ErrorKind::FilesystemLoop` (`:253`), but that variant is unstable (`io/error.rs:298`, `#[unstable(feature = "io_error_more")]`). So the loop test must assert `raw_os_error().is_some()`, not the kind.

## Interfaces and data
None. No signature, error variant or template context changes. The only change is which outcome the loader returns for an unresolvable entry: `Err(MangoError::IoPath { path, source })` instead of skipping it.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| A dedicated `General` message ("broken symlink", "symlink loop") | Out of scope. The maintainer chose to reuse the assets I/O error (AC-2.5), and a clearer message would have to change the assets error too. |
| Classify with `symlink_metadata` first, then `metadata` only for links | More code for no gain. `metadata` already covers every case, and the error must name the link path, which `io_at(path, …)` already does. |
| Warn on stderr and continue | Rejected by the maintainer (Out of scope: a warning instead of an error). |
| Fail only for markdown-named links | Contradicts AC-2.3. It would also need a name check before resolution, reversing today's order. |

## Risks
- **Existing sites break.** Any site with a stale link (including Emacs `.#name` lock files while a file is being edited) now fails to build. This is intended and recorded as breaking (AC-6.2).
- **Which error comes first.** When a site has both an unresolvable link and another bad file, the one reported depends on `read_dir` order (RISK-8, out of scope). The new tests avoid depending on this: each fixture has exactly one failing entry, or accepts either link of a loop.
- **Windows.** The code path is identical on Windows, where an entry whose `fs::metadata` fails now fails the build instead of being skipped. No test creates symlinks on Windows (all symlink tests are `#[cfg(unix)]`, as the requirements allow in REQ-5). The design does not rely on how Windows reports a dangling link. Whatever `metadata` returns there, the only change is that `Err` fails instead of skipping.
- **Test path construction.** Expected paths are built one segment per `join` (`site.join("posts").join("broken.md")`), per the CLAUDE.md convention. Tests never hard-code a `./` or `site/` prefix.

**Unverified assumptions:** None. Every claim above is backed by the std 1.92.0 source, by existing tests (`plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`), or by the recorded manual run.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` (constitution, Commands).
- Safety net for the `[baseline]` criteria in the code being changed:
  - Already covered by existing tests, which must pass unmodified:
    - AC-1.4: `load_rejects_symlinked_directory`, `load_rejects_symlink_loop_to_ancestor`, `build_fails_on_symlinked_content_folder_keeping_output`.
    - AC-1.5: `load_follows_symlinked_markdown_file`, `build_reads_symlinked_markdown_file`.
    - AC-1.6: `load_ignores_symlinked_non_markdown_file`.
    - AC-1.7: `load_accepts_symlinked_site_root`.
    - AC-1.8: `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop` (in `assets.rs`, which this change does not touch).
  - Not replaced, but untested: AC-4.3 (a resolving hidden entry is handled as today). No test loads a hidden markdown file. **T-1** pins it on the unchanged code.
  - AC-1.1, AC-1.2 and AC-1.3 are replaced by REQ-2, so they get no safety net. Their pinning tests are removed in T-2 (AC-5.5).
- New and changed behavior:
  - `src/content/loader.rs`:
    - `load_fails_on_dangling_symlink` covers AC-2.1, AC-2.3, AC-2.5 and AC-5.1.
    - `load_fails_on_symlink_loop`, with a top-level and a nested case, covers AC-2.2, AC-2.5 and AC-5.2.
    - `load_fails_on_dangling_symlink_in_nested_folder` covers AC-2.1 and AC-5.3.
    - `load_publishes_hidden_markdown_file` covers AC-4.3.
  - `tests/build.rs`: `build_fails_on_dangling_symlink_in_site_keeping_output` covers AC-2.5, AC-2.6, AC-3.1 and AC-5.4.
  - AC-2.4 has no dedicated test (REQ-5). It is covered through AC-2.1 and AC-2.2.
  - AC-4.1, AC-4.2, AC-4.4 and AC-4.5 are covered by every existing test passing unchanged, including the fixture manifest, `fixture_build_is_deterministic` and `fixture_internal_links_resolve`.
  - The Tester should also build `example/` at `branch_point` and at HEAD and compare the two `dist` trees byte for byte (AC-4.4), and confirm that `Cargo.toml` and `Cargo.lock` are unchanged (AC-4.5).

## Tasks
### T-1 Pin that a hidden markdown file is published
- Kind: safety-net
- Satisfies: AC-4.3
- Files: `src/content/loader.rs`
- Done when:
  - `loader.rs` has a new test, `load_publishes_hidden_markdown_file`, tagged `// AC-risk-5.4.3`. It is not platform-gated, because it uses no symlinks.
  - The test writes `site.join(".notes.md")` with `page("Notes", false)` next to `site.join("posts").join("one.md")`.
  - It asserts that `load` succeeds and that the slugs, sorted, are exactly `[".notes", "posts/one"]`.
  - It also asserts that the `.notes` page's `slug.url()` is `/.notes/`.
  - The test passes on the unchanged code, and `cargo test` is green.

### T-2 Fail the load on an unresolvable entry
- Kind: implementation
- Satisfies: AC-1.1, AC-1.2, AC-1.3 (replaced), AC-2.1, AC-2.2, AC-2.3, AC-2.4, AC-2.5, AC-4.1, AC-4.2, AC-4.5, AC-5.1, AC-5.2, AC-5.3, AC-5.5
- Files: `src/content/loader.rs`
- Done when:
  - **The change.** `traverse` uses `let target = fs::metadata(path).map_err(|e| MangoError::io_at(path, e))?;`. The comment above it no longer says the entry is skipped or that the change is tracked as RISK-5. It says an unresolvable entry fails with the same I/O error as `assets::plan`, before any name check.
  - **Removed tests.** `load_ignores_dangling_symlink` and `load_ignores_symlink_loop_chain` are gone, together with their `// AC-11.6 [baseline] …` and `// AC-11.7 [baseline] …` comments.
  - **Dangling links.** New `#[cfg(unix)] load_fails_on_dangling_symlink`, tagged `// AC-risk-5.5.1, AC-risk-5.2.1, AC-risk-5.2.3, AC-risk-5.2.5`:
    - It loops over the names `broken.md`, `broken.txt`, `broken` and `.#post.md`, each in its own `fixture_dir` (so the `read_dir` order never matters).
    - In each: `dir = fixture_dir(..)`, `site = dir.join("site")`, a real `site.join("real.md")`, and a link `site.join(name)` pointing to `dir.join("missing-target")`, which does not exist.
    - Assertions: `load(&site)` returns `MangoError::IoPath { path, source }` with `path == link` and `source.kind() == std::io::ErrorKind::NotFound`.
    - The message starts with `format!("Mango I/O Error at '{}': ", link.display())`, contains `(os error `, and does not contain `missing-target`.
  - **Loops, top-level and nested.** New `#[cfg(unix)] load_fails_on_symlink_loop`, tagged `// AC-risk-5.5.2, AC-risk-5.2.2, AC-risk-5.2.5`. This covers the maintainer's note:
    - Case one: `a.md -> b.md` and `b.md -> a.md` at the site root.
    - Case two: the same pair inside `site.join("posts")`, next to a real `posts/real.md`.
    - Each case gets its own `fixture_dir`, and the links use relative targets (`symlink("b.md", …)`), as the removed test did.
    - Assertions for each case: the call returns (terminates) with `MangoError::IoPath { path, source }`, where `path` equals one of the two link paths (built one segment per `join`) and `source.raw_os_error().is_some()`. The message contains that path's `display()`.
  - **Nested dangling link.** New `#[cfg(unix)] load_fails_on_dangling_symlink_in_nested_folder`, tagged `// AC-risk-5.5.3, AC-risk-5.2.1`:
    - The link is `site.join("posts").join("deep").join("broken.md")` and points to a missing target. The site also has a real `site.join("posts").join("real.md")`.
    - It asserts `MangoError::IoPath` with `path == link`.
  - **Checks.**
    - All three new tests fail on the branch-point code (it returns `Ok`).
    - Every remaining existing test passes unmodified, including all symlink tests in `loader.rs` and `assets.rs` and `tests/plan.rs`.
    - `Cargo.toml` and `Cargo.lock` are untouched.
    - The gate is green.

### T-3 End-to-end: a dangling link fails the build and keeps the output
- Kind: test
- Satisfies: AC-2.5, AC-2.6, AC-3.1, AC-5.4
- Files: `tests/build.rs`
- Done when:
  - New `#[cfg(unix)] build_fails_on_dangling_symlink_in_site_keeping_output`, tagged `// AC-risk-5.5.4, AC-risk-5.2.6, AC-risk-5.3.1, AC-risk-5.2.5`, placed after `build_reads_symlinked_markdown_file`. It follows `build_fails_on_symlinked_content_folder_keeping_output`:
    1. `dir = temp_dir(..)`, `site = dir.join("site")`, `out = dir.join("dist")`, with `site/posts/one.md`.
    2. `assert_success(&build_temp_site(&site, &out))`, then write `marker.txt` into `out` and take `before = snapshot(&out)`.
    3. Add `link = site.join("posts").join("broken.md")`, a symlink to `dir.join("missing-target")`, and rebuild.
  - Assertions on the rebuild:
    - `assert_failure(&output, "dangling symlink in site")` and `output.status.code() == Some(1)`.
    - Stderr contains `format!("Mango I/O Error at '{}': ", link.display())` and `os error`, and does not contain `missing-target`.
    - `snapshot(&out) == before`.
  - The test fails on the branch-point code, and the gate is green.

### T-4 User and developer docs
- Kind: docs
- Satisfies: AC-6.1, AC-6.2, AC-6.3
- Files: `README.md`, `CHANGELOG.md`, `CLAUDE.md`
- Done when:
  - **`README.md:117`.** The sentence "Unlike in `meta/assets/`, a broken link or a link loop under `site/` is ignored rather than an error." is replaced. The new wording says that a broken link or a link loop under `site/` is a build error too, as in `meta/assets/`, whatever the link's name. The rest of the bullet is unchanged.
  - **`CHANGELOG.md`.** A third bullet is added under `## [Unreleased]` / `### Changed`, after the RISK-6 one. It starts with `**Breaking:**` and is written for site authors:
    - a broken symlink, or a chain of symlinks that loops, anywhere under the site folder now fails the build with an error naming the link, instead of being skipped silently (which could drop a page or a whole section without a word);
    - this applies to any name, including hidden files such as editor lock files;
    - fix or remove such links before building.

    No version bump.
  - **`CLAUDE.md`, module map, `src/content/` entry.** The clause "and an entry whose target cannot be resolved (dangling link, `ELOOP`) is skipped silently — deliberately unlike `assets::plan`, which errors on both; see RISK-5" is replaced. The new wording says that an entry whose target cannot be resolved (a dangling link, an `ELOOP` chain, an entry removed after `read_dir`), whatever its name, fails the load with `MangoError::io_at(<walked link path>, e)`, the same error `assets::plan` gives (RISK-5).
  - All three are proofread; the repository is public.

### T-5 Spec and backlog bookkeeping
- Kind: docs
- Satisfies: AC-6.4, AC-6.5, AC-6.6
- Files: `specs/_system/overview.md`, `specs/_system/backlog.md`, `specs/_system/legacy-criteria/risk-1-symlinked-content-folders.md`
- Done when:
  - **`overview.md`.**
    - The `src/content/loader.rs` row (line 25) says the loader fails on an unresolvable entry instead of skipping it, and cites `load_fails_on_dangling_symlink` instead of `load_ignores_dangling_symlink`.
    - The Risky areas `traverse` row (line 108) no longer lists RISK-5 as open or cites the removed tests. It says RISK-5 is done (unresolvable entries fail like in `assets::plan`; `load_fails_on_dangling_symlink`, `load_fails_on_symlink_loop`), and keeps the RISK-8 text.
    - No removed test name appears anywhere in the file.
  - **`backlog.md`.**
    - The RISK-5 row (line 32) reads `| [RISK-5](#risk-5) | Unresolvable symlinks under `site/` are silently ignored | low | S | `/spec-feature` | — | done |`.
    - The section header (line 318) reads `… · low · S · `/spec-feature` · done`.
    - The section gains a resolution note:
      - resolved by `specs/risk-5/`;
      - `loader::traverse` now fails on any entry whose `fs::metadata` fails, with the same `io_at` error as `assets::plan`, whatever its name;
      - the two modules now follow one rule;
      - breaking, recorded in `CHANGELOG.md` for 0.2.0;
      - `tests/build.rs::build_fails_on_dangling_symlink_in_site_keeping_output` proves the previous output survives.
    - The note replaces the stale sentence saying `load_ignores_dangling_symlink` and `load_ignores_symlink_loop_chain` pin the current behavior.
  - **Legacy criteria file.** Under `### REQ-3 [baseline] An unresolvable symlink stays silently ignored`, after its existing paragraph and before AC-11.6, a note is added: `**Superseded <YYYY-MM-DD of the edit>:** AC-11.6 and AC-11.7 no longer describe current behavior; they are superseded by specs/risk-5/requirements.md (RISK-5).` The recovered wording of the heading, the paragraph, AC-11.6, AC-11.7 and the Verified-by table is unchanged.
  - The pull request title ending in `(RISK-5)` is the orchestrator's job when landing, not a file change.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | replaced by AC-2.1: T-2 |
| AC-1.2 | replaced by AC-2.2: T-2 |
| AC-1.3 | replaced by AC-2.4: T-2 |
| AC-1.4 | T-2 (existing tests unchanged) |
| AC-1.5 | T-2 (existing tests unchanged) |
| AC-1.6 | T-2 (existing tests unchanged) |
| AC-1.7 | T-2 (existing tests unchanged) |
| AC-1.8 | T-2 (`assets.rs` untouched; existing tests unchanged) |
| AC-2.1 | T-2 |
| AC-2.2 | T-2 |
| AC-2.3 | T-2 |
| AC-2.4 | T-2 |
| AC-2.5 | T-2, T-3 |
| AC-2.6 | T-3 |
| AC-3.1 | T-3 |
| AC-4.1 | T-2 |
| AC-4.2 | T-2 |
| AC-4.3 | T-1 |
| AC-4.4 | T-2 (no fixture change; fixture tests unchanged) |
| AC-4.5 | T-2 |
| AC-5.1 | T-2 |
| AC-5.2 | T-2 |
| AC-5.3 | T-2 |
| AC-5.4 | T-3 |
| AC-5.5 | T-2 |
| AC-6.1 | T-4 |
| AC-6.2 | T-4 |
| AC-6.3 | T-4 |
| AC-6.4 | T-5 |
| AC-6.5 | T-5 |
| AC-6.6 | T-5 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260926-004309-risk-5-broken-symlinks-in-site | Initial design. `loader::traverse` returns `MangoError::io_at(path, e)` when `fs::metadata` fails, instead of skipping the entry, matching `assets::collect`. 5 tasks: <ul><li>a safety net for hidden markdown files;</li><li>the one-statement change, with the two skip tests replaced by dangling-link (four names, plus nested) and loop (top-level and nested) failure tests;</li><li>an E2E snapshot test;</li><li>user and developer docs;</li><li>spec and backlog bookkeeping.</li></ul> |
