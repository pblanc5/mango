<!-- Published by dev-pipeline run 20260926-163650-risk-10-skip-hidden-entries, stage design v1, approved 2026-09-26T17:00:28Z. -->

# Hidden files and folders are skipped: Design

Spec ID: `risk-10` · Requirements: `specs/risk-10/requirements.md`

## Overview
A new crate-private function, `is_hidden(name: &OsStr) -> bool`, decides whether a name starts with `.`. Both directory walkers call it on each entry's own name (`DirEntry::file_name()`), straight after the entry is listed and before any other step. Those later steps are `fs::metadata`, `file_type`, reading the file, parsing frontmatter and building the slug. When the name is hidden, the walker does `continue`, so a hidden file is dropped and a hidden folder is never descended into. The check only ever sees names that come out of `read_dir`, so the `--site` and `--assets` folders themselves, and every folder above them, can never be skipped. No other stage changes. Sections, tags, the home page, the feed, the sitemap and the collision check all work from the loaded pages and planned assets, so a skipped entry leaves no trace in the output.

## Affected components
| Component / file | Change |
|---|---|
| `src/hidden.rs` (new) | `pub(crate) fn is_hidden(name: &OsStr) -> bool`, which holds the single rule. Unit tests. |
| `src/lib.rs` | Add `mod hidden;` (private, like every other module). |
| `src/content/loader.rs` | `traverse` skips hidden entries before `fs::metadata`. The comment that says "no name is exempt, hidden ones included" is rewritten. Tests are added, removed and replaced (see T-1 and T-3). |
| `src/build/generate/assets.rs` | `collect` skips hidden entries before `file_type()`, `strip_prefix` and `fs::metadata`. Tests are added (see T-1 and T-4). |
| `tests/build.rs` | New E2E tests for the skip, the dot-only replacement test and the fixture tests. The two new fixture tests need a small `build_with_config` helper. |
| `example/site/blog/.scratch.md`, `example/site/.notes/post-ideas.md`, `example/meta/assets/css/.stylelintrc.json` (new) | Hidden fixture entries. They are realistic and valid, and must never appear in the output. |
| `README.md`, `CHANGELOG.md`, `CLAUDE.md` | User and developer docs. |
| `specs/_system/overview.md`, `specs/risk-5/requirements.md`, `specs/arch-3/requirements.md`, `specs/_system/backlog.md` | Overview rows, supersession notes and the backlog status. |

## Approach
1. **One rule, one place.** `src/hidden.rs`:
   ```rust
   /// An entry is hidden when its own name, as listed in its parent folder,
   /// starts with `.` (RISK-10). Hidden entries under the site and assets
   /// folders are skipped without being resolved, read or checked.
   pub(crate) fn is_hidden(name: &OsStr) -> bool {
       name.as_encoded_bytes().first() == Some(&b'.')
   }
   ```
   - The function takes a name, not a path. That makes AC-3.4 hold by construction: the only callers pass `DirEntry::file_name()`, which is the entry's name "without any leading path component(s)". A root folder is never a `DirEntry` of its own walk.
   - Comparing the first encoded byte works on every platform, and for names that are not UTF-8. `OsStr::as_encoded_bytes` is a "self-synchronizing superset of UTF-8 … also a superset of 7-bit ASCII", so a first byte of `0x2E` means the first character is `.`.
   - Nothing else is consulted: not the extension, the file type, the link target, or the Windows hidden attribute (AC-3.2).
   - The function lives at the crate root, not in `content/`, because the asset planner in `build/generate/` uses it too. Neither walker owns the rule.
2. **Loader (`traverse`).** Directly after `let entry = result.map_err(...)?;`, and before `entry.path()` or `fs::metadata`:
   ```rust
   if is_hidden(&entry.file_name()) {
       continue;
   }
   ```
   A hidden folder is never passed to `traverse`, so nothing inside it is visited, whatever it is named (AC-4.2). The RISK-5 comment above the `fs::metadata` call currently ends with "This runs before any name check, so no name is exempt, hidden ones included." Replace that sentence with: "Hidden entries (a name starting with `.`) were skipped above without being resolved, so only visible entries get here, and no visible name is exempt."
