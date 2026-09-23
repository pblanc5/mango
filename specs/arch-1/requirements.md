<!-- Published by dev-pipeline run 20260922-224052-arch-1-split-plan-commit, stage spec v3, approved 2026-09-22T23:23:34-05:00. -->

# Split the build into plan and commit; add a library crate: Requirements

Spec ID: `arch-1`

## Summary
mango's most important promise is that a failed build never touches the previous output. Today that promise holds only because the statements in one ~80-line function happen to be in the right order: a new line in the wrong place breaks it silently. This spec asks for the build to be split into a **planning phase**, which loads, checks and renders everything and writes nothing, and a **commit phase**, which is the only thing that may empty and write the output folder and which can only be fed by a successful plan. Because there is no library crate, the pipeline can currently be tested only by spawning the binary; this spec also asks for a library alongside the binary so collision, ordering and draft behaviour can be tested in-process. Nothing a site author sees (flags, defaults, help, messages, exit codes, output bytes) changes.

## Context
- **Who needs it.** Maintainers (the guarantee becomes structural rather than positional; tests get cheaper), and two queued items: FEAT-1 (the dev server wants to re-plan on every change without rewriting `dist/`) and ARCH-2 (one output model) build on this seam. SPEC-3 deliberately waits for it because this work moves many test sites.
- **Source.** `specs/_system/backlog.md#arch-1` (problem, proposal, done-when). The proposal there names `plan`/`commit`/`BuildPlan`/`lib.rs`; those are the backlog's sketch, and this document treats them as capabilities, leaving names and shapes to the design.
- **Existing guarantees this must preserve** are recorded in `specs/_system/legacy-criteria/batch-2-consistent-output.md` (REQ-3, REQ-4, REQ-9: collisions fail before anything is written, cleaning with a safety check, render everything in memory before cleaning) and `batch-1-trustworthy-build.md` (REQ-2: errors to stderr, exit 1). The baseline section below restates the parts this change touches.
- **Rules that bind this change** (`specs/constitution.md`): no new dependencies; no dead code and no `#[allow(dead_code)]`; every behaviour change has a test that fails without it (one approved exception, AC-4.5); bad input fails before the output folder is cleaned, proven with a `snapshot` test; determinism is enforced by `fixture_build_is_deterministic`; `ensure_safe_to_clean`'s refusals must keep holding; docs change with the code.
- **Related items.** ARCH-7 removes the `"."`-joined site path (see AC-3.3); DOC-1 refreshes `specs/_system/overview.md`, whose one sentence stating there is no library crate is corrected by this change (AC-9.4).
- **Decisions taken at approval (2026-09-22)** that shape the criteria below:
  1. A plan is bound to the output folder it was planned against; committing it elsewhere is not possible (AC-4.5, AC-4.6, AC-5.4 are firm, not provisional).
  2. The `./<site>` quirk is preserved exactly; ARCH-7 removes it with its own test (AC-3.3, AC-8.2).
  3. The library is an internal seam: it may change freely, is not documented in `README.md`, and carries no semver promise; `CLAUDE.md` says so (AC-9.3).
  4. The design lists every E2E test it moves in-process or deletes, with the test that keeps each behaviour covered (AC-7.10).
  5. The one sentence in `specs/_system/overview.md` that says there is no library crate is corrected in this change; line-number refreshes stay with DOC-1 (AC-9.4).
  6. The safety check's canonicalize-skip quirk is preserved unchanged; tightening it is out of scope (AC-1.5, AC-4.7).
  7. Code comments for this spec's criteria read `// AC-arch-1.<req>.<m>`, e.g. `// AC-arch-1.4.2` for AC-4.2 (AC-7.4).
  8. AC-4.5 is verified by review of the public surface, as an approved exception to the "every behaviour change has a test" rule.

## Current behavior

