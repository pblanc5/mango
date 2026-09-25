<!-- Published by dev-pipeline run 20260922-224052-arch-1-split-plan-commit, stage design v1, approved 2026-09-22T23:40:01-05:00. -->

# Split the build into plan and commit; add a library crate: Design

Spec ID: `arch-1` · Requirements: `specs/arch-1/requirements.md`

## Overview
Today `cli::build` (`src/cli.rs:89-171`) does everything in one function. The design cuts it at line 152/154 (after the last `output::render`, before `current_dir`/`ensure_safe_to_clean`): everything above becomes `plan()`, which returns a `BuildPlan` whose fields are private; everything below becomes `commit(plan)`, which is the only function that calls `clean_contents`, `output::write` and `assets::copy`. The crate gains `src/lib.rs`, whose module tree is entirely private and which re-exports exactly seven items (`plan`, `commit`, `clean`, `BuildOptions`, `BuildPlan`, `PlannedOutput`, `MangoError`). Because every module is private, a `pub` item that nothing uses still triggers the dead-code lint, so the "no dead code" rule stays compiler-enforced everywhere except those seven names. The clap definitions stay in `src/cli.rs`, which becomes a module of the **binary** only (`mod cli;` in `main.rs`), so the binary parses and dispatches and nothing else. In-process tests live in a new integration test file `tests/plan.rs` that uses only the public API, which is the same surface FEAT-1 will consume. No message, flag, output byte or exit code changes.

## Affected components
| Component / file | Change |
|---|---|
| `Cargo.toml` | **No edit.** With both `src/lib.rs` and `src/main.rs` present Cargo auto-detects a lib and a bin target, both named `mango`. No `[lib]` section, no dependency change (AC-6.5). |
| `src/lib.rs` (new) | Library root: `mod build; mod config; mod content; mod error; mod render;` (all private) plus the seven `pub use` re-exports. Doc comment states the internal-seam rule (AC-9.3 wording also goes in `CLAUDE.md`). |
| `src/main.rs` | Becomes `mod cli;` + `fn main()` (unchanged stderr/exit-code handling). The five private `mod` lines for the pipeline modules go. |
| `src/cli.rs` | Binary-only module. Keeps `BuildOpts`, `CleanOpts`, `MangoActions`, `MangoCli` and `run()` verbatim (help text, defaults, `not implemented` text). `build()`, `clean()`, `current_dir()`, `ensure_safe_to_clean()`, `clean_contents()` and the whole `#[cfg(test)]` module leave. Dispatch becomes `Build(opts) => mango::commit(mango::plan(&opts.into_options())?)`, `Clean(opts) => mango::clean(Path::new(&opts.dist))`. |
| `src/build/mod.rs` | Add `pub mod clean;` and `pub mod pipeline;`. |
| `src/build/pipeline.rs` (new) | `BuildOptions`, `BuildPlan`, `PlannedOutput`, `plan()`, `commit()`. The body of `plan()` is `cli.rs:90-152` moved; the body of `commit()` is `cli.rs:154-170` moved. |
| `src/build/clean.rs` (new) | `ensure_safe_to_clean`, `clean_contents`, `current_dir` (moved verbatim from `cli.rs:190-266`, `pub(crate)`), `pub fn clean(dist: &Path)` (the `mango clean` operation, `cli.rs:173-188` verbatim), and the seven moved unit tests with their legacy tags; fixture dir becomes `target/unit-fixtures/clean/`. |
| `src/build/output.rs` | `#[derive(Debug)]` on `RenderedFile` (so `BuildPlan: Debug`, needed by `expect_err` in tests). Nothing else. |
| `src/error.rs` | `io_at` becomes `pub(crate)`: it is used only inside the crate and would otherwise be part of the public surface through the re-exported `MangoError`. |
| `tests/plan.rs` (new) | 13 in-process tests against the public API (11 new, 2 moved from `tests/build.rs`), with their own small helpers. |
| `tests/build.rs` | Two safety-net tests added (T-1, T-2); `home_recent_respects_recent_count_and_skips_undated` and `build_excludes_draft_pages` removed (moved to `tests/plan.rs`). No other line changes. |
| `CLAUDE.md` | `build`/`clean` bullets, module map, tests convention, internal-seam statement (AC-9.1, AC-9.3). |
| `specs/_system/overview.md` | Only the line-13 sentence "There is no library crate: …" (AC-9.4). |
| `specs/_system/backlog.md` | ARCH-1 `open` → `done` in the index row and item header, with a short landed-summary paragraph in the style of RISK-1/RISK-2 (AC-9.2). |
| `README.md` | **No edit** (AC-8.4; checked: it mentions neither `cli.rs`, `lib.rs` nor a library, and the "Known limitations" section is unaffected). |