3. **Assets (`collect`).** Directly after `let child = child.map_err(...)?;`, and before `child.path()`, `child.file_type()` and `fs::metadata`: `if is_hidden(&child.file_name()) { continue; }`. Put the skip first so that `file_type()` can't raise an error for a hidden entry either (AC-5.3).
4. **Dot-only names (AC-3.5).** Every file or folder name whose slug segment would be made only of dots starts with `.`, so it is now skipped before `Page::new`:
   - a folder named `.`-only is hidden;
   - `...md` has the stem `..` and `..md` has the stem `..`, and both names start with `.`.

   The dot-only branch in `Slug::parse` is therefore unreachable from the CLI. It stays, for three reasons. It is part of `Slug`'s invariant ("a `Slug` only exists after that validation"). `Slug::from_content_path` does not know its caller. And AC-7.7 requires the slug unit tests of that branch (`dot_only_segments_get_their_own_message`, `dot_only_file_names_are_rejected`) to stay. The branch is not dead code in the constitution's sense: `parse` is used, the branch is reachable through it, and those tests exercise it. `slug.rs` does not change.
5. **Why skipping is enough for every output (AC-4.1, AC-4.4, AC-5.4).**
   - Sections come only from ancestors of loaded pages (`index/section.rs`). A folder whose pages were all skipped, or a hidden folder, gets no section, exactly as if the entries had been deleted.
   - Tags, home, feed and sitemap all derive from the loaded pages.
   - Asset outputs are exactly the planned list.

   So the output of a site with hidden entries is the output of the same site without them, by construction. The E2E and fixture tests check this byte for byte.
6. **Silence (AC-4.5).** Nothing is logged. The binary writes to stderr only in `main.rs` on error. `clean.rs:247` has an `eprintln!`, but it is in a test.

## Interfaces and data
- New, crate-private: `hidden::is_hidden(name: &std::ffi::OsStr) -> bool`.
- No change to any public item (the seven re-exports), CLI flag, config field, template context, error text or `MangoError` variant.
- Observable change: hidden entries under `--site` and `--assets` produce no output and no error.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Filter on `path.file_name()` of the full path inside `traverse` | Equivalent in practice, but a path-based helper could be handed the root path by mistake, which would break AC-3.4. Taking the `DirEntry` name makes the root unreachable by construction. |
| Filter after loading, e.g. drop pages whose slug has a segment starting with `.` | The entry would still be resolved, read and validated, so a hidden dangling link or bad frontmatter would still fail the build (breaks AC-4.3 and AC-5.3). |
| Check hidden names only after `fs::metadata`, so broken hidden links still fail | This is the rejected alternative in Open question 3. AC-4.3 requires no inspection. |
| Copy the one-line check into `loader.rs` and `assets.rs` | Two copies of the rule can drift apart. REQ-3 asks for one rule for both folders. |
| Put `is_hidden` in `content/loader.rs` and import it from `assets.rs` | The asset planner would depend on the content loader for a rule that belongs to neither. A crate-root module is neutral. |
| Use `walkdir` or `ignore` | New dependency (AC-6.4, non-negotiable). The rule is one comparison. |
| Remove the now-unreachable dot-only branch from `Slug::parse` | It weakens `Slug`'s own invariant, and AC-7.7 requires its unit tests to stay, which would fail. |
| Also honor the Windows hidden attribute | Excluded by AC-3.2. It would also make the rule platform-dependent. |