### REQ-1 [baseline] `mango build` runs every fallible step before touching the output folder
Evidence: `src/cli.rs:89-171` (`build`), `src/cli.rs:197-266` (`ensure_safe_to_clean`, `clean_contents`); tests `failed_rebuild_keeps_previous_output`, `page_render_error_keeps_previous_output`, `section_render_error_keeps_previous_output`, `config_error_reported_before_templates_error`, `build_fails_on_page_and_section_output_collision`, `build_fails_when_assets_path_has_no_name`, `build_refuses_output_containing_config` (all `tests/build.rs`); `CLAUDE.md` "CLI surface" bullet for `build`.
Status: inferred, confirm at approval
- AC-1.1 [baseline] WHEN `mango build` runs THE build command SHALL perform these steps in this order: load pages; load the site config; load the templates; resolve the assets destination; list the asset files; build the page, section-index, home, tag-index and tag-page items; build the feed and sitemap in memory (only when `base_url` is set); check for output collisions; render every item to memory; run the safety check; empty the output folder; write every rendered and generated file; copy the listed assets.
- AC-1.2 [baseline] IF any step up to and including the safety check fails THEN THE build command SHALL exit with status 1, print the error to stderr and nothing to stdout, and leave the output folder's contents exactly as they were, stray files included.
- AC-1.3 [baseline] WHEN more than one input is bad THE build command SHALL report only the first failure in the order of AC-1.1 (a malformed config is reported and a missing templates folder is not mentioned).
- AC-1.4 [baseline] IF an I/O failure happens after the output folder has been emptied THEN THE build command SHALL fail with an error naming the path, and partial output may remain (a documented limitation, `CLAUDE.md`).
- AC-1.5 [baseline] THE safety check SHALL refuse, comparing canonicalized paths, an output folder that is or contains the current directory, or that is or contains the site folder, the templates folder, the assets folder or the config file, with an error naming the output path and the offending path; a protected path that cannot be canonicalized, for any reason and not only "not found", is skipped; an output folder that does not exist is safe. (The skip-for-any-reason quirk is preserved unchanged by this change; decision 6.)
- AC-1.6 [baseline] WHEN the output folder is emptied THE build command SHALL remove its contents but keep the folder, removing symlinks without following them; IF the output path exists but is not a directory THEN THE build command SHALL fail with an error naming it.
- AC-1.7 [baseline] THE build command SHALL hold every rendered page, section, home, tag and generated file in memory until the write step, so memory use grows with site size and no output is written incrementally.

### REQ-2 [baseline] The crate is a single binary and the pipeline is testable only by spawning it
Evidence: `src/main.rs:1-18` (every module is a private `mod` of the binary), `Cargo.toml` (no `[lib]` section), `specs/_system/overview.md:13` ("There is no library crate"), `tests/build.rs:18-24` (`run_mango` spawns `CARGO_BIN_EXE_mango`), `tests/build.rs` (about 2,250 lines, 66 `#[test]` functions at the baseline commit).
Status: inferred, confirm at approval
- AC-2.1 [baseline] THE crate SHALL build one binary target and no library target; no code outside `src/` can call any pipeline step.
- AC-2.2 [baseline] THE E2E test suite SHALL exercise the whole pipeline only by spawning the compiled binary and asserting on exit status, stderr, stdout and files on disk.
- AC-2.3 [baseline] THE unit tests SHALL cover individual steps in isolation (collision checking over hand-built items in `build::output`, listing order in `build::index::section`, draft filtering in `content::loader`, safety refusals in `cli`), and no test SHALL run load-through-render as one in-process call.

### REQ-3 [baseline] CLI surface that the split must leave intact
Evidence: `src/cli.rs:20-87` (options, subcommands, dispatch), `src/cli.rs:173-188` (`clean`), `src/main.rs:10-18` (stderr + exit code), `README.md:30-38`; tests `help_shows_flag_descriptions`, `run_command_is_not_implemented_error`, `run_command_takes_no_options`, `publish_is_not_a_command`, `clean_succeeds_for_missing_dir`, `clean_refuses_cwd`, `clean_fails_naming_path_when_target_is_a_file`, `build_without_config_uses_defaults`, `build_fails_when_explicit_config_missing`.
Status: inferred, confirm at approval
- AC-3.1 [baseline] THE `mango build` command SHALL accept `--site` (default `site`), `--templates` (default `meta/templates`), `--assets` (default `meta/assets`), `-o`/`--output` (default `dist`) and `--config` (optional), every path relative to the current directory, with the help text pinned by `help_shows_flag_descriptions`.
- AC-3.2 [baseline] WHEN `--config` is not given THE `mango build` command SHALL read `mango.json` from the current directory if it exists and use defaults otherwise; IF `--config` names a file that does not exist THEN THE `mango build` command SHALL fail with an error naming that path.
- AC-3.3 [baseline] THE `mango build` command SHALL join the `--site` value onto `.` before use, so an error about the site folder itself names it as `./<site>`, while the templates, assets, output and config paths are used exactly as given. (Preserved exactly by this change; ARCH-7 removes the join with its own test; decision 2.)
- AC-3.4 [baseline] THE `mango clean` command SHALL accept `-d`/`--dist` (default `dist`), succeed silently when the folder is missing, remove the folder entirely when present, and refuse (exit 1, deleting nothing) a target that is or contains the current directory or the default `site`, `meta/templates` or `meta/assets` under the current directory; it SHALL NOT protect a config file (documented limitation).
- AC-3.5 [baseline] THE `mango run` command SHALL take no options and fail with `the 'run' command is not implemented yet`, exit status 1; `mango publish` SHALL be rejected by the argument parser as an unrecognized subcommand.
- AC-3.6 [baseline] IF any command fails THEN THE binary SHALL print the error's display text to stderr, nothing to stdout, and exit with status 1; WHEN a command succeeds THE binary SHALL exit with status 0.

