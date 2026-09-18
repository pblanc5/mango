# Backlog

Tracked improvements for mango. Each item can be handed to the dev pipeline as-is: `/spec-feature specs/_system/backlog.md#arch-1` for design-heavy work, `/ship-feature` for local changes. When an item is picked up, set its status to `in progress` and link the spec; when it is merged, set `done`. Name the item ID in the commit subject (e.g. `… (RISK-2)`) so `git log --grep` finds the change — don't record commit hashes here, since they can't be written in the commit they describe and go stale on a rebase.

Statuses: `open`, `in progress`, `done`, `dropped`. Sizes: **S** (one module, under a day), **M** (a few modules), **L** (cross-cutting).

## Index

| ID | Title | Priority | Size | Workflow | Depends on | Status |
|---|---|---|---|---|---|---|
| [FEAT-1](#feat-1) | Dev server (`run`) | medium | L | `/spec-feature` | ARCH-1, a dependency approval | open |
| [FEAT-2](#feat-2) | `publish`: dropped, never implemented | — | — | — | — | dropped |
| [ARCH-1](#arch-1) | Split the build into plan and commit; add `lib.rs` | high | L | `/spec-feature` | — | open |
| [ARCH-2](#arch-2) | One output model instead of three | high | L | `/spec-feature` | ARCH-1, ARCH-3 | open |
| [ARCH-3](#arch-3) | `Slug` and `Tag` newtypes | high | M | `/spec-feature` | — | open |
| [ARCH-4](#arch-4) | Structured error variants instead of `General(String)` | medium | M | `/ship-feature` | — | open |
| [ARCH-5](#arch-5) | Break up `render/template.rs` | medium | M | `/ship-feature` | ARCH-2 (easier after) | open |
| [ARCH-6](#arch-6) | Tidy the `Page` model | medium | S | `/ship-feature` | — | open |
| [ARCH-7](#arch-7) | Smaller cleanups: frontmatter return type, CLI path types | low | S | `/ship-feature` | — | open |
| [OPS-1](#ops-1) | Continuous integration | medium | S | `/ship-feature` | a git remote | open |
| [SPEC-1](#spec-1) | Confirm the constitution's open proposals | medium | S | manual | — | open |
| [RISK-1](#risk-1) | Symlink loops in the site folder | medium | S | `/ship-feature` | — | done |
| [RISK-2](#risk-2) | Symlinked folders inside the assets folder | medium | S | `/ship-feature` | — | done |
| [RISK-3](#risk-3) | Raw HTML in content is trusted but undocumented | low | S | `/ship-feature` | — | open |
| [RISK-4](#risk-4) | Unknown frontmatter keys are silently ignored | medium | S | `/spec-feature` | — | open |
| [RISK-5](#risk-5) | Unresolvable symlinks under `site/` are silently ignored | low | S | `/ship-feature` | — | open |
| [TEST-1](#test-1) | No test for CRLF line endings | low | S | `/ship-feature` | — | done |
| [DOC-1](#doc-1) | Stale line references in the system overview | low | S | `/ship-feature` | — | open |

Recommended order: ARCH-1 → ARCH-3 → ARCH-2, then ARCH-4 to ARCH-7 in any order. FEAT-1 comes after ARCH-1, which gives it the seam it needs; FEAT-2 is closed as dropped. The RISK and TEST items are independent and can be done at any time.

---

## Architecture

Source: architecture review of 2026-09-14 (commit `8f2eec7`). The review found the core design sound (stage-per-module layout, the `RenderItem` seam, view models separate from `Page`, a single error type); these items address where that design has been stretched rather than extended.

### ARCH-1
**Split the build into plan and commit; add `lib.rs`** · high · L · `/spec-feature` · open

**Problem.** `cli::build` (`src/cli.rs`) parses paths, loads, indexes, generates, checks collisions, renders, checks safety, cleans and writes in one function of about 80 lines. The project's most important guarantee, that nothing touches the output folder until everything has succeeded, is enforced only by statement order and a comment; a new line in the wrong place silently breaks it. There is no library crate, so the pipeline can only be tested by spawning the binary, which is why `tests/build.rs` is about 1,900 lines.

**Proposal.**
```rust
// src/lib.rs
pub fn plan(opts: &BuildOptions) -> Result<BuildPlan, MangoError>;   // load, render, check; no writes
pub fn commit(plan: BuildPlan, dist: &Path) -> Result<(), MangoError>; // safety check, clean, write, copy
```
`main.rs` parses arguments and calls the library. `BuildPlan` holds every output, fully rendered, plus the paths the safety check must protect.

**Done when.**
- The only way to write output is `commit(BuildPlan)`, and a `BuildPlan` can only come from a successful `plan`.
- `cli.rs` contains argument parsing and dispatch only.
- At least the collision, ordering and draft behaviors are tested in-process against `plan`, without spawning the binary; E2E tests keep covering exit codes and stderr.
- All existing tests pass; CLAUDE.md's pipeline description is updated.

**Also enables.** The future `run` dev server can call `plan` on every change; parallel rendering becomes a local change.

### ARCH-2
**One output model instead of three** · high · L · `/spec-feature` · depends on ARCH-1, ARCH-3 · open

**Problem.** Outputs come in three shapes: `RenderItem` (slug + template + context), `GeneratedFile` (path + text) and `AssetFile` (source → destination). `check_collisions`, rendering and writing each handle the three separately. Two symptoms: `source: String` is a free-text label that exists only for error messages, and `page_date: Option<NaiveDate>` was bolted onto `RenderItem` so the sitemap could find `lastmod`.

**Proposal.**
```rust
enum ItemKind { Page { date: Option<NaiveDate> }, Section, Home, TagIndex, Tag(Tag), Feed, Sitemap, Asset }
enum Body { Template { name: &'static str, context: tera::Context }, Text(String), Copy(PathBuf) }
struct Output { path: OutputPath, kind: ItemKind, body: Body }
```
Collision labels become `Display for ItemKind`; the sitemap selects `ItemKind::Page { date }`; collision checking, rendering and writing each work over one `Vec<Output>`.

**Done when.**
- `RenderItem`, `GeneratedFile` and `AssetFile` are replaced by one type; `source` strings and `page_date` are gone.
- Collision error messages are unchanged (existing tests pass without edits to their expected text).
- The sitemap no longer depends on a field that exists only for it.

### ARCH-3
**`Slug` and `Tag` newtypes** · high · M · `/spec-feature` · open

**Problem.** Slugs are plain `String`s and their logic is spread across five files: construction and validation in `content/page.rs` (`generate_slug`, `slug_url`, `tag_slug`), path building in `build/output.rs` (`get_final_path`), parent lookup in `build/index/section.rs` (`extract_parent_slug` with `rsplit_once`), top-level detection in `build/generate/home.rs` (`!slug.contains('/')`), and URL building in `render/template.rs` (five `slug_url` calls). Tags are `String`s validated once and then trusted by convention.

**Proposal.** A `Slug` type, only constructible through validation, with `parent()`, `is_top_level()`, `segments()`, `url()` and `output_path(dist)`; a `Tag` type constructed by `validate_tags`, with `url()` and `slug()`.

**Done when.**
- No module outside the `Slug`/`Tag` implementation splits, joins or formats slug strings.
- An invalid slug or tag cannot be constructed.
- Template contexts and output are byte-identical (the fixture and determinism tests pass unchanged).

### ARCH-4
**Structured error variants instead of `General(String)`** · medium · M · `/ship-feature` · open

**Problem.** `MangoError::General(String)` covers collisions, invalid file names, clean-safety refusals, a missing templates folder and unimplemented commands. Messages are built with `format!` at each call site, and the loader prepends file paths with a `with_path` closure. Tests therefore assert on substrings of stderr.

**Proposal.** Variants such as `Collision { path, first, second }`, `FileConflict { path, file, dir }`, `InvalidFileName { path, segment }`, `UnsafeClean { target, reason }`, `NotADirectory { path }`, `NotImplemented { command }`, and `Frontmatter { path, reason }` carrying the path. Keep the `io_at` pattern, which already works this way.

**Done when.**
- `General` is removed or used only for truly uncategorized errors.
- Every message's wording lives in `src/error.rs`.
- Unit tests match on variants; E2E tests still check that stderr names the file and value.

### ARCH-5
**Break up `render/template.rs`** · medium · M · `/ship-feature` · easier after ARCH-2 · open

**Problem.** The largest source file (about 280 non-test lines) does four jobs: loading Tera, defining every view model, constructing every `RenderItem`, and running markdown (`render_page` calls `to_html`). `RenderItem` is a build-output concept, but `build` depends on `render` for it. Small smells: `PageTemplate::add_content` rebuilds the whole struct to set one field, and `use tera::Context;` is repeated inside each constructor.

**Proposal.** `render/context.rs` for view models, `render/template.rs` for Tera loading only, item construction moved into `build/generate/*`, the output type moved into `build/` (or replaced by ARCH-2's `Output`).

**Done when.** Each file has one job, `build` no longer imports its core type from `render`, and output is byte-identical.

### ARCH-6
**Tidy the `Page` model** · medium · S · `/ship-feature` · open

**Problem.**
- `PageType::General` is the only variant and nothing reads `Page.kind`; under the no-dead-code rule it should go until a second page type exists.
- Construction is two-phase: `Page::new` leaves `slug` empty and `generate_slug(&mut self)` fills it in later, so a `Page` without a slug can exist.
- `compare_summaries` (the listing order) lives in `build/index/section.rs` but is used by sections, tags, the home page and the feed; it is a content rule.
- `home::recent_pages` builds `PageSummary`s only to sort them, then discards them, and `feed::build` re-checks `page.date` with an unreachable `continue`.

**Proposal.** Remove `PageType`; construct `Page` from path + site + frontmatter in one step; move the ordering next to `Page` (sorting on `&Page` directly); have `recent_pages` return pages paired with their date so the feed needs no `let … else`.

**Done when.** No unused type or field remains on `Page`, every `Page` has a valid slug from construction, and ordering has one home.

### ARCH-7
**Smaller cleanups** · low · S · `/ship-feature` · open

- `frontmatter::parse` returns `Option` for "no frontmatter", which the loader immediately turns into an error. Return the error from `parse` instead.
- CLI path options are `String`s converted with `Path::new`; declare them as `PathBuf`. `project_path` (`"."`) is joined onto `site` but not onto `templates` or `assets`; remove it.

**Done when.** Both are changed with behavior and messages unchanged.

---

## Features

Product-level work, as opposed to the refactoring that makes up the rest of this backlog. `run` is declared in `src/cli.rs` and returns `MangoError::General("the 'run' command is not implemented yet")` with exit status 1; `README.md` lists it under "Known limitations". `publish` was declared the same way and has been removed — see FEAT-2 for why.

### FEAT-1
**Dev server (`run`)** · medium · L · `/spec-feature` · depends on ARCH-1 and a dependency approval · open

**Problem.** `Commands::Run` takes no options and errors out. For a static site generator the edit-and-refresh loop is the day-to-day workflow, and without it every change means re-running `mango build` by hand and reloading the browser. `run_command_is_not_implemented_error` and `run_command_takes_no_options` pin the stub; both must be consciously replaced when this lands, not quietly deleted.

**Settle these in the spec, before any code.**
- **Which dependency, and is it approved?** A dev server needs an HTTP server and almost certainly a file watcher. `tide` was dropped rather than kept for this, and "no new dependencies without the maintainer's explicit approval" is a non-negotiable, so this is the first gate. `CLAUDE.md` is explicit that `run`'s flags and its server dependency are added when it is implemented, not before.
- **What is served.** `dist/` on disk, or the in-memory `BuildPlan` with nothing written at all. Serving the plan avoids cleaning and rewriting the output folder on every keystroke, which is the main reason ARCH-1 comes first.
- **Rebuild strategy.** Full rebuild per change (simple, and cheap once `plan()` exists) or incremental.
- **Live reload** (inject a socket and refresh the page) or serve-and-reload-by-hand.
- **Flags.** At least `--port`; plus the existing `--site`/`--templates`/`--assets`/`-o`/`--config`.

**Done when.** `mango run` serves the built site over HTTP and rebuilds when content, templates, assets or the config change. A failed rebuild keeps serving the last good output and reports the error on stderr rather than exiting — the same "never destroy good output on bad input" guarantee `build` already makes. The stub tests are replaced by tests of the real behavior, and `README.md` documents the flags and any limitations.

### FEAT-2
**`publish`: dropped, never implemented** · — · — · — · — · dropped

**Decided by the maintainer (2026-09-17): mango will not have a `publish` command.** The subcommand was declared in `src/cli.rs` early on and never had a meaning behind it — no target, protocol or requirement for it existed anywhere in the repo, and it did nothing but return "the 'publish' command is not implemented yet". Rather than specify one after the fact, it was removed: the enum variant, its dispatch arm, its help text and its stub test are gone, so `mango publish` is now a clap "unrecognized subcommand" error rather than a promise mango was not keeping.

**Why dropped rather than specified.** Every plausible reading — commit `dist/` to a `gh-pages`-style branch, rsync or SFTP to a host, upload to object storage, call a host's deploy API — puts mango into the credential-handling and deployment business, needs at least one new dependency against a standing non-negotiable, and duplicates tooling site authors already have. The one reading that avoided all of that, shelling out to a command configured in `mango.json`, is a wrapper thin enough that the author can just run the command. mango's job ends at a correct, deterministic `dist/`.

**Pinned by** `publish_is_not_a_command` in `tests/build.rs`, which asserts the command is rejected as unknown, that the old stub error is gone, and that `--help` no longer advertises it. If publishing is ever wanted, it starts as a new item with a decided scope, not as a resurrected stub.

---

## Operations and specs

### OPS-1
**Continuous integration** · medium · S · `/ship-feature` · depends on a git remote · open

No CI exists because the repository has no remote. Once one is added, run the definition-of-done gate (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`) on every push, on Linux and Windows (the code has Windows-specific paths in slug handling and `clean_contents`).

### SPEC-1
**Confirm the constitution's open proposals** · medium · S · manual · open

`specs/constitution.md` has items marked **Proposed, confirm**:
- New specs start at acceptance-criteria group `AC-10`.
- "Keep diffs reviewable by formatting only what you change."
- The `/spec-feature` vs `/ship-feature` guidance.

Confirm or change each, then remove the markers.

**Evidence for the `AC-10` decision, found 2026-09-17 while dropping FEAT-2.** The constitution notes that "IDs have been reused across earlier batches", which understates it on two counts, and confirming `AC-10` fixes neither.

- **Existing IDs collide, and mostly can't be disambiguated.** `AC-7.3` currently means three different things: "entries are sorted by `loc`, compared as plain strings" (`src/build/generate/sitemap.rs:152`, batch 4), "`Cargo.toml` dependencies are unchanged" (an earlier batch), and, until FEAT-2, "`mango publish` exits non-zero with the not-implemented message" (`tests/build.rs`). Some tags carry a `(batch N)` qualifier and most do not: 56 qualified against 125 bare, so the qualifier cannot be relied on to tell two uses of an ID apart.
- **`AC-10` is already taken.** The proposal is that new specs start at group `AC-10`, but `AC-10.1` through `AC-10.9` are in use today across `src/build/generate/assets.rs` and `tests/build.rs`. Whatever is confirmed has to name a group that is actually free, or say how the next free one is found.
- **The definitions are not in the repository.** Every criterion from the old `tasks.md`-driven batches is defined only in `.dev-pipeline/runs/**/01-plan*.md`, and `.dev-pipeline/` is gitignored (`.gitignore:5`). From a fresh clone, every `// AC-<group>.<n>` comment in `src/` and `tests/` resolves to nothing.

This is a legacy problem, not an ongoing one: `spec-feature.yaml` publishes to `specs/{spec_id}/requirements.md`, a tracked path, so criteria from spec-driven runs are resolvable. So the decision to make alongside `AC-10` is what happens to the legacy tags — retire them, qualify each with its batch, or leave them as archaeology and accept that they don't resolve. Retiring the tag on a **withdrawn** criterion is already precedent: FEAT-2's test carries no AC ID, because the criterion it named was deleted rather than changed.

---

## Risks and gaps

Source: `specs/_system/overview.md`, "Risky areas" (items marked **Inferred, confirm**), and the audit of 2026-09-14.

### RISK-1
**Symlink loops in the site folder** · medium · S · `/ship-feature` · done

`loader::traverse` recursed with `path.is_dir()`, which follows symlinks, and had no cycle guard: a symlink pointing at an ancestor folder was followed and the whole site re-walked beneath it, silently publishing up to ~40 duplicated copies of every page under bogus nested URLs with exit status 0. (The reported symptom, a stack overflow, does not happen on Linux: the kernel's 40-symlink-per-resolution limit stops the recursion first and `is_dir()` swallows the resulting `ELOOP`, so the corrupt build succeeded without a word — worse than an abort, because nothing signalled it.) Resolved by rejecting symlinked folders rather than detecting cycles: `traverse` now classifies entries with `fs::metadata` plus a non-following `is_symlink` check and fails with `content folder '<rel>' is a symlink to a directory`, so no cycle guard is needed for symlinks (a cycle among real directories, such as a bind mount or a Windows junction, would still recurse unbounded; that is out of scope, not impossible); symlinked markdown files are still read. `content/loader.rs::tests::load_rejects_symlink_loop_to_ancestor` and `tests/build.rs::build_fails_on_symlinked_content_folder_keeping_output` prove the loop case fails cleanly and leaves the previous output intact. Unresolvable links stay silently ignored, unlike in `assets::plan`; that difference is deferred to [RISK-5](#risk-5).

### RISK-2
**Symlinked folders inside the assets folder** · medium · S · `/ship-feature` · done

`assets::collect` used `DirEntry::file_type()`, which does not follow symlinks, so a symlink to a folder was planned as a file and `fs::copy` failed after the output folder was cleaned. Resolved by rejecting it before cleaning: `assets::plan` now classifies entries with `fs::metadata`, fails on a symlinked folder (`asset '<rel>' is a symlink to a directory`) and on an unresolvable link, and `tests/build.rs::build_fails_on_symlinked_asset_folder_keeping_output` proves the previous output survives.

### RISK-3
**Raw HTML in content is trusted but undocumented** · low · S · `/ship-feature` · open

pulldown-cmark passes raw HTML through and templates print `page.content | safe`, so content authors can inject any HTML. That is normal for a static site generator, but `README.md` does not say so. Document it under "Known limitations" (or add an option to strip raw HTML if untrusted content is ever a use case).

### RISK-4
**Unknown frontmatter keys are silently ignored** · medium · S · `/spec-feature` · open

`MangoFrontmatter` lacks `deny_unknown_fields` (unlike `SiteConfig`), so a typo such as `"tag"` or `"dates"` is dropped without warning. Making it strict is a user-visible behavior change for existing sites, so specify it: which fields exist, the error message, and whether drafts are checked.

### RISK-5
**Unresolvable symlinks under `site/` are silently ignored** · low · S · `/ship-feature` · open

An entry under `site/` whose target cannot be resolved — a dangling symlink, or a chain that loops (`a -> b -> a`) — is skipped without a word by `loader::traverse`, which is the behavior inherited from `path.is_dir()`/`path.is_file()` swallowing I/O errors. `assets::plan` already treats both as `io_at` build errors (RISK-2), so the two modules deliberately differ. Making the loader strict is a user-visible behavior change for existing sites and has no bearing on the runaway traversal, so it was deferred from RISK-1. Decide whether a broken or looping link under `site/` should fail the build (and whether a warning is enough), then align the two modules or record why they differ. `content/loader.rs::tests::load_ignores_dangling_symlink` and `load_ignores_symlink_loop_chain` pin the current behavior.

### TEST-1
**No test for CRLF line endings** · low · S · `/ship-feature` · done

Frontmatter parsing handled `\r\n` (verified by hand during the audit) but nothing tested it. Pinned with characterization tests only — no production change, and the audit's claim held on every point. Six unit tests in `content/frontmatter.rs` cover a wholly-CRLF document (`parses_crlf_frontmatter_and_normalizes_body`), CRLF/LF equivalence (`crlf_and_lf_documents_parse_identically`), a BOM combined with CRLF (`byte_order_mark_with_crlf_is_ignored`), endings mixed line by line (`parses_mixed_lf_and_crlf_line_endings`) and an unterminated CRLF block (`unterminated_crlf_block_is_an_error`). The behavior pinned: `str::lines` strips the `\r`, so the delimiters match and the JSON block is re-joined with `\n`, and the body is normalized to LF with its final newline dropped. `crlf_body_is_returned_verbatim_when_no_frontmatter` records the one asymmetry — the "no frontmatter" early return hands the content back verbatim, CRLF and all. It is unreachable from the CLI (`loader::load` turns `None` into the missing-frontmatter error), so it was pinned rather than fixed; [ARCH-7](#arch-7) rewrites that return and will retire the test.

End to end, `tests/build.rs::build_accepts_crlf_line_endings` builds a CRLF temp site (exact output manifest, `<time datetime="2026-01-24">` and `/tags/crlf/` proving no `\r` leaked into a JSON date or tag, no `\r` in the rendered content region, `feed.xml` or `sitemap.xml`), and `crlf_and_lf_sites_build_identical_output` compares that site byte for byte with its LF twin, feed and sitemap included. **What each one can catch**, measured by mutating the body's `join("\n")` to `join("\r\n")` in an isolated copy: five unit tests fail, and `build_accepts_crlf_line_endings` fails too. The E2E test detects it through one deliberate construct on the test page — a raw *inline* HTML tag split across two lines. pulldown-cmark normalizes line endings in almost everything it parses — paragraphs, fenced and indented code, HTML blocks, link titles and footnotes all came back identical under probes of some sixty constructs, run independently three times — but it passes raw inline HTML through verbatim. That and a multi-line code span, asserted next to it as a second and independent signal (it renders with two spaces instead of one), are the exceptions those probes found; the evidence is broad, not exhaustive, so treat the list as open. Should a future pulldown-cmark normalize inline HTML as well, the first assertion degrades to vacuous rather than failing, so the unit tests remain the primary guard. `crlf_and_lf_sites_build_identical_output` stays green under the mutation and always will: it is a symmetry property, and a regression in the shared join path perturbs the CRLF and LF builds alike — it guards against the two inputs being handled *differently*, not against normalization itself. Nothing was added to `example/site`: line endings in a committed file are not durable (`core.autocrlf`, `.gitattributes`), so every CRLF input is constructed in test code.

### DOC-1
**Stale line references in the system overview** · low · S · `/ship-feature` · open

`specs/_system/overview.md` cites line numbers (e.g. in `src/cli.rs` and `example/meta/templates/page.html`) that moved in later commits. Refresh them, or cite functions instead of lines so they stay valid; ARCH-1 to ARCH-5 will move most of them again, so this is best done after those land. The "Risky areas" rows for `loader::traverse` and `assets::collect` are also stale now that RISK-1 and RISK-2 are done; refresh them in the same pass.
