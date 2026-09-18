# RISK-2 — symlinked folders inside the assets folder

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260916-205751-specs-system-backlog-md-risk-2-symlinked/01-plan.v1.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260916-205751-specs-system-backlog-md-risk-2-symlinked`
- Groups defined here: `AC-10`
- Criteria: 12

## Requirements

### REQ-1 A symlinked folder inside the assets folder fails the build before anything is cleaned
Planning must classify entries by what they resolve to, and refuse a symlink that resolves to a directory with a clear error, so the failure happens in `assets::plan` (before `clean_contents`) instead of in `assets::copy` (after it).
- AC-10.1 `assets::plan` returns `Err(MangoError::General(_))` when an entry directly inside the assets folder is a symlink whose target is a directory. The message names the asset's relative path and says it is a symlinked folder, e.g. `asset 'vendor' is a symlink to a directory ('<source path>'): symlinked folders are not copied; replace it with a real folder`.
- AC-10.2 The same error is produced for a symlinked folder nested at any depth (e.g. `css/vendor` → a directory), and the relative path in the message uses `/` separators, matching the existing `asset '<relative path>'` label convention.
- AC-10.3 `assets::plan` creates nothing: the destination folder still does not exist after the rejection.
- AC-10.4 A `mango build` whose assets folder contains a symlinked folder exits with status 1, prints the error to stderr (nothing on stdout), and leaves the previous output **byte-for-byte unchanged**: a `snapshot(&out)` taken before the failing build equals `snapshot(&out)` after it.

### REQ-2 An unresolvable symlink inside the assets folder also fails during planning
A dangling symlink or a symlink loop currently survives planning and blows up in `copy` after cleaning, the same destructive path.
- AC-10.5 `assets::plan` returns an error naming the path when an entry is a symlink whose target does not exist (dangling link); the error is the standard `MangoError::io_at` form for that path.
- AC-10.6 `assets::plan` returns an error (rather than recursing forever or overflowing the stack) when an entry is a symlink loop (`a` → `b` → `a`).

### REQ-3 Symlinks that resolve to files, and a symlinked assets root, keep working
No currently working input may start failing.
- AC-10.7 `assets::plan` lists a symlink whose target is a regular file as a normal asset, and `assets::copy` writes the target's **contents** to the destination.
- AC-10.8 A `mango build` whose assets folder contains a symlink to a file succeeds and writes `dist/assets/<name>` with the target file's contents.
- AC-10.9 A build whose `--assets` path is itself a symlink to a directory still succeeds and copies that directory's files (regression guard for the `assets_source.is_dir()` check).

### REQ-4 Documentation and backlog reflect the rule
- AC-10.10 `README.md` states the rule where site authors will see it: under "Known limitations" (and/or the assets line), symlinked folders inside `meta/assets/` are a build error, while symlinks to files are copied as their contents.
- AC-10.11 `CLAUDE.md` is updated where assets planning is described (the `build` pipeline bullet and/or the `generate/assets.rs` entry in the module map) to say that `plan` resolves symlinks, rejects symlinked folders and unresolvable links, and that this keeps the failure before cleaning.
- AC-10.12 `specs/_system/backlog.md` RISK-2 is set to `done` with the commit hash in both the index table and the item body, per the backlog's own instructions.

## Verified by

From the run's test report (`03-test.v1.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-10.1 | new | `assets::tests::plan_rejects_symlinked_directory` + ad-hoc CLI (`asset 'vendor' is a symlink to a directory ('assets/vendor')`, `MangoError::General`) | pass |
| AC-10.2 | new | same test, nested `css/vendor` case + ad-hoc CLI (`asset 'css/vendor' …`, `/` separators) | pass |
| AC-10.3 | new | same test asserts `!dest.exists()` after the rejection | pass |
| AC-10.4 | new | `tests/build.rs::build_fails_on_symlinked_asset_folder_keeping_output` + ad-hoc: exit 1, stdout empty, stderr names the asset, sha256 manifest identical before/after | pass |
| AC-10.5 | new | `assets::tests::plan_fails_on_dangling_symlink` + ad-hoc (`Mango I/O Error at 'assets/broken.css': No such file or directory (os error 2)` — the `io_at` form) | pass |
| AC-10.6 | new | `assets::tests::plan_fails_on_symlink_loop` + ad-hoc (`Mango I/O Error at 'assets/b': Too many levels of symbolic links (os error 40)`, terminates, no stack overflow) | pass |
| AC-10.7 | new | `assets::tests::plan_and_copy_follow_symlinked_files` (asserts the copy is not a symlink and has the target's contents) | pass |
| AC-10.8 | new | `tests/build.rs::build_copies_symlinked_asset_file` + ad-hoc (`dist/assets/reset.css` is a regular file with the target's bytes) | pass |
| AC-10.9 | baseline | `tests/build.rs::build_accepts_symlinked_assets_root` + ad-hoc (`--assets` = symlink to a dir builds and copies) | pass |
| AC-10.10 | new | `README.md` "Known limitations" bullet covers folder link = error, file link = contents copied, symlinked `--assets` root allowed | pass |
| AC-10.11 | new | `CLAUDE.md` `build` pipeline bullet + `generate/assets.rs` module-map entry describe `fs::metadata` classification and "fails before cleaning" | pass |
| AC-10.12 | new | `specs/_system/backlog.md` RISK-2 = `done` in the index row and the item body, body records the resolution — **no commit hash** (see note below) | pass (with noted deviation) |
| — | baseline | Whole suite incl. `build_generates_site_from_fixture`, `fixture_build_is_deterministic`, `fixture_internal_links_resolve`, `build_fails_on_missing_assets_folder_keeping_output`, `plan_is_sorted_labelled_and_touches_nothing` | pass, unchanged |