## User stories
- As a maintainer, I want the "nothing touches the output folder until everything has succeeded" rule to be enforced by the structure of the build rather than by statement order, so that a future change cannot break it silently.
- As a maintainer, I want to run the whole pipeline in-process from a test, so that collision, ordering and draft behaviour can be checked cheaply and precisely instead of only through the binary's exit code and stderr.
- As the future dev-server author (FEAT-1), I want to compute a complete, fully rendered build in memory without writing anything, so that I can serve it and re-plan on every change without emptying `dist/`.
- As a site author, I want `mango build` and `mango clean` to behave exactly as before, so that this refactor is invisible to me.

## Requirements

### REQ-4 The build has a write-free planning phase and a separate commit phase
The two halves of today's `build` become two operations. Planning does everything that can fail on bad input and produces a complete plan; committing is the only thing that changes the output folder. This turns AC-1.2 from a property of statement order into a property of the seam. A plan is bound to the output folder it was planned against (decision 1).
- AC-4.1 THE build library SHALL expose the build as two phases: a planning phase (load pages, load config, load templates, resolve the assets destination, list assets, build every item, build the feed and sitemap, check collisions, render every item) and a commit phase (safety check, empty the output folder, write, copy assets).
- AC-4.2 WHEN the planning phase runs THE planning phase SHALL create, modify or delete no file or folder, whether it succeeds or fails; a test SHALL prove this by snapshotting the site, templates, assets and output folders before and after planning.
- AC-4.3 WHEN the planning phase succeeds THE planning phase SHALL return a build plan that already contains every output fully rendered (every page, section index, home page, tag index and tag page, and the feed and sitemap when `base_url` is set) and every asset copy to perform, so that no content, config, template, file-name, tag, date, collision or render error can occur after planning.
- AC-4.4 IF any input is bad THEN THE planning phase SHALL fail with the same error type and the same display text that `mango build` reports today for that input, and WHEN several inputs are bad THE planning phase SHALL report the first failure in the order of AC-1.1 (preserving AC-1.3).
- AC-4.5 THE commit phase SHALL accept only a build plan, and THE build library SHALL offer code outside it no way to obtain or alter a build plan other than a successful planning phase (no public constructor, no public mutable access to its outputs, no separate destination argument at commit time). **Verification: by review of the library's public surface, not by an automated test.** This is an approved exception (maintainer, 2026-09-22) to the constitution's "every behaviour change has a test that fails without it" rule, granted because a compile-fail check would need a new dependency and the property is structural rather than behavioural. The design SHALL enumerate the public items through which a plan can be reached so the reviewer can check them one by one.
- AC-4.6 WHEN a build plan is committed THE commit phase SHALL, in this order: run the safety check; empty the output folder the plan was planned against (keeping the folder itself, per AC-1.6); write every rendered and generated output; copy every planned asset.
- AC-4.7 IF the safety check refuses the output folder THEN THE commit phase SHALL fail with today's error text (AC-1.5, including its canonicalize-skip behaviour unchanged) and SHALL neither delete nor write anything.
- AC-4.8 THE commit phase SHALL protect the same paths as today: the site, templates and assets folders and the config file used when the plan was made, plus the current directory and its parents at commit time.
- AC-4.9 THE commit phase SHALL be the only operation in the build library that empties or writes into the output folder; no other public operation SHALL delete or write output.
- AC-4.10 WHEN `mango build` runs THE build command SHALL plan and then commit, and IF planning fails THEN THE build command SHALL not commit and SHALL leave the output folder untouched (AC-1.2 continues to hold end to end, proven by the existing `snapshot`-based E2E tests).