## Approach

**1. Two targets, one crate name.** `src/lib.rs` is the library root; `src/main.rs` the binary root. Both are in `src/`, and `src/cli.rs` is declared only by `main.rs` (`mod cli;`), so it compiles into the binary alone and can use clap freely; the library never sees clap. Inside the binary the library is `mango::…` (edition-2024 extern prelude handles the shared name). `clap` stays in `[dependencies]` and is used by the bin target only; Cargo does not warn about that.

**2. Public surface = the re-exports in `lib.rs`, nothing else.** Every module stays private in `lib.rs`; the existing `pub mod`/`pub fn` inside them are crate-reachable but not externally visible, so the compiler keeps warning about unused ones (the trap named in AC-6.4 only applies to items reachable from outside). The complete list of externally reachable items, for the AC-4.5 review:

| Item | Kind | Can it obtain or alter a `BuildPlan`? |
|---|---|---|
| `mango::plan(&BuildOptions) -> Result<BuildPlan, MangoError>` | fn | **The only constructor.** |
| `mango::commit(BuildPlan) -> Result<(), MangoError>` | fn | Consumes it; no destination parameter. |
| `mango::clean(&Path) -> Result<(), MangoError>` | fn | No. |
| `mango::BuildOptions` (5 `pub` fields) | struct | No (input only). |
| `mango::BuildPlan` | struct, all fields private, derives `Debug` only (no `Default`, no `Clone`, no `From`) | Not constructible; no `&mut` accessor. |
| `BuildPlan::output_dir(&self) -> &Path` | method | Read-only. |
| `BuildPlan::outputs(&self) -> impl Iterator<Item = PlannedOutput<'_>>` | method | Read-only borrows (`&Path`, `&str`). |
| `mango::PlannedOutput<'a>` | enum, derives `Debug, PartialEq, Eq` | Borrowed views; cannot reach the plan. |
| `mango::MangoError` (+ its `pub` variants, `Display`, `Error`, `From<io::Error>`, `From<tera::Error>`) | enum | No. `io_at` becomes `pub(crate)`. |

`MangoError`'s `From` impls come from `#[from]` and exist today; they do not touch plans.

**3. `plan()` writes nothing.** It is `cli.rs:90-152` moved, with two deliberate carry-overs: `Path::new(".").join(&opts.site)` stays (AC-3.3, decision 2; a comment names ARCH-7), and `opts.config` `None` resolves to `config::DEFAULT_CONFIG_PATH` with `explicit = false` exactly as today (AC-3.2). It reads through `loader::load`, `config::load`, `template::load_templates`, `assets::plan` (reads the folder listing only), then builds items, generated files, runs `check_collisions(dist, …)` with the same `dist` (so messages keep naming `<dist>/…`, AC-5.4), renders everything, and returns:

```rust
pub struct BuildPlan {
    output_dir: PathBuf,           // `opts.output` as given (AC-5.4)
    protected: Vec<PathBuf>,       // [site_path, templates, assets, config_path] (AC-4.8)
    files: Vec<output::RenderedFile>, // pages, sections, home, tags, generated — today's write order
    assets: Vec<assets::AssetFile>,
}
```
The four `output::render` calls and `render_generated` are concatenated into `files` in the same order the five `output::write` calls run today, so the write sequence is byte-for-byte the same. Reviewer check for AC-4.2/AC-4.9: `pipeline.rs` contains no `fs::` write/remove call, and the only callers of `clean::clean_contents`, `output::write` and `assets::copy` in the crate are `commit` (grep is enough).

