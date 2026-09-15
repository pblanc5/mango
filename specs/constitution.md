<!-- Published by dev-pipeline run 20260914-223413-bootstrap-specs-for-this-project, stage constitution v2, approved 2026-09-14T22:59:58-05:00. -->

# Project constitution

Rules and conventions every dev-pipeline persona follows in this project. Edit freely; the pipeline reads this file on every run.

## Specs location
Specs live in `specs/`: one folder per feature (`specs/<spec-id>/requirements.md` and `design.md`), plus `specs/_system/overview.md`.

## Commands
No CI config was found (no `.github/`, no other CI files). Every command below comes from the docs.

| Purpose | Command | Source |
|---|---|---|
| Gate (must pass before done) | `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` | `CLAUDE.md:75`, `README.md:110` |
| Build | `cargo build` | `CLAUDE.md:34` |
| Test (unit + `tests/build.rs` E2E) | `cargo test` | `CLAUDE.md:35` |
| Lint | `cargo clippy --all-targets -- -D warnings` | `CLAUDE.md:75` (plain `cargo clippy` at `CLAUDE.md:36`; `[lints.clippy] all = "warn"` in `Cargo.toml:10-11`) |
| Format (write) | `cargo fmt` | `CLAUDE.md:37` |
| Format (check) | `cargo fmt --check` | `CLAUDE.md:75`, `README.md:110` |
| Run against the fixture site (from `example/`) | `cd example && cargo run -- build` | `CLAUDE.md:40` |
| Run against the fixture site (from repo root) | `cargo run -- build --site example/site --templates example/meta/templates --assets example/meta/assets -o example/dist` | `CLAUDE.md:42` |
| Install | `cargo install --path .` | `README.md:10` |

Toolchain: Rust `1.92.0` with rustfmt and clippy (`rust-toolchain.toml:2-3`), edition 2024 (`Cargo.toml:4`).

**Path gotcha:** CLI paths are relative to the current directory. The defaults are `site/`, `meta/templates/`, `meta/assets/`, `dist/` and `mango.json`. None of these exist at the repo root; the sample site is under `example/` (`CLAUDE.md:45`).

## Conventions
- **Errors:** return `MangoError` via `?`. Where a category exists, add a variant to `src/error.rs` instead of using ad-hoc strings (`CLAUDE.md:66`). Seen in `src/config.rs` and `src/content/loader.rs`.
- **Path-specific I/O errors:** go through `MangoError::io_at(path, e)` (`CLAUDE.md:66`). Seen in `src/cli.rs`, `src/build/generate/assets.rs` and `src/build/output.rs`.
- **CLI:** clap derive style (`CLAUDE.md:67`). Seen in `src/cli.rs:20-84`.
- **Unit tests:** go in `#[cfg(test)] mod tests` next to the code. Filesystem fixtures are created per test under `target/unit-fixtures/<module>/...` (`CLAUDE.md:68`). Seen in `src/config.rs`, `src/content/loader.rs` and `src/build/output.rs`.
- **E2E tests:** live in `tests/build.rs` and run the compiled binary. They build either the fixture (once, via `fixture_dist()` into `target/integration-dist`) or per-test temp sites under `CARGO_TARGET_TMPDIR`. They check exit status, stderr and output files, and reuse the existing helpers (`run_mango`, `temp_dir`, `build_temp_site`, `build_with`, `snapshot`, `assert_success`, `assert_failure`, `assert_previous_output_intact`, `built_site_with_private_templates`) (`CLAUDE.md:68`; `tests/build.rs`).
- **Fixtures:** `example/site`, `example/meta` and `example/mango.json` are committed and cover every success-path feature. `build_generates_site_from_fixture` checks the exact output manifest, and the `fixture_*` tests check the rest. Error-case files never go in the fixture; they are built in test code. `example/dist` is generated and gitignored (`CLAUDE.md:69`, `.gitignore:2`).
- **Build pipeline order:** load, plan assets, build render items and generated files, check collisions, render everything in memory, run the safety check, and only then clean and write. New outputs must go through `output::check_collisions` and be rendered before anything is cleaned (`CLAUDE.md:49-51`; `build()` in `src/cli.rs`).
- **Strict inputs:** dates, tags, file names and `base_url` are validated strictly. Bad values fail the build with an error naming the file and the value. Drafts are validated too (`CLAUDE.md:7-11, 29`).
- **Feed and sitemap** are generated in Rust, not Tera. XML escaping goes through `build/generate/xml.rs` (`CLAUDE.md:13`, `CLAUDE.md:62`).
- **Acceptance-criteria IDs:** tests are tagged with `// AC-<group>.<n>` comments that point to acceptance criteria. They appear in 18 files, e.g. `src/error.rs:57`, `src/build/output.rs:366` and `tests/build.rs:472`. **Decided by the maintainer (this run):** new specs continue this numbering. Groups `AC-1` to `AC-9` are in use at baseline, and IDs have been reused across earlier batches. Proposed reading, **confirm:** a new spec takes the next unused group number (`AC-10.1`, `AC-10.2`, ...), and its tests carry those IDs.
- **Module layout:** folder modules use `mod.rs` (`src/build/mod.rs`, `src/content/mod.rs`, `src/render/mod.rs`, `src/build/generate/mod.rs`, `src/build/index/mod.rs`).
- **Template names** (`page.html`, `section.html`, `home.html`, `tags.html`, `tag.html`) are hardcoded in `src/render/template.rs` (`CLAUDE.md:61`).