## Risks
- **Breaking for authors.** Pages under hidden names disappear from the site, the feed and the sitemap without an error (AC-4.5 forbids reporting them). Mitigation: a `**Breaking:**` changelog entry that tells authors to rename such pages (AC-8.2), and a README rule that lists the skipped names.
- **An unreachable validation branch.** The dot-only branch of `Slug::parse` can no longer be hit from the CLI (see Approach 4). CLAUDE.md must say so, so that nobody later "fixes" an E2E test that can no longer fail.
- **Hidden-named `--assets` folder.** With `--assets .assets`, the asset destination is still named after the folder, so files go to `dist/.assets/`. That comes from how `plan` derives `asset_folder` (`pipeline.rs:110`), which is unchanged and out of scope. AC-3.4 only requires that the folder is not skipped. The fixture's assets folder is `assets`, so the AC-7.6 assertion is unaffected.
- **The templates folder is not filtered** (out of scope). An editor swap file there is still picked up by the Tera glob, as it is today.
- **Committed dotfiles in a public repository.** The fixture entries must be realistic and contain nothing private. Git and both CI platforms check out dotfiles normally.
- **Test platform coverage.** The symlink cases are `#[cfg(unix)]`, as in RISK-5, so Windows CI runs only the name-based cases.

Evidence for external behavior (Rust 1.92.0 standard library, `~/.rustup/toolchains/1.92.0-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/`):
- `DirEntry::file_name` "Returns the file name of this directory entry without any leading path component(s)" and returns `OsString`, not a `Result`, so it cannot fail (`fs.rs:2496-2521`).
- `read_dir`: "Entries for the current and parent directories (typically `.` and `..`) are skipped" (`fs.rs:3122-3123`). The skip never sees `.` or `..` as entries.
- `OsStr::as_encoded_bytes`, stable since 1.74 (the toolchain is 1.92): the encoding is "a self-synchronizing superset of UTF-8 … also a superset of 7-bit ASCII" (`ffi/os_str.rs:1052-1068`).
- A dangling or looping link is returned by the `read_dir` iterator as an `Ok` entry, and only `fs::metadata` fails on it. So a skip placed before `metadata` does exempt it. Evidence: the existing test `load_fails_on_dangling_symlink` (`src/content/loader.rs:634-664`) asserts that the error's path is the link itself. Iterator errors are mapped to the folder instead (`loader.rs:26-27`), so the failure comes from `metadata`. `load_fails_on_symlink_loop` shows the same for loops.

**Unverified assumptions:** None.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- Safety net:
  - The `[baseline]` criteria AC-1.1 to AC-1.5 and AC-2.1 to AC-2.3 describe exactly the behavior this change replaces for hidden names. Pinning them would only mean deleting the pins in T-3 and T-4, so no pins are added. The tests that do pin them (`load_publishes_hidden_markdown_file`, the `.#post.md` case, the dot-only tests) are removed or replaced under AC-7.7.
  - What has to survive unchanged is AC-6.1: visible entries behave as before. That is already covered by existing tests:
    - `load_fails_on_dangling_symlink` (visible cases), `load_fails_on_symlink_loop`, `load_fails_on_dangling_symlink_in_nested_folder`
    - `load_rejects_symlinked_directory`, `load_follows_symlinked_markdown_file`, `load_ignores_symlinked_non_markdown_file`, `load_accepts_symlinked_site_root`
    - `invalid_file_name_is_an_error_naming_the_file_even_for_drafts`, `draft_pages_are_excluded`
    - `plan_rejects_symlinked_directory`, `plan_fails_on_dangling_symlink`, `plan_fails_on_symlink_loop`, `plan_and_copy_follow_symlinked_files`
    - `build_fails_on_dangling_symlink_in_site_keeping_output`, `build_fails_on_symlinked_asset_folder_keeping_output`
  - Three behaviors that must **not** change are untested today, and T-1 pins them before any production change. Each test passes both before and after:
    - names starting with `_` are loaded and planned (AC-3.3);
    - a site folder or assets folder with a hidden name, or under a hidden ancestor, is walked (AC-3.4);
    - a visible symlink to a hidden-named file is read as a page (AC-3.2).