### REQ-5 A build plan can be inspected without being committed
In-process tests (REQ-7) and the future dev server need to see what a build would write without writing it.
- AC-5.1 THE build plan SHALL let a caller enumerate every output it would write: for rendered and generated outputs, the path relative to the output folder and the exact bytes; for asset copies, the destination path relative to the output folder and the source path.
- AC-5.2 WHEN the same inputs are planned twice in one process THE planning phase SHALL produce plans whose enumerated outputs are identical in paths, contents and order.
- AC-5.3 WHEN a plan is committed THE set of files present in the output folder afterwards SHALL be exactly the plan's enumerated outputs, no more and no fewer.
- AC-5.4 THE build plan SHALL be bound to the one output folder it was planned against and SHALL name it, so that collision errors keep naming the full output path exactly as today (`output path '<dist>/…' would be written by both …`) and a plan cannot be committed into a different folder (decision 1).

### REQ-6 A library crate exists, and the CLI layer only parses and dispatches
- AC-6.1 THE crate SHALL provide a library target alongside the binary, and THE binary SHALL do nothing but parse arguments and call the library.
- AC-6.2 THE CLI layer SHALL contain only argument definitions, defaults, help text and the dispatch of each subcommand; it SHALL contain no pipeline step, no safety check and no cleaning logic.
- AC-6.3 THE `mango clean` command SHALL keep the behaviour in AC-3.4 unchanged, with its logic reachable through the library rather than living in the CLI layer.
- AC-6.4 THE library SHALL expose only what the binary and the tests use; nothing SHALL be made public "for later", and no `#[allow(dead_code)]` SHALL be added. (Making an item public in a library silences the dead-code lint, so this needs a deliberate check rather than reliance on the compiler.)
- AC-6.5 THE change SHALL add no dependency to `Cargo.toml`, dev-dependencies included.
- AC-6.6 THE change SHALL keep the gate green: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` with no new warnings.

### REQ-7 The pipeline is tested in-process; the E2E suite keeps the binary's contract
Each new in-process test carries a tag in the spec-scoped form `// AC-arch-1.<req>.<m>` (decision 7); AC-7.1, AC-7.6 to AC-7.9 and AC-7.11 are one observable behaviour each so that every tag is unambiguous.
- AC-7.1 THE test suite SHALL include an in-process test, not spawning the binary, that a top-level page colliding with its section index fails planning with the message naming the output path and both sources.
- AC-7.2 THE test suite SHALL include an in-process test that planning a bad site leaves every input folder and the output folder unchanged (the proof for AC-4.2).
- AC-7.3 THE E2E test suite SHALL keep covering, by spawning the binary: exit status 0 on success and 1 on failure; stderr naming the file or path and the bad value; empty stdout on failure; the fixture output manifest, determinism and internal-link tests; and previous-output-intact snapshots for each failure class that exists today (bad page, bad config, missing templates, render error, collision, assets, safety refusal).
- AC-7.4 WHEN a test is moved from the E2E suite in-process or deleted THE behaviour it verified SHALL remain covered by at least one test, and a moved test SHALL carry its `// AC-…` tag comment with it; no legacy criterion listed in `specs/_system/legacy-criteria/` SHALL lose its last verifying test; new tests written for this spec SHALL be tagged `// AC-arch-1.<req>.<m>`.
- AC-7.5 WHERE a behaviour is covered both in-process and end to end THE E2E test MAY be reduced to the exit-status, stderr and on-disk assertions, but SHALL NOT be removed if it is the only test of a snapshot (previous-output-intact) guarantee.
- AC-7.6 THE test suite SHALL include an in-process test that a file-vs-folder conflict (one output is a file at a path another output needs as a directory) fails planning with the message naming both outputs.
- AC-7.7 THE test suite SHALL include an in-process test that, within a section, the plan lists pages newest date first, undated last, ties by title then slug.
- AC-7.8 THE test suite SHALL include an in-process test that the plan's home page lists at most `recent_count` dated pages and drops undated ones.
- AC-7.9 THE test suite SHALL include an in-process test that a draft page appears in none of the plan's outputs or listings (section, home, tag).
- AC-7.10 THE design SHALL list every E2E test it moves in-process or deletes, naming for each the test that keeps its behaviour covered, so the reviewer can check AC-7.4 one test at a time (decision 4).
- AC-7.11 THE test suite SHALL include an in-process test that a draft with malformed frontmatter, an invalid date or an invalid tag still fails planning naming the file.