**4. `commit()` is `cli.rs:154-170` moved:** `current_dir()` → `ensure_safe_to_clean(&plan.output_dir, &cwd, &plan.protected)` → `clean_contents(&plan.output_dir)` → `output::write(&plan.files)` → `assets::copy(&plan.assets)`. Same order, same functions, same error texts (AC-4.6, AC-4.7). It is a free function in `pipeline.rs`, the same module as `BuildPlan`, so the private fields never need `pub(crate)` and no other module can read or build a plan.

**5. `mango::clean` and AC-4.9.** `clean` (the `mango clean` operation) removes the whole folder with `remove_dir_all` after the same safety check. Reading AC-4.9 with AC-6.3 (which requires this logic to be reachable through the library): AC-4.9 forbids any other public operation from *emptying or writing into* the build's output folder as part of a build; `clean` is the separate user-invoked removal of the folder itself, unchanged from today. The approver should confirm this reading; it is not a change to either criterion.

**6. `outputs()` enumerates relative paths.** Internally the paths stay full (`dist.join(…)`), as today, so `check_collisions`, `write` and `copy` and every message are untouched. `outputs()` yields `path.strip_prefix(&self.output_dir).unwrap_or(path)`: every stored path is built by joining onto `output_dir` inside `plan()`, which is the plan's only constructor, so the fallback is unreachable, and the AC-5.3 test proves the enumeration matches the disk after a commit. Making paths relative natively is left to ARCH-2, which reshapes the output types anyway.

**7. In-process tests are integration tests (`tests/plan.rs`), not unit tests.** They exercise the library exactly as the binary and the future dev server do, so they double as the proof that the seven re-exports are sufficient and that nothing else is needed (AC-6.4). They write a five-template minimal theme per test (each template prints only titles/slugs, one per line) so listing order is asserted as exact strings rather than by scraping the fixture theme. They never call `set_current_dir` (tests run in parallel), so the cwd-refusal case stays with the unit test `refuses_cwd` and the E2E `build_refuses_to_clean_cwd`.

**8. Two baseline criteria are superseded, not preserved.** REQ-2 describes the state this spec replaces: AC-2.1 (no library target) is replaced by AC-6.1, and the second clause of AC-2.3 (no in-process load-through-render test) by REQ-7. AC-2.2 (the E2E suite only spawns the binary) remains true of `tests/build.rs`. The coverage table maps them to the tasks that replace them.

## Interfaces and data

```rust
// src/lib.rs (complete)
mod build; mod config; mod content; mod error; mod render;
pub use build::clean::clean;
pub use build::pipeline::{BuildOptions, BuildPlan, PlannedOutput, commit, plan};
pub use error::MangoError;

// src/build/pipeline.rs
pub struct BuildOptions {
    pub site: PathBuf,          // joined onto "." inside plan() (AC-3.3)
    pub templates: PathBuf,
    pub assets: PathBuf,
    pub output: PathBuf,
    pub config: Option<PathBuf>, // None → "mango.json", missing file allowed
}
#[derive(Debug)]
pub struct BuildPlan { /* private, see Approach 3 */ }
#[derive(Debug, PartialEq, Eq)]
pub enum PlannedOutput<'a> {
    /// A rendered or generated file: path relative to the output folder, exact contents.
    File { path: &'a Path, contents: &'a str },
    /// An asset copy: destination relative to the output folder, and the source file.
    Copy { path: &'a Path, source: &'a Path },
}
impl BuildPlan {
    pub fn output_dir(&self) -> &Path;
    /// Files first (pages, sections, home, tags, feed, sitemap), then asset copies: the write order.
    pub fn outputs(&self) -> impl Iterator<Item = PlannedOutput<'_>>;
}
pub fn plan(opts: &BuildOptions) -> Result<BuildPlan, MangoError>;
pub fn commit(plan: BuildPlan) -> Result<(), MangoError>;

// src/build/clean.rs
pub fn clean(dist: &Path) -> Result<(), MangoError>;                    // `mango clean`
pub(crate) fn current_dir() -> Result<PathBuf, MangoError>;
pub(crate) fn ensure_safe_to_clean(target: &Path, cwd: &Path, protected: &[&Path]) -> Result<(), MangoError>;
pub(crate) fn clean_contents(dist: &Path) -> Result<(), MangoError>;

// src/cli.rs (binary): BuildOpts gains one private helper
impl BuildOpts { fn into_options(self) -> mango::BuildOptions { /* String → PathBuf, field by field */ } }
```
`ensure_safe_to_clean` keeps `&[&Path]`; `commit` builds that slice from `plan.protected` with `iter().map(PathBuf::as_path)`, as `clean` already does today.