- New and changed behavior:
  - `hidden.rs` unit tests: AC-3.1, AC-3.2 (name only; not UTF-8), AC-3.3.
  - Loader unit tests: AC-4.1, AC-4.2 (AC-7.1); AC-4.3, AC-3.5 (AC-7.2); AC-3.2 (hidden link to a visible file).
  - Asset unit tests: AC-5.1, AC-5.2, AC-5.3 (AC-7.3).
  - E2E `build_skips_hidden_entries`: AC-4.4, AC-4.5, AC-5.4, AC-6.2 (AC-7.5).
  - E2E `build_skips_dot_only_file_name`: AC-3.5 (AC-7.7).
  - Fixture tests `fixture_publishes_no_hidden_entries` and `fixture_output_is_unchanged_without_hidden_entries`: AC-7.6, AC-6.3, AC-4.4, AC-5.4.
  - The unchanged manifest in `build_generates_site_from_fixture`, together with `fixture_build_is_deterministic` and `fixture_internal_links_resolve`: AC-6.3.

## Tasks
### T-1 Pin the names and roots that must not be skipped
- Kind: safety-net
- Satisfies: AC-3.2, AC-3.3, AC-3.4, AC-6.1, AC-7.1 (the `_` part), AC-7.4
- Files: `src/content/loader.rs`, `src/build/generate/assets.rs`
- Add these tests (test code only, no production change), tagged `// AC-risk-10.<req>.<m>`:
  - `src/content/loader.rs`:
    - `load_publishes_underscore_names` (AC-3.3): `_notes.md` and `_drafts/idea.md` load with the slugs `_notes` and `_drafts/idea`.
    - `load_walks_site_folder_with_hidden_name` (AC-3.4): the site is `fixture_dir(..).join(".site")` containing `posts/one.md`, and then `fixture_dir(..).join(".local").join("site")`. Both give `["posts/one"]`.
    - `#[cfg(unix)] load_reads_visible_link_to_hidden_file` (AC-3.2): `site/notes.md` is a symlink to `dir/.secret.md`, outside the site. It loads as `notes`.
  - `src/build/generate/assets.rs`:
    - `plan_copies_underscore_names` (AC-3.3): `_partials/a.css` is planned.
    - `plan_walks_assets_folder_with_hidden_name` (AC-3.4): `src = dir.join(".assets")` containing `css/main.css`. The labels are `["asset 'css/main.css'"]`.
- Done when: the new tests pass against the unchanged production code, and the gate passes.

### T-2 Add the hidden-name rule
- Kind: implementation
- Satisfies: AC-3.1, AC-3.2, AC-3.3, AC-6.4
- Files: `src/hidden.rs`, `src/lib.rs`
- Create `src/hidden.rs` with `is_hidden` as in Approach 1, including its doc comment, and add `mod hidden;` to `src/lib.rs`, next to the other private modules.
- Unit tests, tagged `// AC-risk-10.3.1, AC-risk-10.3.2, AC-risk-10.3.3`:
  - hidden: `.notes.md`, `.drafts`, `.git`, `.DS_Store`, `.#post.md`, `...md`, `..md`
  - not hidden: `notes.md`, `_notes.md`, `a.b`, `notes.`, `#post.md#`, `~notes.md`, `""`
  - `#[cfg(unix)]`: `OsStr::from_bytes(b".\xff")` is hidden and `b"\xff."` is not.
- `Cargo.toml` and `Cargo.lock` don't change.
- Done when: the unit tests pass, and the gate passes with no dead-code warning. `is_hidden` must be used by T-3 in the same change. Until T-3 lands, the item is unused and clippy's dead-code lint fails, so land T-2 and T-3 together before running the gate.