### REQ-8 Nothing changes for site authors
- AC-8.1 THE `mango build` command SHALL produce byte-identical output to the build before this change for the fixture site: `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `fixture_internal_links_resolve` and every other `fixture_*` test pass without edits to their expectations.
- AC-8.2 THE `mango build`, `mango clean` and `mango run` commands SHALL keep their flags, defaults, help text, error texts (the `./<site>` form of AC-3.3 included) and exit statuses: every existing E2E test passes with no change to an expected string, path or status (edits limited to moving a test in-process, or to helper code).
- AC-8.3 THE `mango build` command SHALL keep the failure precedence of AC-1.3 (`config_error_reported_before_templates_error` passes unchanged).
- AC-8.4 THE README SHALL need no change, since no site-author-visible behaviour, flag, output or limitation changes and the library is not a documented interface; IF the design does change something visible THEN THE README SHALL be updated in the same change.

### REQ-9 Developer documentation and backlog follow the code
- AC-9.1 THE `CLAUDE.md` `build` bullet under "CLI surface" and the module map SHALL describe the two phases, the library crate, where the clean and safety logic now lives, and SHALL restate the "nothing is deleted or written until loading, planning and rendering have succeeded" guarantee as a property of the seam rather than of statement order.
- AC-9.2 THE backlog SHALL mark ARCH-1 `done` in the squash commit that lands it, with `(ARCH-1)` in the subject, and the dependency notes on FEAT-1, ARCH-2 and SPEC-3 SHALL still read correctly afterwards.
- AC-9.3 THE `CLAUDE.md` SHALL state that the library is an internal seam: it may change freely, is not documented in `README.md`, and carries no semver promise (decision 3).
- AC-9.4 THE `specs/_system/overview.md` sentence "There is no library crate: `src/main.rs` is the only target, and there is no `lib.rs`." (line 13 at the baseline commit) SHALL be corrected in this change to describe the library and binary targets; no other line reference in that file SHALL be refreshed here (that is DOC-1; decision 5).

## Out of scope
- Replacing the three output shapes with one (ARCH-2), `Slug`/`Tag` newtypes (ARCH-3), structured error variants (ARCH-4), splitting `render/template.rs` (ARCH-5), and ARCH-6/ARCH-7 cleanups, including removing the `"."`-joined site path (decision 2).
- Tightening the safety check's canonicalize-skip (AC-1.5) to skip only "not found" (decision 6).
- The dev server (FEAT-1), parallel rendering, incremental builds, and any change to what is written or when.
- Committing a plan into a folder other than the one it was planned against (decision 1).
- Rewriting the legacy `// AC-<group>.<n>` tags into spec-scoped form (SPEC-3); this change only carries existing tags along.
- Refreshing line references in `specs/_system/overview.md` (DOC-1), beyond the one sentence in AC-9.4.
- Any change to messages, exit codes, flags, output URLs, listing order or the fixture site.
- Making the library API a documented, stable interface for third parties (decision 3).
- An automated test for the post-clean I/O failure allowance (AC-1.4); it is inherited unchanged and remains a documented limitation (critic finding 5, skipped by the maintainer).

## Open questions
None. The seven questions from attempt 1 were all resolved on 2026-09-22 by accepting their recommendations; the outcomes are recorded as decisions 1 to 7 under Context and folded into AC-1.5, AC-3.3, AC-4.5, AC-4.6, AC-4.7, AC-5.4, AC-7.4, AC-7.10, AC-8.2, AC-9.3 and AC-9.4.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-22 | 20260922-224052-arch-1-split-plan-commit | New spec from backlog ARCH-1: baseline of today's build order, crate layout and CLI surface; requirements for a write-free planning phase, a sole-writer commit phase fed only by a successful plan, an inspectable plan, a library crate with a parse-and-dispatch CLI layer, in-process pipeline tests, and an unchanged site-author contract. |
| 2026-09-22 | 20260922-224052-arch-1-split-plan-commit | Attempt 2: corrected the E2E test count (66); split AC-7.1 into one behaviour per criterion (AC-7.1, AC-7.6 to AC-7.9); AC-4.5 verified by review as an approved exception; resolved all seven open questions per their recommendations and recorded them as decisions (plan bound to its output folder, `./<site>` kept, internal library, design lists moved tests, overview sentence fixed, canonicalize-skip kept, `// AC-arch-1.<req>.<m>` tag form); added AC-7.10, AC-9.3, AC-9.4. |
| 2026-09-22 | 20260922-224052-arch-1-split-plan-commit | Attempt 3: split AC-7.9 so it holds one behaviour (draft pages excluded from every plan output and listing); the draft-validation half (malformed frontmatter, invalid date or invalid tag on a draft still fails planning naming the file) is now AC-7.11; REQ-7 preamble lists AC-7.11 among the one-behaviour-each criteria. No other change. |