`tests/plan.rs` helpers (private to that file; duplicated from `tests/build.rs` on purpose, see Alternatives): `temp_dir`, `write_file`, `frontmatter(title, date, tags_json, draft)`, `Project { site, templates, assets, out }` + `project(name)` which writes the minimal templates (`page.html` = `{{ page.title }}`; `section.html` = `{% for p in section.pages %}{{ p.title }} {{ p.slug }}\n{% endfor %}`; `home.html` = `{% for p in home.recent %}{{ p.title }}\n{% endfor %}`; `tags.html` = `{% for t in tags %}{{ t.name }}\n{% endfor %}`; `tag.html` = `{% for p in tag.pages %}{{ p.title }}\n{% endfor %}`) and one asset `style.css`; `options(&Project, config: Option<&Path>) -> BuildOptions`; `tree(dir) -> BTreeMap<String, Vec<u8>>` (relative path → bytes, for before/after snapshots); `file<'a>(&'a BuildPlan, rel: &str) -> &'a str`.

Schema/format changes: none. Output bytes: none.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Put clap in the library (`mango::cli::run()`), `main.rs` just calls it | The library would parse process arguments, and "the binary does nothing but parse and call the library" (AC-6.1) would be hollow. Keeping `cli.rs` as a binary-only module gives AC-6.2 a file to point at. |
| `commit(plan, dist)` as in the backlog sketch | Rejected by decision 1 (AC-4.5, AC-5.4): a plan is bound to the folder it was planned against. |
| Store output paths relative to `output_dir` inside the plan | Cleaner, but touches `check_collisions`, `render`, `write`, `assets::plan`/`copy` and about ten unit tests for no behaviour change; ARCH-2 rewrites those types anyway. The `strip_prefix` view costs one line. |
| In-process tests as `#[cfg(test)]` unit tests inside `src/` | They could use crate internals and would not prove that the public surface is sufficient; `tests/plan.rs` uses only what FEAT-1 will use, which is the AC-6.4 measure. |
| Shared `tests/common/mod.rs` for helpers used by both test files | Each integration test file is its own crate; any helper one of them does not use warns as dead code under `-D warnings`, and the only fix is `#[allow(dead_code)]`, which the constitution forbids. ~40 duplicated helper lines are the cheaper price. |
| Make `commit` a method (`plan.commit()`) or a typestate/trait | Cosmetic; free functions match the backlog vocabulary and the CLI reads as `commit(plan(&opts)?)`. |
| Move more E2E tests in-process (`build_generates_ancestor_section_indexes`, `build_allows_index_md_inside_section`, `build_fails_on_invalid_tag_in_draft`, …) | Each move is a review item under AC-7.4/AC-7.10 and shrinks the on-disk contract. Two moves show the mechanism; the rest stay until SPEC-3 rewrites tags anyway. |

