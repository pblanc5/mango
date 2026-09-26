# Backlog

Tracked improvements for mango. Each item can be handed to the dev pipeline as-is: `/spec-feature specs/_system/backlog.md#arch-1` for design-heavy work, `/ship-feature` for local changes. An item is not marked when it is picked up: the branch is that signal — `agents/claude/<id>-<short-desc>` for agent work, `users/patrick/<id>-<short-desc>` for work done by hand, per [the constitution](../constitution.md) — and the item goes `open` → `done` in the squash commit that lands it. Name the item ID in the commit subject (e.g. `… (RISK-2)`) so `git log --grep` finds the change — don't record commit hashes here, since they can't be written in the commit they describe and go stale on a rebase.

Statuses: `open`, `done`, `dropped`. Sizes: **S** (one module, under a day), **M** (a few modules), **L** (cross-cutting).

## Index

| ID | Title | Priority | Size | Workflow | Depends on | Status |
|---|---|---|---|---|---|---|
| [FEAT-1](#feat-1) | Dev server (`run`) | medium | L | `/spec-feature` | ARCH-1, a dependency approval | open |
| [FEAT-2](#feat-2) | `publish`: dropped, never implemented | — | — | — | — | dropped |
| [FEAT-3](#feat-3) | Agent skill for using mango as a tool | medium | M | `/spec-feature` | — | open |
| [FEAT-4](#feat-4) | Plain `key: value` frontmatter instead of JSON | medium | M | `/spec-feature` | — | open |
| [ARCH-1](#arch-1) | Split the build into plan and commit; add `lib.rs` | high | L | `/spec-feature` | — | done |
| [ARCH-2](#arch-2) | One output model instead of three | high | L | `/spec-feature` | ARCH-1, ARCH-3 | done |
| [ARCH-3](#arch-3) | `Slug` and `Tag` newtypes | high | M | `/spec-feature` | — | done |
| [ARCH-4](#arch-4) | Structured error variants instead of `General(String)` | medium | M | `/ship-feature` | — | open |
| [ARCH-5](#arch-5) | Break up `render/template.rs` | medium | M | `/ship-feature` | ARCH-2 (easier after) | open |
| [ARCH-6](#arch-6) | Tidy the `Page` model | medium | S | `/ship-feature` | — | open |
| [ARCH-7](#arch-7) | Smaller cleanups: frontmatter return type, CLI path types | low | S | `/ship-feature` | — | open |
| [OPS-1](#ops-1) | Continuous integration | medium | S | `/ship-feature` | a git remote | done |
| [OPS-2](#ops-2) | Release process, changelog and binaries | medium | S | manual | OPS-1 | done |
| [OPS-3](#ops-3) | Pipeline: evidence for design claims, and a route for design fixes | medium | S | manual | — | done |
| [OPS-4](#ops-4) | Spec critic wraps its artifact in a code fence | low | S | manual | — | open |
| [SPEC-1](#spec-1) | Confirm the constitution's open proposals | medium | S | manual | — | done |
| [SPEC-2](#spec-2) | Publish the legacy acceptance criteria into tracked specs | medium | M | `/ship-feature` | — | done |
| [SPEC-3](#spec-3) | Rewrite the legacy AC tags into spec-scoped form | low | M | `/ship-feature` | ARCH-1 | open |
| [RISK-1](#risk-1) | Symlink loops in the site folder | medium | S | `/ship-feature` | — | done |
| [RISK-2](#risk-2) | Symlinked folders inside the assets folder | medium | S | `/ship-feature` | — | done |
| [RISK-3](#risk-3) | Raw HTML in content is trusted but undocumented | low | S | `/ship-feature` | — | open |
| [RISK-4](#risk-4) | Unknown frontmatter keys are silently ignored | medium | S | `/spec-feature` | — | done |
| [RISK-5](#risk-5) | Unresolvable symlinks under `site/` are silently ignored | low | S | `/spec-feature` | — | done |
| [RISK-6](#risk-6) | Backslashes in file names become folder separators | low | S | `/spec-feature` | — | done |
| [RISK-7](#risk-7) | Asset-copy errors name the source, not the destination | low | S | `/ship-feature` | — | open |
| [RISK-8](#risk-8) | Content page order depends on the filesystem | low | S | `/spec-feature` | — | open |
| [RISK-9](#risk-9) | A JSON array is accepted as frontmatter | low | S | `/spec-feature` | — | open |
| [RISK-10](#risk-10) | Hidden files under `site/` are published | medium | S | `/spec-feature` | — | open |
| [TEST-1](#test-1) | No test for CRLF line endings | low | S | `/ship-feature` | — | done |
| [DOC-1](#doc-1) | Stale line references in the system overview | low | S | `/ship-feature` | — | done |
| [DOC-2](#doc-2) | Line-number citations in the constitution | low | S | `/ship-feature` | — | open |

Recommended order: ARCH-1 → ARCH-3 → ARCH-2, then ARCH-4 to ARCH-7 in any order. FEAT-1 comes after ARCH-1, which gives it the seam it needs; FEAT-2 is closed as dropped. FEAT-3 is independent and can be done at any time; if it lands before FEAT-1, FEAT-1 updates it. The RISK and TEST items are independent and can be done at any time. FEAT-4 would replace JSON frontmatter altogether, so decide on it before RISK-9, which it would make moot. OPS-3 changes only the dev pipeline and can be done at any time.

---

## Architecture

Source: architecture review of 2026-09-14 (commit `8f2eec7`). The review found the core design sound (stage-per-module layout, the `RenderItem` seam, view models separate from `Page`, a single error type); these items address where that design has been stretched rather than extended.

### ARCH-1
**Split the build into plan and commit; add `lib.rs`** · high · L · `/spec-feature` · done

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

**Landed.** The crate now builds a library (`src/lib.rs`) alongside the binary. Every module under `lib.rs` is private and the public surface is exactly seven re-exports — `plan`, `commit`, `clean`, `BuildOptions`, `BuildPlan`, `PlannedOutput`, `MangoError` — so a `pub` item nothing uses still trips the dead-code lint. `src/build/pipeline.rs` holds the seam: `plan(&BuildOptions)` loads, checks and renders everything while writing nothing, and `commit(BuildPlan)` is the only function that empties or writes the output folder. `BuildPlan`'s fields are private and `plan` is its only constructor, so "nothing is touched until everything has succeeded" is structural rather than positional; the plan is bound to the folder it was planned against (`commit` takes no destination, contrary to the sketch above) and `BuildPlan::outputs()` enumerates every file and asset copy for inspection. `ensure_safe_to_clean`, `clean_contents` and `mango clean` moved to `src/build/clean.rs` with their seven unit tests; `src/cli.rs` is binary-only and now contains nothing but clap definitions and dispatch. New `tests/plan.rs` has 13 in-process tests covering collisions (exact and file-vs-folder), failure precedence, the write-free guarantee, plan enumeration and determinism, commit's output and safety refusal, section ordering, `recent_count`, draft exclusion, draft validation and `clean`; `home_recent_respects_recent_count_and_skips_undated` and `build_excludes_draft_pages` moved there from `tests/build.rs`, which gained two safety-net tests (pipeline-order failure precedence and the `./<site>` error form) and is otherwise unchanged. No message, flag, output byte or exit code changed, and no dependency was added.

### ARCH-2
**One output model instead of three** · high · L · `/spec-feature` · depends on ARCH-1, ARCH-3 · done

**Problem.** Outputs come in three shapes: `RenderItem` (slug + template + context), `GeneratedFile` (path + text) and `AssetFile` (source → destination). `check_collisions`, rendering and writing each handle the three separately. Two symptoms: `source: String` is a free-text label that exists only for error messages, and `page_date: Option<NaiveDate>` was bolted onto `RenderItem` so the sitemap could find `lastmod`.

**Proposal.**
```rust
enum ItemKind { Page { date: Option<NaiveDate> }, Section, Home, TagIndex, Tag(Tag), Feed, Sitemap, Asset }
enum Body { Template { name: &'static str, context: tera::Context }, Text(String), Copy(PathBuf) }
struct Output { path: OutputPath, kind: ItemKind, body: Body }
```
Collision labels become `Display for ItemKind`; the sitemap selects `ItemKind::Page { date }`; collision checking, rendering and writing each work over one `Vec<Output>`. Since ARCH-3, `Tag` is `content::tag::Tag` and output locations come from `content::slug::Slug` (`Slug::output_path`), which `OutputPath` can wrap or replace for the slug-addressed kinds.

**Done when.**
- `RenderItem`, `GeneratedFile` and `AssetFile` are replaced by one type; `source` strings and `page_date` are gone.
- Collision error messages are unchanged (existing tests pass without edits to their expected text).
- The sitemap no longer depends on a field that exists only for it.

**Landed.** One crate-private `Output { kind: OutputKind, body: Body }` in `src/build/output.rs` (spec `specs/arch-2/`). `OutputKind` is `Page { slug, date }`, `Section(Slug)`, `Home`, `TagIndex`, `Tag(Tag)`, `Feed`, `Sitemap` or `Asset { folder, rel }`; `Body` is `Template { name, context }`, `Text(String)` or `Copy(PathBuf)`. The sketch's stored `path` was not adopted: the location is derived from the kind (`Output::path(dist)`, via `Slug::output_path` for slug-addressed kinds), so it cannot disagree with the slug. The collision label is `Display for OutputKind`, and the sitemap selects and dates entries with an exhaustive `match` on the kind, so it ignores the feed, itself and assets whatever list it is given. `pipeline::plan` builds one `Vec<Output>` (pages, sections, home, tag index, tag pages, feed, sitemap, assets last); `check_collisions` takes that slice, `output::render` turns it into `RenderedOutput`s (`Contents::Text` or `Contents::Copy`), `BuildPlan` holds that one list and `commit` writes it with a single `output::write`, which now also copies assets. `RenderItem`, `GeneratedFile`, `AssetFile`, `RenderedFile`, `render_generated`, `assets::copy`, every `source`/`label` string and `page_date` are gone. The five constructors in `render/template.rs` now return `Output`, so `build` no longer imports its core type from `render` (see ARCH-5). Rewritten unit tests kept their names and tags; `tests/plan.rs` gained five safety-net tests (full kind order, exact reachable labels, page-vs-asset collision, collision before render error, first render error in order), and unit tests pin every kind's label and path, commit-time I/O error paths and sitemap selection by kind. Two quirks found on the way were kept and are tracked as [RISK-7](#risk-7) and [RISK-8](#risk-8). Fixture output is byte-identical, no message, order or exit code changed, the public surface and `README.md` are unchanged, and no dependency was added.

### ARCH-3
**`Slug` and `Tag` newtypes** · high · M · `/spec-feature` · done

**Problem.** Slugs are plain `String`s and their logic is spread across five files: construction and validation in `content/page.rs` (`generate_slug`, `slug_url`, `tag_slug`), path building in `build/output.rs` (`get_final_path`), parent lookup in `build/index/section.rs` (`extract_parent_slug` with `rsplit_once`), top-level detection in `build/generate/home.rs` (`!slug.contains('/')`), and URL building in `render/template.rs` (five `slug_url` calls). Tags are `String`s validated once and then trusted by convention.

**Proposal.** A `Slug` type, only constructible through validation, with `parent()`, `is_top_level()`, `segments()`, `url()` and `output_path(dist)`; a `Tag` type constructed by `validate_tags`, with `url()` and `slug()`.

**Done when.**
- No module outside the `Slug`/`Tag` implementation splits, joins or formats slug strings.
- An invalid slug or tag cannot be constructed.
- Template contexts and output are byte-identical (the fixture and determinism tests pass unchanged).

**Landed.** Two crate-private types in `src/content/` (spec `specs/arch-3/`). `Slug` (`slug.rs`) wraps the `/`-joined text; its field is private and its only constructors are `from_content_path` (the file-name validation), `parent()`, `Tag::slug`, the fixed `Slug::home()` and `Slug::tag_index()`, and a validating test-only `from_test_text`. It owns `url()`, `output_path(dist)`, `parent()`, `is_top_level()` and `segments()`, orders byte-wise on the joined text (`a-c` before `a/b`, as before) and serializes as the plain string, so template contexts are unchanged. `Tag` (`tag.rs`) is built only by `Tag::parse`/`parse_list` (validation plus de-duplication, same `Frontmatter` message) and provides `slug()` and `url()`. `generate_slug`, `slug_url`, `tag_slug`, `validate_tags`, `get_final_path`, `extract_parent_slug`, the `SectionSlug` alias, `!slug.contains('/')` and the literal `""`/`"tags"` slugs are gone. `Page::new` now takes the content path and site folder and validates date, tags, then slug, so a `Page` always has a valid slug and a frontmatter error still wins over a file-name error — this closes ARCH-6's two-phase-construction bullet. One user-visible change: a file-name segment made only of dots (`...md`, `posts/...md`, `..md`) used to be accepted and could write outside its folder (`posts/...md` silently overwrote the home page's `index.html`, since the collision check compares unresolved paths); it now fails before cleaning with `<file>: invalid file name '..': a segment cannot consist only of dots` (`tests/build.rs::build_fails_on_dot_only_file_name_keeping_output`). The backslash-to-`/` quirk was kept and is tracked as [RISK-6](#risk-6). Fixture output is byte-identical, no dependency was added and the library's public surface is unchanged.

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

**Problem.** The largest source file (about 280 non-test lines) does four jobs: loading Tera, defining every view model, constructing every template `Output`, and running markdown (`render_page` calls `to_html`). Small smells: `PageTemplate::add_content` rebuilds the whole struct to set one field, and `use tera::Context;` is repeated inside each constructor.

**Proposal.** `render/context.rs` for view models, `render/template.rs` for Tera loading only, item construction moved into `build/generate/*`.

**Done when.** Each file has one job, `build` no longer imports its core type from `render`, and output is byte-identical.

**Partly landed with ARCH-2.** The "`build` no longer imports its core type from `render`" bullet is done: the output type is `build::output::Output`, and `RenderItem` is gone. The dependency now runs the other way — `render/template.rs` imports `Output` from `build` because it still constructs the template outputs — and moving that construction into `build/generate/*` removes the edge. View models, Tera loading, item construction and `add_content` stay open.

### ARCH-6
**Tidy the `Page` model** · medium · S · `/ship-feature` · open

**Problem.**
- `PageType::General` is the only variant and nothing reads `Page.kind`; under the no-dead-code rule it should go until a second page type exists.
- ~~Construction is two-phase: `Page::new` leaves `slug` empty and `generate_slug(&mut self)` fills it in later, so a `Page` without a slug can exist.~~ Done with ARCH-3: `Page::new(fm, content, kind, path, site)` builds the `Slug` in the same step.
- `compare_summaries` (the listing order) lives in `build/index/section.rs` but is used by sections, tags, the home page and the feed; it is a content rule.
- `home::recent_pages` builds `PageSummary`s only to sort them, then discards them, and `feed::build` re-checks `page.date` with an unreachable `continue`.

**Proposal.** Remove `PageType`; ~~construct `Page` from path + site + frontmatter in one step~~ (landed with ARCH-3); move the ordering next to `Page` (sorting on `&Page` directly); have `recent_pages` return pages paired with their date so the feed needs no `let … else`.

**Done when.** No unused type or field remains on `Page`, every `Page` has a valid slug from construction (already true since ARCH-3), and ordering has one home.

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

### FEAT-3
**Agent skill for using mango as a tool** · medium · M · `/spec-feature` · open

**Problem.** An AI agent asked to build or edit a mango site has only `README.md`, which is written for people, and `--help`, which lists flags but none of the rules that make a build fail. The failure modes an agent is most likely to hit are exactly the ones mango is strict about: CLI paths resolve against the current working directory, frontmatter is JSON rather than YAML, `title`/`author`/`description`/`draft` are required, dates must be `YYYY-MM-DD`, tags are lowercase ASCII with single hyphens, file names allow only `[A-Za-z0-9._-]`, `/tags/` is reserved, and five templates are required. Each one costs a failed build and a guess. A skill file (a `SKILL.md` with frontmatter `name` and `description`, in the Agent Skills format Claude Code loads) would give an agent those rules, the commands and the fix for each error up front.

**Not the same thing as `.claude/`.** Everything under `.claude/` is for agents working **on** mango's code. This skill is for agents **using** mango to build a site, usually in some other repository, so it is product surface, not dev tooling.

**Settle these in the spec, before writing it.**
- **Where it lives and how it reaches users.** A folder shipped in this repo (e.g. `skills/mango/SKILL.md`) for users to copy into their own `.claude/skills/`, a plugin, or a `mango` subcommand that prints or installs it. Printing it from the binary keeps it in step with the installed version, but it is new CLI surface.
- **What it covers.** At least: the commands and the cwd-relative path rule; the site layout; the frontmatter format and every content rule, with a valid example page; `mango.json` fields; the template names and each template's context; the output URLs; the "a failed build leaves the previous output untouched" guarantee; a table mapping each error message to its fix. Whether to split it into a short `SKILL.md` plus reference files that are loaded only when needed (e.g. the template contexts), so the part that is always loaded stays small.
- **Keeping it true.** The content rules are already written in `README.md` and `CLAUDE.md`, and a third copy drifts. Options: a test that checks the skill against the code (every CLI flag, every config field, every template name appears in it, and every example page and command in it builds), generating the skill from `README.md`, or making the skill the single source that `README.md` links to.
- **Proving it works.** Whether the evaluation is only static (the example site in the skill builds, via a `tests/build.rs` test) or also a manual trial in which an agent with only the skill builds a small site from a prompt, recorded in the spec.
- **What it does not do.** No new behavior in mango itself, such as machine-readable (`--json`) errors; if the spec finds one is needed, it becomes its own item.

**Done when.** The skill is in the repo at the path the spec picks, with the frontmatter the format requires and a `description` that names when to use it. It covers everything listed above. A test fails if a CLI flag, config field or required template is missing from it, or if its example site no longer builds. `README.md` tells site authors it exists and how to install it.

### FEAT-4
**Plain `key: value` frontmatter instead of JSON** · medium · M · `/spec-feature` · open

**Problem.** Frontmatter is a JSON object, which is awkward to write by hand: every string needs quotes, braces and commas must balance, and a trailing comma fails the build. The data is small and flat (six keys: four strings, one date, one boolean and a list of tags), so a full data language buys nothing. Raised by the maintainer on 2026-09-25 while specifying RISK-4. **Undecided:** the maintainer will decide later whether mango should make this change at all.

**Proposal.** One `key: value` pair per line between the existing `---` delimiters, split on the first colon, so a title may contain colons:
```
---
title: Rust and WASM: drawing on a canvas
author: Wren Calloway
description: Notes from a weekend experiment
date: 2026-01-24
tags: rust, wasm, canvas
draft: false
---
```
`tags` is comma-separated. That is unambiguous because a tag cannot contain a comma or a space, and a missing comma still fails loudly: `tags: rust wasm` becomes the single tag `rust wasm`, which the tag rules reject. Templates keep receiving `page.tags` as a list, so no theme changes. The parser is mango's own, with no new dependency, so errors can name the line, and RISK-4's "one error listing every key problem" rule falls out naturally.

**Settle these in the spec, before any code.**
- **Quotes.** Is `title: "Hello"` the text `Hello`, or the text with its quote marks? Whatever is chosen, it must be one rule with no exceptions.
- **YAML lookalikes.** The format looks like YAML, so authors will try YAML syntax: `- item` lists, `|` and `>` blocks, `#` comments, `[a, b]` lists, nested keys. Each one needs a defined outcome, and silently meaning something else is not acceptable.
- **Values.** Leading and trailing whitespace, empty values (`tags:` with nothing after it means no tags?), multi-line values (probably none), and how `draft` is spelled (`true`/`false` only?).
- **Required keys.** Whether `draft` (and perhaps `author` or `description`) should become optional with a default, as `draft` is in most generators.
- **Keys.** RISK-4's rules carry over: six accepted keys, compared exactly, unknown and missing keys reported together. Duplicate keys need a rule.
- **Migration.** This breaks every existing content file, including `example/site`. It needs a `CHANGELOG.md` entry with before-and-after examples, and a decision on whether mango offers any conversion help.
- **Identity.** `mango.json` stays JSON. The spec should say why having two formats is acceptable: config and content are different things.

**Done when.** Content files use the new format and JSON frontmatter is rejected with an error that points to the new format. The example site, `README.md`, `CLAUDE.md` and FEAT-3's skill (if it has landed) are updated. The RISK-4 behavior is preserved in the new parser, and RISK-9 is closed as moot.

---

## Operations and specs

### OPS-1
**Continuous integration** · medium · S · `/ship-feature` · depends on a git remote · done

No CI exists because the repository has no remote. Once one is added, run the definition-of-done gate (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`) on every push, on Linux and Windows (the code has Windows-specific paths in slug handling and `clean_contents`).

**Landed.** `.github/workflows/ci.yml` runs the gate as three separate steps (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`) on every `push` and `pull_request`, with no branch filters, in a matrix over `ubuntu-latest` and `windows-latest` with `fail-fast: false`, so both results are always reported. The toolchain comes only from `rust-toolchain.toml` (`rustup toolchain install` with no arguments, then `rustup show`); the workflow names no version. A step before checkout sets `git config --global core.autocrlf false`, so the Windows checkout keeps the committed LF line endings the byte-exact fixture tests expect. The workflow has read-only permissions (`contents: read`), uses no secrets, a 30-minute timeout, and no action but `actions/checkout@v5`. The remote itself (`https://github.com/pblanc5/mango`, private, default `master`) was added by hand on 2026-09-24, not by this item. No dependency was added. The suite had never run on Windows before this change. The first Windows run failed `build_reports_first_failure_in_pipeline_order`: the test built its expected path as `site.join("posts/one.md")`, which on Windows is `…\site\posts/one.md`, while mango correctly prints the native `…\site\posts\one.md`. Seven assertions in `tests/plan.rs` had the same flaw but had not run yet, because `cargo test` stops at the first failing test binary. Output paths are built one segment at a time and asset labels normalize to `/`, so the product was right. The fix, made in the same change at the maintainer's request, builds every expected path one segment at a time (`.join("posts").join("one.md")`); joins that only create files keep their `/`, which Windows accepts.

### OPS-2
**Release process, changelog and binaries** · medium · S · manual · depends on OPS-1 · done

mango had a version (`0.1.0` in `Cargo.toml`) but no tag, no release and no changelog, so there was nothing to download and no record of what changed.

**Landed.** Decided with the maintainer on 2026-09-25 and written down as **Releasing** in `specs/constitution.md`, next to **Landing**. Releases are GitHub Releases tagged `vX.Y.Z`, with binaries for `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc` (the platforms CI tests; macOS left out for now) and a `SHA256SUMS` file; nothing goes to crates.io. Semantic Versioning applies to the site-author surface, not the library. `CHANGELOG.md` follows Keep a Changelog, starts with an empty `[Unreleased]` section, and every pull request that changes what a site author sees adds an entry to it (definition of done, item 7). A `Release vX.Y.Z` pull request bumps the version and dates the section; once it merges, the maintainer runs `.github/workflows/release.yml` by hand from the Actions tab. That workflow is hand-written with only GitHub's own actions and `gh`, and adds no dependency. It refuses a branch other than the default, an existing tag or release, and a missing or empty changelog section. It reuses the CI gate (`ci.yml` gained `workflow_call`), builds with `--release --locked`, checks the binary's `--version`, and creates a **draft** Release; the tag exists only once the maintainer publishes it. Agents may prepare the release pull request but never run the workflow, tag or publish. Checked locally: the musl build links statically without `musl-tools`, the notes extraction picks the right section (first, middle or last) and rejects a missing one, and `actionlint` (with shellcheck) passes on both workflows. The workflow itself first runs for real with v0.1.0.

### OPS-3
**Pipeline: evidence for design claims, and a route for design fixes** · medium · S · manual · done

**Problem.** Two weaknesses in the dev pipeline (`.claude/`) showed up in the RISK-4 run on 2026-09-25.
1. **Unchecked claims in a design.** Design v1 stated two facts about serde and serde_json that were false: that `deny_unknown_fields` was unreachable at runtime, and that a `serde_json::Value` parse accepts the same input as the typed parse. Neither was checked against the library source, although it was on disk. The first was caught by a Developer test, the second by the Reviewer, and neither reached `master`. But they cost a review loop, a stop and two extra design passes.
2. **No cheap path for "the code is right and the design is wrong".** The Developer may not edit specs, and review can only send work back to develop. A correct deviation from the design therefore forced a `scope_change` stop, a design re-approval and another full develop, test and review round, and the develop pass in between could do nothing about the finding.

**Proposal.**
- **Evidence for claims.** The architect persona must back every claim about third-party library behavior with evidence (a source line in the locked crate version, or an existing test), or list it as an assumption that a safety-net task proves first. Design v3 of RISK-4 is the model.
- **A route for design fixes.** Let review route design-only findings to the design stage rather than to develop. For example, give review's findings a target, or add an `on_design_changes` handler to the workflow. The re-run design still stops for the maintainer's approval.

**Done when.** The architect persona's instructions include the evidence rule. The `spec-feature` workflow and `run-workflow.md` have a route for design-only findings that needs no `scope_change` stop and still requires the maintainer's approval of the amended design. The constitution records both changes.

**Landed.** Done by hand rather than through the pipeline, since it changes the pipeline itself; the maintainer approved the plan first. Recorded in the constitution as **Designs and design amendments**.
- **Evidence:** `pipeline-architect.md` requires a source line in the locked version, fetched documentation or an existing test for every claim about external behavior. Its output template gains **Unverified assumptions** under Risks, and each unverified claim becomes the first safety-net task. On a re-run fed by a routed review, the Architect brings the design in line with the approved code and changes no requirement.
- **Route:** the Reviewer adds `route: design` to a `changes-requested` artifact only when every blocking and should-fix finding is in the design and the code is correct; with code findings left, the code goes first. `scope_change` is reserved for wrong requirements. The Developer may depart from a wrong design inside the listed Files, marking it **design amendment needed** with evidence, and still never edits the design. `run-workflow.md` accepts the new stage key `on_design_changes` (its `goto` must be an earlier or the same stage with `gate: human`), validates `route`, and sends a routed review to that handler, whose design then stops for approval before develop, test and review run again. `spec-feature.yaml` routes review to `design` at most twice; `feature.yaml` has no design stage and is unchanged. The workflow template documents the key.

### OPS-4
**Spec critic wraps its artifact in a code fence** · low · S · manual · open

In the RISK-6 and RISK-5 runs (2026-09-25), the spec critic twice returned its artifact wrapped in a ```` ```markdown ```` code fence, so the YAML frontmatter was not on the first line and the orchestrator had to re-request it (`format_error`). One of those replies also gave a finding count in its summary that did not match its findings table. The likely cause is that `.claude/agents/pipeline-spec-critic.md` shows its output template inside a `~~~markdown` fence, which the model copies; the other personas use the same template style but did not fence their replies in these runs. As a workaround, the orchestrator added a line to the critic's prompt ("Your final message must begin with the `---` frontmatter line itself: do not wrap it in a code fence"), which departs from `run-workflow.md`'s rule that persona prompts are passed exactly.

**Done when.** The critic persona's Output rules say explicitly that the `~~~` fence only delimits the template and must not appear in the reply, and that the summary's finding counts must match the Findings table. If the same wording is added to the other personas for consistency, say so. The orchestrator's workaround line is no longer needed.

### SPEC-1
**Confirm the constitution's open proposals** · medium · S · manual · done

All **Proposed, confirm** markers are gone from `specs/constitution.md`. Decided by the maintainer, 2026-09-17:

- **Acceptance-criteria IDs are spec-scoped:** `AC-<spec-id>.<n>`, so an ID names the file that defines it (`// AC-arch-1.3` → `specs/arch-1/requirements.md`) and two specs cannot collide. This **replaces** the proposal that new specs start at group `AC-10`, which was withdrawn as already false: `AC-1` through `AC-12` are all in use today, every spec-driven run consumes another group, and any number written into the constitution is stale after the next run.
- **"Keep diffs reviewable by formatting only what you change" is dropped.** `cargo fmt` runs over the whole crate in the gate, so the rule cannot apply to Rust; nothing enforced it for Markdown; and its wording came from the gitignored `tasks.md`, which the constitution already says is not project policy. A rule that binds nothing is noise.
- **The `/spec-feature` vs `/ship-feature` lists are confirmed**, with one carve-out: removing an unimplemented surface has no behavior to specify, so it can take the direct route. FEAT-2 is the precedent; adding or changing a surface still needs `/spec-feature`.

The fourth decision — what happens to the existing numeric tags — is a job rather than a decision, and is tracked as [SPEC-2](#spec-2).

### SPEC-2
**Publish the legacy acceptance criteria into tracked specs** · medium · M · `/ship-feature` · done

**201 criteria across 48 requirements** are now in [`specs/_system/legacy-criteria/`](legacy-criteria/README.md), recovered verbatim from each run's approved plan artifact, each with the `## Verified by` table from that run's test report naming the test that checked it. Before this, every `// AC-<group>.<n>` comment in `src/` and `tests/` resolved to nothing from a clone: the definitions existed only in the gitignored `.dev-pipeline/runs/**`.

**Numbers corrected during the work.** Earlier estimates of 335 and then 249 criteria were both inflated by counting *references* — test-plan mappings (`- AC-1.1 → test_name`, written at column 0 in batch 2) and risk notes that cite an ID. Only definitions inside a `## Requirements` section count, and there are 201.

**The ambiguity is now documented rather than removed.** The four `tasks.md` batches each numbered from `AC-1`, so `AC-1` to `AC-6` are four-way ambiguous, `AC-7` three-way, `AC-8`–`AC-9` two-way, and `AC-10`–`AC-12` unique. The index states this per group, and gives the lookup that does work: find the **test's name** in the `## Verified by` tables, which identifies the run, then resolve the ID inside that file. Verified end to end on the three-way `AC-7.3`.

**Not done here:** rewriting the tags in the code, which is [SPEC-3](#spec-3).

### SPEC-3
**Rewrite the legacy AC tags into spec-scoped form** · low · M · `/ship-feature` · depends on ARCH-1 · open

**Problem.** The 181 `// AC-<group>.<n>` comments in `src/` and `tests/` now resolve (SPEC-2), but only through a lookup: the number alone is ambiguous for every group below `AC-10`. Rewriting each to the spec-scoped `AC-<spec-id>.<n>` form the constitution requires would make them self-identifying.

**Why it waited for ARCH-1.** ARCH-1 (done) split the build into `plan`/`commit`, added `lib.rs`, and moved pipeline tests in-process out of `tests/build.rs`, moving a share of the 181 tag sites — the seven `src/cli.rs` unit tests now live in `src/build/clean.rs`, and two E2E tests in `tests/plan.rs`. Their tags travelled with them unchanged, so this rewrite can now go ahead against a settled layout.

**Method.** For each tag, the test's name resolves it against the `## Verified by` tables in `specs/_system/legacy-criteria/`. Where a test has since been renamed, `git log -S'<the tag line>' -- <file>` gives the commit that introduced it and its date places it in one run. Both are mechanical; lines carrying several IDs need care.

**Done when.** Every AC tag in `src/` and `tests/` is spec-scoped or removed as withdrawn, `cargo test` still passes (comments only), and the constitution's "Legacy numeric IDs" bullet says tags are self-identifying.

---

## Risks and gaps

Source: `specs/_system/overview.md`, "Risky areas" (items marked **Inferred, confirm**), and the audit of 2026-09-14.

### RISK-1
**Symlink loops in the site folder** · medium · S · `/ship-feature` · done

`loader::traverse` recursed with `path.is_dir()`, which follows symlinks, and had no cycle guard: a symlink pointing at an ancestor folder was followed and the whole site re-walked beneath it, silently publishing up to ~40 duplicated copies of every page under bogus nested URLs with exit status 0. (The reported symptom, a stack overflow, does not happen on Linux: the kernel's 40-symlink-per-resolution limit stops the recursion first and `is_dir()` swallows the resulting `ELOOP`, so the corrupt build succeeded without a word — worse than an abort, because nothing signalled it.) Resolved by rejecting symlinked folders rather than detecting cycles: `traverse` now classifies entries with `fs::metadata` plus a non-following `is_symlink` check and fails with `content folder '<rel>' is a symlink to a directory`, so no cycle guard is needed for symlinks (a cycle among real directories, such as a bind mount or a Windows junction, would still recurse unbounded; that is out of scope, not impossible); symlinked markdown files are still read. `content/loader.rs::tests::load_rejects_symlink_loop_to_ancestor` and `tests/build.rs::build_fails_on_symlinked_content_folder_keeping_output` prove the loop case fails cleanly and leaves the previous output intact. Unresolvable links stayed silently ignored, unlike in `assets::plan`; that difference was deferred to [RISK-5](#risk-5), which has since made them a build error too.

### RISK-2
**Symlinked folders inside the assets folder** · medium · S · `/ship-feature` · done

`assets::collect` used `DirEntry::file_type()`, which does not follow symlinks, so a symlink to a folder was planned as a file and `fs::copy` failed after the output folder was cleaned. Resolved by rejecting it before cleaning: `assets::plan` now classifies entries with `fs::metadata`, fails on a symlinked folder (`asset '<rel>' is a symlink to a directory`) and on an unresolvable link, and `tests/build.rs::build_fails_on_symlinked_asset_folder_keeping_output` proves the previous output survives.

### RISK-3
**Raw HTML in content is trusted but undocumented** · low · S · `/ship-feature` · open

pulldown-cmark passes raw HTML through and templates print `page.content | safe`, so content authors can inject any HTML. That is normal for a static site generator, but `README.md` does not say so. Document it under "Known limitations" (or add an option to strip raw HTML if untrusted content is ever a use case).

### RISK-4
**Unknown frontmatter keys are silently ignored** · medium · S · `/spec-feature` · done

`MangoFrontmatter` lacks `deny_unknown_fields` (unlike `SiteConfig`), so a typo such as `"tag"` or `"dates"` is dropped without warning. Making it strict is a user-visible behavior change for existing sites, so specify it: which fields exist, the error message, and whether drafts are checked. Resolved by `specs/risk-4/`: `frontmatter::check_keys` rejects any key other than the six fields, drafts included, with one `Frontmatter` error per page listing every unknown key (sorted) and every missing required key, plus the accepted keys; key errors take precedence over wrong types, dates, tags and file names. Breaking, recorded in `CHANGELOG.md` for 0.2.0. `tests/build.rs::build_fails_on_unknown_frontmatter_keys_keeping_output` proves the previous output survives.

### RISK-5
**Unresolvable symlinks under `site/` are silently ignored** · low · S · `/spec-feature` · done

An entry under `site/` whose target cannot be resolved — a dangling symlink, or a chain that loops (`a -> b -> a`) — is skipped without a word by `loader::traverse`, which is the behavior inherited from `path.is_dir()`/`path.is_file()` swallowing I/O errors. `assets::plan` already treats both as `io_at` build errors (RISK-2), so the two modules deliberately differ. Making the loader strict is a user-visible behavior change for existing sites and has no bearing on the runaway traversal, so it was deferred from RISK-1. Decide whether a broken or looping link under `site/` should fail the build (and whether a warning is enough), then align the two modules or record why they differ. Resolved by `specs/risk-5/`, which chose to fail the build, with no warning: `loader::traverse` now fails on any entry whose `fs::metadata` fails (a dangling link, a looping chain, an entry removed after `read_dir`) with the same `io_at` error as `assets::plan`, naming the link, whatever its name, hidden names included. The two modules now follow one rule. Breaking, recorded in `CHANGELOG.md` for 0.2.0. `tests/build.rs::build_fails_on_dangling_symlink_in_site_keeping_output` proves the previous output survives.

### RISK-6
**Backslashes in file names become folder separators** · low · S · `/spec-feature` · done

`Slug::from_content_path` rewrote every `\` in the site-relative path to `/`, which is right on Windows (where `\` is a separator) but on Unix turned a single file literally named `a\b.md` into the slug `a/b`: it was published at `/a/b/` and created a section `a` that had no folder, even though the file-name rule does not allow `\` in a name. ARCH-3 kept this unchanged and pinned it with a test that has since been removed. Resolved by `specs/risk-6/`, which chose rejection over documenting the behavior: `from_content_path` now splits only on the platform's own separators (`std::path::is_separator`), so on Linux and macOS a `\` in a file or folder name stays in its segment and gets the existing invalid-file-name error, naming the name as it is on disk (`'a\b'`), drafts included and before any collision check. Windows is unchanged. Breaking, recorded in `CHANGELOG.md` for 0.2.0. `tests/build.rs::build_fails_on_backslash_in_file_name_keeping_output` proves the previous output survives.

### RISK-7
**Asset-copy errors name the source, not the destination** · low · S · `/ship-feature` · open

When copying an asset fails during `commit`, `output::write` reports `IoPath` with the asset's **source** path, even when the destination is the problem (for example, a folder already sits where the file should go). The message then points at the input file, which is not what failed. `copy_failure_is_an_error_naming_the_path` does not catch this, because it only checks for `style.css`, which appears in both paths. `copy_failure_names_the_source_path` (arch-2, AC-5.4) pins the current behavior. Name the destination instead, or both paths, and update that test.

### RISK-8
**Content page order depends on the filesystem** · low · S · `/spec-feature` · open

`loader::load` returns pages in `read_dir` order, which is not sorted and differs between filesystems. Content pages come first in the plan, so the order of `BuildPlan::outputs()`, the order files are written in, and which page's render error is reported when several pages fail all vary by filesystem. The output bytes do not: every listing is sorted, and a page-vs-page collision prints the same labels either way. Recorded as baseline AC-2.4 in `specs/arch-2/requirements.md`. Proposal: sort pages by slug in the loader, so enumeration and error precedence are deterministic too. Because this changes which error a user sees first, it is a `/spec-feature` change.

### RISK-9
**A JSON array is accepted as frontmatter** · low · S · `/spec-feature` · open

serde's derived deserializer for `MangoFrontmatter` also accepts a JSON array whose values are in field order, so a page whose frontmatter is `["t", "a", "d", null, null, false]` builds today, with no keys at all. RISK-4 left this alone on purpose: an array has no keys, so its key check doesn't apply, and the spec keeps non-object errors as they were (AC-5.1 in `specs/risk-4/requirements.md`; Risks in `specs/risk-4/design.md`). Nobody writes frontmatter this way on purpose, but it is a second, undocumented format. Proposal: reject any frontmatter that is not a JSON object, with an error naming the file. Because it rejects input that builds today, it is a `/spec-feature` change. [FEAT-4](#feat-4) would make this item moot.

### RISK-10
**Hidden files under `site/` are published** · medium · S · `/spec-feature` · open

Every markdown file under `site/` is a page, including hidden ones: a file named `site/.notes.md` is published at `/.notes/` (confirmed with the binary on 2026-09-25 while specifying RISK-5, and pinned by `load_publishes_hidden_markdown_file`). So is anything in a hidden folder: `site/.drafts/secret.md` is published at `/.drafts/secret/`, and the folder gets a section index at `/.drafts/` that lists it (also confirmed with the binary). Under `meta/assets/`, a hidden file such as `.DS_Store` is copied into `dist/assets/`. Most static site generators skip names that start with `.`, and authors may keep scratch notes there expecting them to stay private. The repository being public makes this a real exposure for anyone copying the layout.

Skipping hidden entries would also fix a consequence of RISK-5: Emacs marks a file being edited with a lock file `.#<name>`, a dangling symlink by design, which now fails the build (RISK-5 AC-2.3 chose no exemption). A hidden-entry rule applied before resolution would exempt it.

**Settle these in the spec.** Whether to skip hidden files, hidden folders, or both; whether names starting with `_` are also skipped, as some generators do; whether the rule also applies under `meta/assets/` or only under `site/`; and whether a skipped entry is checked at all (a hidden broken symlink would then no longer fail the build). It changes which pages are published, so it is breaking, like RISK-4, RISK-5 and RISK-6.

**Done when.** The rule is implemented and tested, `load_publishes_hidden_markdown_file` is replaced by a test of the new behavior, RISK-5's Emacs lock-file note is revisited, and `README.md` and `CHANGELOG.md` say which names are skipped.

### TEST-1
**No test for CRLF line endings** · low · S · `/ship-feature` · done

Frontmatter parsing handled `\r\n` (verified by hand during the audit) but nothing tested it. Pinned with characterization tests only — no production change, and the audit's claim held on every point. Six unit tests in `content/frontmatter.rs` cover a wholly-CRLF document (`parses_crlf_frontmatter_and_normalizes_body`), CRLF/LF equivalence (`crlf_and_lf_documents_parse_identically`), a BOM combined with CRLF (`byte_order_mark_with_crlf_is_ignored`), endings mixed line by line (`parses_mixed_lf_and_crlf_line_endings`) and an unterminated CRLF block (`unterminated_crlf_block_is_an_error`). The behavior pinned: `str::lines` strips the `\r`, so the delimiters match and the JSON block is re-joined with `\n`, and the body is normalized to LF with its final newline dropped. `crlf_body_is_returned_verbatim_when_no_frontmatter` records the one asymmetry — the "no frontmatter" early return hands the content back verbatim, CRLF and all. It is unreachable from the CLI (`loader::load` turns `None` into the missing-frontmatter error), so it was pinned rather than fixed; [ARCH-7](#arch-7) rewrites that return and will retire the test.

End to end, `tests/build.rs::build_accepts_crlf_line_endings` builds a CRLF temp site (exact output manifest, `<time datetime="2026-01-24">` and `/tags/crlf/` proving no `\r` leaked into a JSON date or tag, no `\r` in the rendered content region, `feed.xml` or `sitemap.xml`), and `crlf_and_lf_sites_build_identical_output` compares that site byte for byte with its LF twin, feed and sitemap included. **What each one can catch**, measured by mutating the body's `join("\n")` to `join("\r\n")` in an isolated copy: five unit tests fail, and `build_accepts_crlf_line_endings` fails too. The E2E test detects it through one deliberate construct on the test page — a raw *inline* HTML tag split across two lines. pulldown-cmark normalizes line endings in almost everything it parses — paragraphs, fenced and indented code, HTML blocks, link titles and footnotes all came back identical under probes of some sixty constructs, run independently three times — but it passes raw inline HTML through verbatim. That and a multi-line code span, asserted next to it as a second and independent signal (it renders with two spaces instead of one), are the exceptions those probes found; the evidence is broad, not exhaustive, so treat the list as open. Should a future pulldown-cmark normalize inline HTML as well, the first assertion degrades to vacuous rather than failing, so the unit tests remain the primary guard. `crlf_and_lf_sites_build_identical_output` stays green under the mutation and always will: it is a symmetry property, and a regression in the shared join path perturbs the CRLF and LF builds alike — it guards against the two inputs being handled *differently*, not against normalization itself. Nothing was added to `example/site`: line endings in a committed file are not durable (`core.autocrlf`, `.gitattributes`), so every CRLF input is constructed in test code.

### DOC-1
**Stale line references in the system overview** · low · S · `/ship-feature` · done

`specs/_system/overview.md` cites line numbers (e.g. in `src/cli.rs` and `example/meta/templates/page.html`) that moved in later commits. Refresh them, or cite functions instead of lines so they stay valid; ARCH-1 to ARCH-5 will move most of them again, so this is best done after those land. The "Risky areas" rows for `loader::traverse` and `assets::collect` are also stale now that RISK-1 and RISK-2 are done; refresh them in the same pass. So is the `frontmatter.rs` risk row: it cites `MangoFrontmatter` at `:8-16` and the CRLF handling at `:25` and `:35-36` (now `:23-31`, `:40` and `:50-51`), and still says "CRLF untested", although TEST-1 added those tests (flagged by the RISK-4 review). The repository is now public, so stale references are visible to every reader.

Resolved by rewriting the overview rather than refreshing its numbers: it now cites functions, types, test names and README or CLAUDE.md section headings instead of line numbers, and the per-file line and test counts are gone, so it no longer drifts every time code moves. In the same pass it was brought up to date with ARCH-1/2/3 (the plan/commit seam, `lib.rs`, `build/pipeline.rs`, `build/clean.rs`, the single output model, `Slug` and `Tag`), RISK-1/2/4, TEST-1 and the FEAT-2 drop (`publish` is not a command). Its risk rows now record what is still open: RISK-3, RISK-5, RISK-7, RISK-8 and RISK-9.

### DOC-2
**Line-number citations in the constitution** · low · S · `/ship-feature` · open

DOC-1 replaced every line-number citation in `specs/_system/overview.md` with symbol, test or section names so it stays valid when code moves. `specs/constitution.md` still has 43 such citations above its Changelog (for example `CLAUDE.md:75`, `README.md:110`, `Cargo.toml:4`), most of which have drifted as CLAUDE.md and README grew. Its Commands section also says the toolchain is Rust 1.92.0 "with rustfmt and clippy", while `rust-toolchain.toml` also lists `rust-analyzer`.

**Done when.** Every citation above the constitution's Changelog names a section, symbol or file instead of a line; the toolchain line matches `rust-toolchain.toml`; historical Changelog rows stay verbatim; and a new Changelog row records the change.
