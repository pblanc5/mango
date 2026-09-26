# Reject symlinked folders in the site folder (RISK-1)

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260916-215211-risk-1-symlink-loops-in-the-site-folder/01-plan.v2.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260916-215211-risk-1-symlink-loops-in-the-site-folder`
- Groups defined here: `AC-11`
- Criteria: 15

## Requirements

### REQ-1 A symlinked folder in the site tree is a build error, not a recursion
`traverse` must classify each entry without following it first, and refuse to descend into a symlink that resolves to a directory. This removes the stack-overflow path entirely.

- AC-11.1 A symlink under the site folder that resolves to a directory fails `loader::load` with `MangoError::General`; the message contains `content folder '<rel>'` (path relative to the site folder, `/` as separator) and the phrase `symlink to a directory`. Verified for a link at the site root (`vendor`) and one nested in a real subfolder (`posts/vendor`).
- AC-11.2 A symlink pointing at the site folder itself or at an ancestor of its own location (`site/loop -> site`) fails with the same `content folder '<rel>'` error and the call returns normally — no unbounded recursion, and the test completes.
- AC-11.3 The error message tells the author what to do, ending with guidance to replace it with a real folder, and names the link's own path as walked (the `<path>` in the message is the path under the site folder, not the link target).

### REQ-2 Symlinks to files keep working
Today a symlinked markdown file is loaded and a symlink to anything else is ignored. Both behaviors survive the change, matching assets, which follows symlinked files.

- AC-11.4 A symlink under the site folder resolving to a markdown file outside the site is loaded as a page, and its slug comes from the link path (`site/posts/linked.md -> <outside>/shared/post.md` gives slug `posts/linked`).
- AC-11.5 A symlink resolving to a non-markdown file (e.g. `notes.txt`) is ignored: no page, no error.

### REQ-3 [baseline] An unresolvable symlink stays silently ignored
Today a dangling link and a link chain that loops are both invisible to the loader, because `is_dir()`/`is_file()` return `false` on an I/O error. The rewrite must not change that by accident: making them an error is a deliberate strictness change, deferred to `RISK-5` (AC-11.15). These criteria are regression guards, not new behavior. (These two IDs carried a different, now-withdrawn assertion in v1; they are re-cast here on the maintainer's instruction and keep their numbers.)

**Superseded 2026-09-25:** AC-11.6 and AC-11.7 no longer describe current behavior; they are superseded by `specs/risk-5/requirements.md` (RISK-5).

- AC-11.6 [baseline] A dangling symlink under the site folder (`site/posts/broken.md -> <nonexistent>`) does not fail `load`: the call succeeds, the link produces no page, and the other pages in the site are returned as usual. True whether or not the link's name ends in `.md`.
- AC-11.7 [baseline] A mutual link chain under the site folder (`a -> b`, `b -> a`) does not fail `load`: the call succeeds and terminates, and neither link produces a page.

### REQ-4 The site folder itself may still be a symlink
Entering through a symlinked root is how a shared content folder is legitimately used, and `assets` allows it (`build_accepts_symlinked_assets_root`).

- AC-11.8 `loader::load` on a path that is a symlink to a real directory succeeds and returns the pages inside it (the `!path.is_dir()` guard keeps its follow semantics).

### REQ-5 The CLI fails safely and visibly
- AC-11.9 E2E: a site built once, with a marker file added to the output, then rebuilt with `site/loop -> site` present. The build exits non-zero, stderr contains `content folder 'loop'` and `symlink to a directory`, and `snapshot(&out)` is byte-identical to the snapshot taken before the failing build.
- AC-11.10 E2E: a temp site containing a symlink to a markdown file outside the site builds successfully and writes that page's `index.html` at the slug taken from the link path.

### REQ-6 Docs and backlog follow the code
- AC-11.11 `README.md` "Known limitations" documents the site rule alongside the existing assets bullet: a symlink under `site/` may point at a file (it is read), a symlink to a folder is a build error, a broken or looping link is ignored (not an error, unlike in `meta/assets/`), and the `--site` folder itself may be a symlink.
- AC-11.12 `CLAUDE.md` states the rule where the loader is described (the `src/content/` module-map entry): entries are classified without following them first, symlinked folders are rejected, symlinked files are read, and an entry whose target cannot be resolved is skipped — including the note that this differs from `assets::plan`, which errors.
- AC-11.13 `specs/_system/backlog.md` sets RISK-1 to `done` in both the index row and the section, with a one-line resolution naming the behavior and the proving tests, in the style of the RISK-2 entry, and referencing `RISK-5` for the deferred unresolvable-link question.
- AC-11.14 `specs/_system/backlog.md` gains a new item `RISK-5` — status `open`, size **S**, priority `low`, workflow `/ship-feature`, in both the index table (in ID order, after RISK-4) and the "Risks and gaps" section — recording that an unresolvable symlink under `site/` (a dangling link, or a chain that loops) is silently ignored, that making it an error is a deliberate strictness change deferred from RISK-1, and that `assets::plan` already errors on both, so the two modules differ until this is settled.

### REQ-7 Delivery follows the project's git convention
- AC-11.15 The work lands on `master` as a single squash commit whose subject names the item, e.g. `Reject symlinked folders in the site folder (RISK-1)`, with a message that stands on its own; the feature branch is deleted. The gate `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` passes with no new warnings and no `#[allow]`.