## Risks
- **`pub` silences the dead-code lint.** Mitigated structurally: only `lib.rs` re-exports are public. Reviewer check: `lib.rs` has exactly the seven `pub use` names and no `pub mod`.
- **`strip_prefix` fallback in `outputs()`** is unreachable by construction; if a future change stored a path not under `output_dir`, `commit_writes_exactly_the_enumerated_outputs` (AC-5.3) fails.
- **Legacy-criteria tables cite `cli::tests::refuses_cwd` etc.** (`specs/_system/legacy-criteria/batch-2-consistent-output.md:108-113`, `batch-3-usable-site.md:65-66,123-124`). The module prefix goes stale after the move; the constitution says lookup is by test **name**, which is unchanged, and the files are verbatim records (SPEC-2), so they are not edited. SPEC-3 will pass over them.
- **Moved E2E tests lose their on-disk assertions.** Covered elsewhere: draft exclusion on disk by `fixture_excludes_drafts_everywhere`, the fixture manifest and `rebuild_removes_newly_drafted_page`; `recent_count` on disk by `fixture_orders_listings_and_caps_recent`.
- **AC-5.2 relies on `read_dir` order being repeatable** within one process for an unchanged folder. It is on every mainstream filesystem, and `fixture_build_is_deterministic` already depends on the same property across processes.
- **Parallel in-process tests share one cwd.** No test changes it; every test uses its own folder under `CARGO_TARGET_TMPDIR`; `commit`'s cwd check compares against the crate root, which is never inside a test's output folder.
- **Windows:** `clean_contents_removes_dir_symlink` moves unchanged under `#[cfg(windows)]`; the `./<site>` safety-net test builds its expected string with `Path::new(".").join(..).display()` so it holds for `.\nowhere` too.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- Safety net (baseline criteria in code that moves, `cli.rs`):
  - AC-1.1 / AC-1.3 (full failure precedence): only the config-before-templates pair is pinned today (`config_error_reported_before_templates_error`). **T-1** adds `build_reports_first_failure_in_pipeline_order` (E2E).
  - AC-3.3 (`./<site>` form): no test. **T-2** adds `build_names_relative_site_path_with_dot_prefix` (E2E; ARCH-7 replaces it when it removes the join).
  - AC-1.2: `failed_rebuild_keeps_previous_output`, `page_render_error_keeps_previous_output`, `section_render_error_keeps_previous_output`, `build_fails_on_page_and_section_output_collision`, `build_fails_on_file_vs_folder_conflict_keeping_output`, `build_fails_on_missing_assets_folder_keeping_output`, `build_fails_when_assets_path_has_no_name`, `build_refuses_*` — existing, unchanged.
  - AC-1.5: `refuses_cwd`, `refuses_parent_of_cwd`, `refuses_protected_path_or_its_parent` (includes the skipped-missing-path quirk), `missing_target_is_safe_and_clean_contents_is_noop` — moved intact in T-3.
  - AC-1.6: `clean_contents_empties_but_keeps_dir`, `clean_contents_errors_when_target_is_a_file`, `clean_contents_removes_dir_symlink` — moved intact in T-3.
  - AC-1.4: none needed — a documented limitation with no test by decision (spec "Out of scope").
  - AC-1.7: none needed — structural (the plan holds `Vec<RenderedFile>`); `plan_holds_every_output_fully_rendered` observes it.
  - AC-3.1/3.2/3.4/3.5/3.6: `help_shows_flag_descriptions`, `build_without_config_uses_defaults`, `build_reads_default_mango_json_from_cwd`, `build_fails_when_explicit_config_missing`, `clean_*`, `run_command_*`, `publish_is_not_a_command`, every `assert_failure` — existing, unchanged.