### T-3 Skip hidden entries in the content loader
- Kind: implementation
- Satisfies: AC-4.1, AC-4.2, AC-4.3, AC-4.4, AC-3.5, AC-6.1, AC-7.1, AC-7.2, AC-7.7, AC-1.1, AC-1.2, AC-1.3, AC-1.4, AC-1.5
- Files: `src/content/loader.rs`
- Production change: in `traverse`, `use crate::hidden::is_hidden;` and put `if is_hidden(&entry.file_name()) { continue; }` right after the entry is unwrapped, before `entry.path()` and `fs::metadata`. Rewrite the RISK-5 comment as described in Approach 2, and update the `load` doc comment to say that hidden entries are skipped.
- Remove `load_publishes_hidden_markdown_file` and its `// AC-risk-5.4.3` tag.
- Remove `".#post.md"` from the `load_fails_on_dangling_symlink` array. Keep the other three cases and the test's tags.
- Replace `dot_only_file_name_is_rejected_even_for_drafts` with `dot_only_file_names_are_skipped_as_hidden` (AC-3.5): `...md` (draft), `..md` and `posts/...md` next to `posts/one.md`. `load` succeeds with `["posts/one"]`.
- Add:
  - `load_skips_hidden_files_and_folders` (AC-4.1, AC-4.2): valid, non-draft `.notes.md`, `posts/.scratch.md`, `.drafts/secret.md` and `.drafts/deep/visible.md`, plus `posts/one.md`. The result is exactly `["posts/one"]`.
  - `hidden_entries_are_not_checked` (AC-4.3): `.broken.md` (malformed JSON), `posts/.nofm.md` (no frontmatter) and `.drafts/my notes.md` (valid frontmatter, invalid name). `load` succeeds.
  - `#[cfg(unix)] hidden_symlinks_are_not_resolved` (AC-4.3, AC-3.2), with `posts/one.md` in every case:
    - a dangling `.#post.md`;
    - a loop `.a.md` <-> `.b.md`;
    - `.vendor` linked to a folder outside the site that holds a page;
    - `.loop` linked to the site itself;
    - `.linked.md` linked to a visible markdown file outside the site.

    Each case loads exactly `["posts/one"]`.
- Done when: all loader tests pass, and a `grep` for `load_publishes_hidden_markdown_file`, `AC-risk-5.4.3` and `.#post.md` in the `load_fails_on_dangling_symlink` array finds nothing.

### T-4 Skip hidden entries in the asset planner
- Kind: implementation
- Satisfies: AC-5.1, AC-5.2, AC-5.3, AC-6.1, AC-7.3, AC-2.1, AC-2.2, AC-2.3
- Files: `src/build/generate/assets.rs`
- Production change: in `collect`, `use crate::hidden::is_hidden;` and put `if is_hidden(&child.file_name()) { continue; }` right after the entry is unwrapped, before `child.path()`, `file_type()` and `fs::metadata`. Update the `plan` doc comment to say that hidden entries are skipped.
- Add `plan_skips_hidden_files_and_folders` (AC-5.1, AC-5.2): `css/main.css`, `.DS_Store`, `css/.stylelintrc.json`, `.cache/x.css` and `.cache/nested/y.css`. The labels are exactly `["asset 'css/main.css'"]`, and after `copy_into` no `.DS_Store` exists under `dest`.
- Add `#[cfg(unix)] plan_does_not_resolve_hidden_symlinks` (AC-5.3): a dangling `.#main.css`, a loop `.a` <-> `.b` and `.vendor` linked to a folder. The plan succeeds with only the visible file.
- Done when: the asset tests pass, and the existing symlink tests still pass unchanged.