## Verified by

From the run's test report (`03-test.v2.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-11.1 | new | `loader::tests::load_rejects_symlinked_directory` (root + nested); probe: nested link → `content folder 'aaa/deep/vendor'`, `/` separators | pass |
| AC-11.2 | new | `loader::tests::load_rejects_symlink_loop_to_ancestor`; probes with `loop -> site` (abs), `loop -> .`, `posts/up -> ..`, and a loop 5 real dirs deep — all exit 1, no output folder created | pass |
| AC-11.3 | new | assertions inside `load_rejects_symlinked_directory`; probe with target **outside** the site: message names only the walked path (target string absent) and ends `replace it with a real folder` | pass |
| AC-11.4 | new | `loader::tests::load_follows_symlinked_markdown_file` + E2E `build_reads_symlinked_markdown_file`; probe renders `dist/posts/linked/index.html` containing the target's title and body | pass |
| AC-11.5 | new | `loader::tests::load_ignores_symlinked_non_markdown_file`; probe: symlinked `notes.txt` produces no page | pass |
| AC-11.6 | baseline | `loader::tests::load_ignores_dangling_symlink`; CLI probe (`broken.md` + `broken.txt`) exits 0, empty stderr, only the real page; **tamper-proven** to break if `?` replaces the skip | pass |
| AC-11.7 | baseline | `loader::tests::load_ignores_symlink_loop_chain`; CLI probe (`a.md -> b.md -> a.md`) exits 0, terminates, only the real page; **tamper-proven** | pass |
| AC-11.8 | new | `loader::tests::load_accepts_symlinked_site_root`; probe through a symlinked `--site` renders the inner page | pass |
| AC-11.9 | new | E2E `build_fails_on_symlinked_content_folder_keeping_output`; independent probe md5-summed every output file before/after the failing rebuild — **byte-identical**, marker file intact, stderr correct | pass |
| AC-11.10 | new | E2E `build_reads_symlinked_markdown_file`; probe output: `<html>Shared <p>Body Shared</p></html>` at the link's slug | pass |
| AC-11.11 | new | `README.md` bullet (unchanged this attempt): file link read + URL from link path, folder link = error, broken/looping link ignored **unlike `meta/assets/`**, `--site` may be a symlink — all four points present and all four empirically confirmed by probes | pass |
| AC-11.12 | new | `CLAUDE.md:60` after rewording: classification with `fs::metadata` + non-following `is_symlink` before descending, `General` error with the message, symlinked md files read, unresolvable entry skipped, "deliberately unlike `assets::plan`, which errors on both", RISK-5 pointer — **nothing lost** in the rewrite | pass |
| AC-11.13 | new | `backlog.md`: RISK-1 `done` in the index row and the section header, resolution names the behavior, both proving tests, and `[RISK-5](#risk-5)`; history now correct (see below) | pass |
| AC-11.14 | new | `backlog.md`: RISK-5 row `low / S / /ship-feature / open` in ID order after RISK-4; section covers dangling + looping chain, the deferral from RISK-1, the `assets::plan` divergence, and the two pinning tests — only "stack overflow" → "runaway traversal" changed | pass |
| AC-11.15 | new (gate half) | fmt + clippy + test green; no `#[allow(`, no `unwrap`/`expect` in production code (all new `unwrap`s are inside `mod tests`), `Cargo.toml`/`Cargo.lock` untouched; git half still deliberately not performed | pass (gate half) |