- New and changed behavior (`tests/plan.rs`, tags in spec-scoped form; a moved test keeps its legacy tag and gains the new one):
  1. `page_and_section_index_collision_fails_planning` — AC-7.1, AC-5.4, AC-4.4 (asserts the exact string `Mango Error: output path '<out>/posts/index.html' would be written by both page 'posts' and section index 'posts'`).
  2. `file_vs_folder_conflict_fails_planning` — AC-7.6 (`feed.xml.md` with `base_url`).
  3. `first_failure_is_reported_in_pipeline_order` — AC-4.4 (bad config + missing templates → `MangoError::Config`, message does not name the templates path).
  4. `planning_creates_modifies_and_deletes_nothing` — AC-4.2, AC-7.2 (`tree()` of site, templates, assets and a pre-populated output folder, before/after a failing plan and a succeeding plan).
  5. `plan_holds_every_output_fully_rendered` — AC-4.3, AC-5.1, AC-1.7 (exact enumerated path list with and without `base_url`; `File` contents equal the template output; `Copy` source is the asset path).
  6. `planning_twice_gives_identical_outputs` — AC-5.2.
  7. `commit_writes_exactly_the_enumerated_outputs` — AC-5.3, AC-4.6 (stray file and folder in the output beforehand; afterwards the tree equals the enumeration byte for byte, folder kept).
  8. `commit_refuses_output_containing_an_input_and_touches_nothing` — AC-4.7, AC-4.8 (output = the folder holding `site/`; `plan` succeeds, `commit` fails with `refusing to clean output path '…': it is or contains the input path '…'`, tree unchanged).
  9. `section_lists_newest_first_undated_last_then_title_then_slug` — AC-7.7 (two same-date pages, two same-date-same-title pages with different slugs, one undated).
  10. `home_recent_respects_recent_count_and_skips_undated` — **moved** from `tests/build.rs`; tag `// AC-2.2` carried + AC-7.8.
  11. `build_excludes_draft_pages` — **moved** from `tests/build.rs`; tags `// AC-3.2 (batch 1); AC-4.7` carried + AC-7.9; extended from the section index to home, tag index, tag pages, feed, sitemap and the output path list.
  12. `draft_with_bad_frontmatter_date_or_tag_fails_planning` — AC-7.11 (three drafts, each `Err` naming the file and the value).
  13. `clean_is_reachable_through_the_library` — AC-6.3 (existing folder removed, missing folder `Ok`).
- E2E contract kept (AC-7.3, AC-8.1–8.3): every `fixture_*` test, `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `fixture_internal_links_resolve`, `config_error_reported_before_templates_error` and all snapshot tests listed under AC-1.2 above stay byte-identical in `tests/build.rs`. T-7 verifies with `git diff <branch_point> -- tests/build.rs`.
- **Moved or deleted E2E tests (AC-7.10):**

| E2E test (removed from `tests/build.rs`) | Legacy tag | Behaviour now covered by |
|---|---|---|
| `home_recent_respects_recent_count_and_skips_undated` | `// AC-2.2` (batch 3) | same name in `tests/plan.rs` (tag carried); on-disk `recent_count` by `fixture_orders_listings_and_caps_recent` |
| `build_excludes_draft_pages` | `// AC-3.2 (batch 1); AC-4.7` (batch 2) | same name in `tests/plan.rs` (tags carried); on-disk exclusion by `fixture_excludes_drafts_everywhere`, `build_generates_site_from_fixture`, `rebuild_removes_newly_drafted_page`; batch-2 AC-4.7 ("missing target is safe") also by `missing_target_is_safe_and_clean_contents_is_noop` |

No test is deleted outright. Unit tests moved from `src/cli.rs` to `src/build/clean.rs` keep their names and tags (`refuses_cwd`, `refuses_parent_of_cwd`, `refuses_protected_path_or_its_parent`, `missing_target_is_safe_and_clean_contents_is_noop`, `clean_contents_errors_when_target_is_a_file`, `clean_contents_empties_but_keeps_dir`, `clean_contents_removes_dir_symlink`).

## Tasks
### T-1 Safety net: pin the failure precedence of the build order
- Kind: safety-net
- Satisfies: AC-1.1, AC-1.2, AC-1.3, AC-8.3
- Files: `tests/build.rs`
- Done when: `build_reports_first_failure_in_pipeline_order` (tag `// AC-arch-1.1.1; AC-arch-1.1.2; AC-arch-1.1.3; AC-arch-1.8.3`) builds a temp site with a frontmatter-less page, malformed `--config`, missing `--templates`, missing `--assets` and a marker file in the output; asserts, fixing one input at a time, that stderr names in turn the bad page (not the config), the config (not the templates), the templates (not the assets), the assets, with `snapshot(&out)` unchanged each time, and finally succeeds and removes the marker. Passes on the baseline code.

### T-2 Safety net: pin the `./<site>` error form
- Kind: safety-net
- Satisfies: AC-3.3
- Files: `tests/build.rs`
- Done when: `build_names_relative_site_path_with_dot_prefix` (tag `// AC-arch-1.3.3`) runs `build --site nowhere` from a temp cwd with absolute fixture templates/assets and an absolute `-o`; asserts exit 1, stderr contains `format!("{} is not a directory", Path::new(".").join("nowhere").display())`, and the output folder does not exist. Passes on the baseline code.