### T-5 E2E tests: hidden entries build cleanly, and the dot-only replacement
- Kind: test
- Satisfies: AC-4.4, AC-4.5, AC-5.4, AC-6.2, AC-3.5, AC-7.5, AC-7.7, AC-1.4, AC-1.5
- Files: `tests/build.rs`
- Add the helper `build_with_config(site, templates, assets, out, config) -> Output`, which passes `--config`, modeled on `build_with`.
- Add `build_skips_hidden_entries` (`// AC-risk-10.7.5, AC-risk-10.4.4, AC-risk-10.4.5, AC-risk-10.5.4, AC-risk-10.6.2`). The test builds two twins under `temp_dir`, `with/` and `without/`. Each has `site/` and `assets/`, where `assets/` is a copy of `example/meta/assets` so the destination folder name matches. Both twins use the shared `mango.json` with `{"base_url": "https://example.com"}` and the fixture templates.
  - Visible content, in both twins: `posts/one.md` (dated, tagged `rust`) and `about.md`.
  - Only `with/` also gets:
    - `site/.notes.md` (dated, tagged `secret`);
    - `site/posts/.scratch.md`;
    - `site/.drafts/secret.md` and `site/.drafts/my notes.md`;
    - `site/posts/.broken.md` (malformed);
    - `assets/.DS_Store` and `assets/css/.cache/x.css`;
    - on Unix only: dangling symlinks `site/.#post.md`, `site/posts/.#one.md` and `assets/.#main.css`.
  - Assert that both builds exit 0 and that `with`'s stderr is empty.
  - Assert that the `snapshot` manifests are equal and that every file is byte-identical.
  - Assert that no output path segment starts with `.`, and that `posts/one/index.html` exists.
  - Then add a visible `site/posts/bad.md` without frontmatter to `with/` and rebuild. Assert exit 1, that stderr names `bad.md`, and that `snapshot` is unchanged (AC-6.2).
- Replace `build_fails_on_dot_only_file_name_keeping_output` with `build_skips_dot_only_file_name` (`// AC-risk-10.3.5, AC-risk-10.7.7`): build `posts/one.md`, then add `...md` and `posts/..md` and rebuild.
  - The rebuild succeeds with empty stderr.
  - The `snapshot` and the bytes of `index.html` are unchanged.
  - `dir.join("index.html")` does not exist.
- Done when: both tests pass on Linux. On Windows, the non-symlink parts run.

### T-6 Hidden entries in the example fixture, and fixture tests
- Kind: test
- Satisfies: AC-7.6, AC-6.3, AC-4.1, AC-4.2, AC-4.4, AC-5.1, AC-5.4
- Files: `example/site/blog/.scratch.md`, `example/site/.notes/post-ideas.md`, `example/meta/assets/css/.stylelintrc.json`, `tests/build.rs`
- Fixture content:
  - `example/site/blog/.scratch.md`: Wren's scratch pad. Valid JSON frontmatter, `"draft": false`, `"date": "2026-05-10"`, `"tags": ["scratch"]`, and a short realistic body such as a half-written paragraph and a to-do list.
  - `example/site/.notes/post-ideas.md`: a list of future post ideas. Valid frontmatter, `"draft": false`, `"date": "2026-05-12"`, `"tags": ["ideas"]`.
  - `example/meta/assets/css/.stylelintrc.json`: a small valid stylelint config, e.g. `{"extends": "stylelint-config-standard", "rules": {"color-hex-length": "short"}}`.
  - Both pages are dated later than every other fixture page and use tags nowhere else in the fixture. If the skip regressed, they would top the home page and the feed and add `tags/scratch/` and `tags/ideas/`, so the unchanged manifest would catch it.