## Non-negotiables
From the documented "Definition of done" (`CLAUDE.md:71-82`) and the maintainer's decisions in this run:
- **The gate passes** (`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`), with no new warnings and no `#[allow]` added to hide one (source: `CLAUDE.md:75`).
- **No dead code, and no `#[allow(dead_code)]`.** Unused code is removed, not kept "for later" or silenced. There are no exceptions (source: maintainer decision, run `20260914-223413-bootstrap-specs-for-this-project`). The unused `ServerOpts` (`run`'s `--address`/`--port`) was removed after this run; `run` takes no options until it is implemented (`run_command_takes_no_options`).
- **Every behavior change or bug fix has a test that fails without it.** Anything visible from the CLI also gets an E2E test in `tests/build.rs` (source: `CLAUDE.md:76`).
- **A new success-path feature adds fixture content** and updates the `build_generates_site_from_fixture` manifest plus a `fixture_*` test. Error cases use temp-site E2E tests, never fixture files (source: `CLAUDE.md:77`).
- **Bad input fails before the output folder is cleaned,** proven with a `snapshot` test. The error is a `MangoError` on stderr with exit status 1, naming the file or path and the bad value (source: `CLAUDE.md:78`).
- **Output is deterministic, and a test enforces it.** Two builds of the same input must be byte-identical (sorted listings, no timestamps) (source: `CLAUDE.md:79`). The maintainer decided in this run that an automated test must check this: `fixture_build_is_deterministic` in `tests/build.rs` builds the fixture twice and compares every file.
- **Links and XML stay valid:** every internal link in the fixture output resolves (`fixture_internal_links_resolve`), and generated XML is escaped (source: `CLAUDE.md:79`).
- **No new dependencies without the maintainer's explicit approval.** Removing one is fine (source: `CLAUDE.md:80`). `tide` has been dropped (commit `dc66348`, not in `Cargo.toml`). Any remaining reference to it is stale and gets removed, not followed (source: maintainer decision, this run).
- **Docs change with the code:** `README.md` for anything a site author sees, `CLAUDE.md` for architecture, pipeline order and conventions (source: `CLAUDE.md:81`).
- **No TODO stubs, commented-out code, or `unwrap`/`expect` in production code.** Known limitations are listed in `README.md` (source: `CLAUDE.md:82`).
- **`ensure_safe_to_clean` must keep refusing** to clean the cwd, its parents, or a folder that is or contains the site, templates, assets or config (source: `CLAUDE.md:51`, `README.md:99`).
- **HTML output stays root-relative.** `base_url` is used only by the feed and sitemap (source: `CLAUDE.md:7`).
- Keep diffs reviewable by formatting only what you change (**Proposed, confirm**; the wording comes from the gitignored local `tasks.md`, which is not project policy).

## When to use /spec-feature
**Proposed, confirm.**
- **Use `/spec-feature` for:**
  - public or interface changes: CLI subcommands or flags, config fields in `mango.json`, template names or template context fields
  - data or format changes: frontmatter rules, slug/URL/output path rules, feed or sitemap format
  - behavior site authors rely on: validation strictness, collision rules, clean safety, ordering of listings
  - changes across multiple components, e.g. a new output type touching `content/`, `render/`, `build/` and `cli.rs`
  - anything that needs a new dependency
- **Use `/ship-feature` for:**
  - small local changes and bug fixes inside one module
  - internal refactors covered by existing tests, including dead-code removal that doesn't change the CLI surface
  - test-only changes, such as adding the determinism test
  - docs-only changes (`README.md`, `CLAUDE.md`)

## Related docs
- `CLAUDE.md`: architecture, pipeline order, template contexts, module map, conventions and the definition of done. This is the authoritative developer doc.
- `README.md`: the user guide (commands and flags, content rules, config, output URLs, templates, safe builds, known limitations).
- `example/mango.json`, `example/site/`, `example/meta/`: the fixture site that shows every success-path feature.
- `specs/_system/backlog.md`: tracked improvements (architecture review, risks, gaps) with IDs, priorities and suggested workflows. Update an item's status when work on it starts and when it lands.
- `LICENSE`: GPL-3.0-or-later.
- `.claude/pipeline/workflows/*.yaml`, `.claude/commands/*.md`: dev-pipeline workflow definitions.

`tasks.md` (gitignored, local only) is not project policy, and the pipeline should not use it as a source of rules.

## Conflicts found
None open. All four conflicts from the previous draft were resolved by the maintainer and are recorded above:
1. `#[allow(dead_code)]` on `ServerOpts`: not an exception. The rule is no dead code (see Non-negotiables).
2. Stale `tide` references (e.g. `tasks.md:109`): to be removed. `tasks.md` is not policy.
3. `AC-x.y` IDs: new specs continue the existing numbering (see Conventions).
4. Determinism: must be enforced by a test (see Non-negotiables).

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-14 | 20260914-223413-bootstrap-specs-for-this-project | Initial draft from CLAUDE.md, README.md, Cargo.toml, rust-toolchain.toml, .gitignore and tasks.md, checked against src/ and tests/build.rs |
| 2026-09-14 | 20260914-223413-bootstrap-specs-for-this-project | Applied the maintainer's decisions: added "no dead code, no `#[allow(dead_code)]`" (replaces "`ServerOpts` is kept"); stale `tide` references are removed; dropped `tasks.md` from Related docs; new specs continue `AC-x.y` numbering (next group proposed as `AC-10`); determinism must be enforced by a test. Marked both code rules as not yet compliant at baseline. Conflicts 1-4 closed. |
| 2026-09-14 | follow-up to 20260914-223413-bootstrap-specs-for-this-project | Both code rules now met: removed the unused `ServerOpts` and its `#[allow(dead_code)]` (`run` takes no options, `run_command_takes_no_options`); added `fixture_build_is_deterministic`; removed the last stale `tide` reference from `tasks.md`. |