### T-3 Move the clean and safety logic into the crate (`src/build/clean.rs`)
- Kind: implementation
- Satisfies: AC-1.5, AC-1.6, AC-3.4, AC-6.3
- Files: `src/build/clean.rs`, `src/build/mod.rs`, `src/cli.rs`
- Done when: `ensure_safe_to_clean`, `clean_contents`, `current_dir` (`pub(crate)`) and `clean` (`pub`) live in `clean.rs` with bodies unchanged; the seven unit tests move with their tag comments, using `target/unit-fixtures/clean/`; `cli.rs` calls them through `crate::build::clean`; the gate passes and every E2E test is unchanged.

### T-4 The seam: `pipeline.rs`, `lib.rs`, and a parse-and-dispatch `cli.rs`
- Kind: implementation
- Satisfies: AC-4.1, AC-4.3, AC-4.4, AC-4.5, AC-4.6, AC-4.7, AC-4.8, AC-4.9, AC-4.10, AC-5.1, AC-5.4, AC-1.7, AC-2.1, AC-3.1, AC-3.2, AC-3.3, AC-3.5, AC-3.6, AC-6.1, AC-6.2, AC-6.4, AC-6.5, AC-8.2
- Files: `src/build/pipeline.rs`, `src/build/mod.rs`, `src/lib.rs`, `src/main.rs`, `src/cli.rs`, `src/build/output.rs`, `src/error.rs`
- Done when: `pipeline.rs` defines the types and functions exactly as in "Interfaces and data", `plan()` being `cli.rs:90-152` and `commit()` being `cli.rs:154-170` with the `./`-join and default-config behaviour preserved; `lib.rs` declares five private modules and the seven re-exports only; `main.rs` is `mod cli;` plus `main()`; `cli.rs` holds only the clap structs, `run()`, `into_options()` and the `not implemented` arm, and `build()`/`clean()` and the safety/clean helpers are gone from it; `RenderedFile` derives `Debug`; `MangoError::io_at` is `pub(crate)`; `Cargo.toml` is untouched; `grep -n "fs::\|remove_\|write(" src/build/pipeline.rs` shows no filesystem mutation outside `commit`; the gate passes with every test in `tests/build.rs` unchanged.

### T-5 In-process tests (`tests/plan.rs`) and the two moves
- Kind: test
- Satisfies: AC-7.1, AC-7.2, AC-7.4, AC-7.5, AC-7.6, AC-7.7, AC-7.8, AC-7.9, AC-7.10, AC-7.11, AC-4.2, AC-4.3, AC-4.4, AC-4.6, AC-4.7, AC-4.8, AC-5.1, AC-5.2, AC-5.3, AC-5.4, AC-6.3, AC-2.2, AC-2.3
- Files: `tests/plan.rs`, `tests/build.rs`
- Done when: the 13 tests listed under "Test strategy" exist with the stated tags (`// AC-arch-1.<req>.<m>`, moved tests also keeping their legacy tag), use only `mango::{plan, commit, clean, BuildOptions, BuildPlan, PlannedOutput, MangoError}`, never spawn the binary or change the cwd; `home_recent_respects_recent_count_and_skips_undated` and `build_excludes_draft_pages` are removed from `tests/build.rs` and nothing else there changes; `cargo clippy --all-targets -- -D warnings` is clean for the new test crate (no unused helper).

### T-6 Docs and backlog
- Kind: docs
- Satisfies: AC-9.1, AC-9.2, AC-9.3, AC-9.4, AC-8.4, AC-1.4
- Files: `CLAUDE.md`, `specs/_system/overview.md`, `specs/_system/backlog.md`
- Done when: `CLAUDE.md`'s `build` bullet describes `plan` → `commit` and restates the guarantee as "`commit` accepts only a `BuildPlan`, and only a successful `plan` produces one" (keeping the post-clean I/O limitation sentence); the `clean` bullet points at `build/clean.rs`; the module map has `src/lib.rs` (public surface = the seven re-exports; internal seam: may change freely, not documented in `README.md`, no semver promise), `src/main.rs`, binary-only `src/cli.rs`, `src/build/pipeline.rs`, `src/build/clean.rs`; the tests convention mentions `tests/plan.rs`; `overview.md` line 13 reads that the crate has a library target (`src/lib.rs`) and a binary (`src/main.rs` + `src/cli.rs`) with no other line touched; the backlog index row and ARCH-1 header say `done` with a short landed paragraph, and the FEAT-1, ARCH-2 and SPEC-3 dependency notes still read correctly; `README.md` has no diff.