- In `tests/build.rs`, leave the `build_generates_site_from_fixture` manifest **unchanged**.
- Add `fixture_publishes_no_hidden_entries` (`// AC-risk-10.7.6`). It asserts:
  - the three hidden source files exist, so the test is not vacuous;
  - no segment of any `snapshot(fixture_dist())` path (split on `/` and `\`) starts with `.`;
  - no output file contains the two hidden pages' titles;
  - `feed.xml` and `sitemap.xml` don't contain `/.`.
- Add `fixture_output_is_unchanged_without_hidden_entries` (`// AC-risk-10.6.3, AC-risk-10.4.4, AC-risk-10.5.4`):
  - `copy_dir` `example/site` and `example/meta/assets` into `temp_dir(..)/site` and `temp_dir(..)/assets`;
  - delete the three hidden paths from the copies;
  - build with `example/meta/templates` and `example/mango.json` (through `build_with_config`);
  - assert the manifest and every file's bytes equal `fixture_dist()`.
- Done when: the fixture build succeeds, the existing fixture tests (manifest, determinism, links) pass unchanged, and both new tests pass.

### T-7 User and developer docs
- Kind: docs
- Satisfies: AC-8.1, AC-8.2, AC-8.3
- Files: `README.md`, `CHANGELOG.md`, `CLAUDE.md`
- `README.md`, Content section:
  - Remove the sentence "A folder name or file name (without its extension) made only of dots, such as `...md`, is rejected, since it would publish the page outside its folder."
  - Add a paragraph saying that files and folders whose names start with `.` are skipped under `site/` and `meta/assets/`, at any depth, together with everything inside a hidden folder. Give the examples `site/.notes.md`, `site/.drafts/` and `meta/assets/.DS_Store`. Say that such entries are not read or checked, so an editor lock or swap file or a hidden broken symlink never fails the build. Say that names starting with `_` are not skipped (`site/_notes.md` is published at `/_notes/`), and that the `--site` and `--assets` folders themselves may have any name.
- `README.md`, Known limitations: amend both symlink lines. The `site/` line's "whatever the link's name" becomes "whatever the link's name, unless it starts with `.`: hidden entries are skipped without being checked". Add the same note to the assets line.
- `CHANGELOG.md`: under `## [Unreleased]`, add a `### Changed` group with one entry starting `**Breaking:**`, written for site authors. It says:
  - hidden files and folders (names starting with `.`) under the site folder are no longer published, listed, tagged or added to the feed or sitemap;
  - hidden files under the assets folder are no longer copied;
  - hidden entries are not read or checked, so a hidden broken symlink, such as an editor lock file (`.#post.md`), no longer fails the build;
  - names starting with `_` are unaffected;
  - a page that should stay published must be renamed.
- `CLAUDE.md`:
  - **Pipeline line:** "load markdown files … (hidden entries skipped)".
  - **"File names are strict":** a dot-only segment can no longer come from the loader, because such a name starts with `.` and is skipped as hidden. `Slug` still rejects dot-only segments as part of its own invariant, and the slug unit tests pin that. Say that this is why no E2E test of the dot-only error exists.
  - **Module map:** add a `src/hidden.rs` entry (`is_hidden`, the single rule, used by `loader::traverse` and `assets::collect` on `DirEntry::file_name()` before any resolution).
  - **`content/loader.rs` entry:** "whatever its name" becomes "whatever its name, except hidden entries, which are skipped before resolution (RISK-10)". Also say that hidden entries are skipped first.
  - **`generate/assets.rs` entry:** the same note.
  - **`example/` paragraph:** add the hidden fixture entries (`blog/.scratch.md`, `.notes/post-ideas.md`, `meta/assets/css/.stylelintrc.json`), which must never appear in the output.
- Done when: every clause of AC-8.1 to AC-8.3 can be pointed to in the diff, and no remaining sentence in these files says that `...md` is rejected, or that a hidden name fails without the exception.

### T-8 System overview, related specs and backlog
- Kind: docs
- Satisfies: AC-8.4, AC-8.5, AC-8.6
- Files: `specs/_system/overview.md`, `specs/risk-5/requirements.md`, `specs/arch-3/requirements.md`, `specs/_system/backlog.md`
- `specs/_system/overview.md`:
  - Add a Components row for `src/hidden.rs`.
  - Loader row: say that hidden entries are skipped, and cite `load_skips_hidden_files_and_folders`.
  - Slug row: replace the removed `build_fails_on_dot_only_file_name_keeping_output` with `build_fails_on_invalid_file_name_naming_file` only, or drop the citation.
  - `traverse` risk row: add the hidden-entry rule. Unresolvable entries fail "unless hidden (RISK-10, done; `hidden_symlinks_are_not_resolved`)".
  - Assets risk row: add the same rule, citing `plan_does_not_resolve_hidden_symlinks`.
  - Add a Changelog row.
- `specs/risk-5/requirements.md`:
  - After AC-2.3, append **"Superseded for hidden names 2026-09-26 by specs/risk-10/requirements.md:** an entry whose name starts with `.`, such as `.#post.md`, is skipped without being resolved."
  - After AC-4.3, append **"Superseded 2026-09-26 by specs/risk-10/requirements.md:** hidden entries are skipped, not published."
  - Keep the original wording of both criteria, and add a Changelog row.
- `specs/arch-3/requirements.md`:
  - After AC-8.1 and after AC-8.2, append a note: **"No longer applies to files found by the content loader since 2026-09-26 (specs/risk-10/requirements.md):** every dot-only name starts with `.`, so the loader skips it as hidden before any file-name check. The slug type still rejects dot-only segments (AC-5.2, AC-7.4)." On AC-8.2, also say that its E2E test was replaced by `build_skips_dot_only_file_name`.
  - Keep the original wording, and add a Changelog row.
- `specs/_system/backlog.md`: the RISK-10 index row becomes `done`, and the section header line becomes `· done`. Add a "Resolved by `specs/risk-10/`: …" note. The note says which names are skipped, where, that they are not checked (which reverses RISK-5's no-exemption choice for hidden names), that `_` is unaffected, and that it is breaking and recorded in `CHANGELOG.md`. It names `build_skips_hidden_entries` as the E2E proof.
- Done when: no document cites a removed test (`load_publishes_hidden_markdown_file`, `dot_only_file_name_is_rejected_even_for_drafts`, `build_fails_on_dot_only_file_name_keeping_output`), apart from these notes and the changelog rows that record the removal.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-3 (superseded for hidden names; its pinning test is removed) |
| AC-1.2 | T-3 (superseded) |
| AC-1.3 | T-3 (superseded; `.#post.md` case removed) |
| AC-1.4 | T-3, T-5 (superseded; tests replaced) |
| AC-1.5 | T-5 (superseded; E2E shows hidden pages are absent everywhere) |
| AC-2.1 | T-4 (superseded) |
| AC-2.2 | T-4 (superseded) |
| AC-2.3 | T-4 (superseded) |
| AC-3.1 | T-2, T-3, T-4 |
| AC-3.2 | T-1, T-2, T-3 |
| AC-3.3 | T-1, T-2 |
| AC-3.4 | T-1 |
| AC-3.5 | T-3, T-5 |
| AC-4.1 | T-3, T-6 |
| AC-4.2 | T-3, T-6 |
| AC-4.3 | T-3 |
| AC-4.4 | T-3, T-5, T-6 |
| AC-4.5 | T-5 |
| AC-5.1 | T-4, T-6 |
| AC-5.2 | T-4 |
| AC-5.3 | T-4 |
| AC-5.4 | T-5, T-6 |
| AC-6.1 | T-1, T-3, T-4 |
| AC-6.2 | T-5 |
| AC-6.3 | T-6 |
| AC-6.4 | T-2 |
| AC-7.1 | T-1, T-3 |
| AC-7.2 | T-3 |
| AC-7.3 | T-4 |
| AC-7.4 | T-1 |
| AC-7.5 | T-5 |
| AC-7.6 | T-6 |
| AC-7.7 | T-3, T-5 |
| AC-8.1 | T-7 |
| AC-8.2 | T-7 |
| AC-8.3 | T-7 |
| AC-8.4 | T-8 |
| AC-8.5 | T-8 |
| AC-8.6 | T-8 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-26 | 20260926-163650-risk-10-skip-hidden-entries | Initial design. One crate-private rule, `hidden::is_hidden`, is applied to each directory entry's own name first thing in `loader::traverse` and `assets::collect`, so hidden entries are never resolved, read or checked, and the `--site` and `--assets` folders can never be skipped. `Slug`'s dot-only check is kept as the type's invariant. Hidden fixture entries are added with an unchanged manifest. Docs, supersession notes in risk-5 and arch-3, and the backlog are updated. 8 tasks. |