### T-7 Gate and contract check
- Kind: test
- Satisfies: AC-6.6, AC-7.3, AC-8.1, AC-8.2, AC-8.3, AC-4.10
- Files: none (verification only; `tests/build.rs` is inspected, not edited)
- Done when: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` passes; `git diff <branch_point> -- tests/build.rs` contains only the two T-1/T-2 additions and the two T-5 removals; `git diff <branch_point> -- README.md Cargo.toml example/` is empty; `grep -rn "allow(dead_code)" src tests` is empty.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-1, T-4 |
| AC-1.2 | T-1, T-4, T-7 |
| AC-1.3 | T-1, T-4 |
| AC-1.4 | T-6 (documented limitation kept; no test by spec decision) |
| AC-1.5 | T-3 |
| AC-1.6 | T-3 |
| AC-1.7 | T-4, T-5 |
| AC-2.1 | T-4 (superseded by AC-6.1: the library target is added) |
| AC-2.2 | T-5, T-7 (`tests/build.rs` still only spawns the binary) |
| AC-2.3 | T-3, T-5 (unit tests kept intact; the "no in-process pipeline test" clause is superseded by REQ-7) |
| AC-3.1 | T-4, T-7 |
| AC-3.2 | T-4, T-7 |
| AC-3.3 | T-2, T-4 |
| AC-3.4 | T-3, T-7 |
| AC-3.5 | T-4, T-7 |
| AC-3.6 | T-4, T-7 |
| AC-4.1 | T-4 |
| AC-4.2 | T-4, T-5 |
| AC-4.3 | T-4, T-5 |
| AC-4.4 | T-4, T-5, T-7 |
| AC-4.5 | T-4 (public-surface table in this design; review, per the approved exception) |
| AC-4.6 | T-4, T-5 |
| AC-4.7 | T-4, T-5 |
| AC-4.8 | T-4, T-5 |
| AC-4.9 | T-4 |
| AC-4.10 | T-4, T-7 |
| AC-5.1 | T-4, T-5 |
| AC-5.2 | T-5 |
| AC-5.3 | T-5 |
| AC-5.4 | T-4, T-5 |
| AC-6.1 | T-4 |
| AC-6.2 | T-4 |
| AC-6.3 | T-3, T-5 |
| AC-6.4 | T-4 |
| AC-6.5 | T-4, T-7 |
| AC-6.6 | T-7 |
| AC-7.1 | T-5 |
| AC-7.2 | T-5 |
| AC-7.3 | T-7 |
| AC-7.4 | T-3, T-5 |
| AC-7.5 | T-5 |
| AC-7.6 | T-5 |
| AC-7.7 | T-5 |
| AC-7.8 | T-5 |
| AC-7.9 | T-5 |
| AC-7.10 | T-5 (the moved-tests table in this design) |
| AC-7.11 | T-5 |
| AC-8.1 | T-7 |
| AC-8.2 | T-4, T-7 |
| AC-8.3 | T-1, T-7 |
| AC-8.4 | T-6 |
| AC-9.1 | T-6 |
| AC-9.2 | T-6 |
| AC-9.3 | T-6 |
| AC-9.4 | T-6 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-22 | 20260922-224052-arch-1-split-plan-commit | Initial design: library crate with a private module tree and seven re-exports; `plan`/`commit` seam in `src/build/pipeline.rs`, clean and safety logic in `src/build/clean.rs`, clap kept in the binary's `src/cli.rs`; in-process tests in `tests/plan.rs` (two moved from E2E); two safety-net E2E tests; docs and backlog updates. |
