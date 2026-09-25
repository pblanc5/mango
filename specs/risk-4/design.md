<!-- Published by dev-pipeline run 20260925-182336-risk-4-unknown-frontmatter-keys, stage design v1, approved 2026-09-25T19:40:00Z. -->

# Strict frontmatter keys: Design

Spec ID: `risk-4` · Requirements: `specs/risk-4/requirements.md`

## Overview
`frontmatter::parse` gets a key check that runs before the typed parse. The JSON block is parsed once into a `serde_json::Value`. If the result is an object, its keys are compared with the six accepted keys and the four required ones, and every unknown and missing key goes into one `Frontmatter` error with a fixed order. If there are no key errors, or the block is not a JSON object, or it is not valid JSON, the code runs today's `serde_json::from_str::<MangoFrontmatter>` unchanged, so every other error reads exactly as it does now. The loader already puts the file path in front of the message. `Page::new` (date, then tags, then slug) runs only after `parse` returns, so key errors come before everything else without any change to `Page::new` or the loader.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/frontmatter.rs` | Add `ACCEPTED_KEYS` / `REQUIRED_KEYS` constants, `check_keys`, and a private `parse_json` helper that calls `check_keys` and then the existing typed parse. Add `#[serde(deny_unknown_fields)]` to `MangoFrontmatter` so the struct and the constant can't drift apart. New unit tests. |
| `src/content/loader.rs` | No production change. New unit tests for precedence (AC-5.3), drafts (AC-3.3) and baseline pins. |
| `tests/plan.rs` | Add an "unknown key" case to `draft_with_bad_frontmatter_date_or_tag_fails_planning`. |
| `tests/build.rs` | New E2E test: stderr, exit status and an unchanged output snapshot. |
| `README.md` | A sentence under the frontmatter field table. |
| `CHANGELOG.md` | A `### Changed` entry under `## [Unreleased]`. |
| `CLAUDE.md` | Frontmatter rules paragraph and the `frontmatter.rs` module-map entry. |
| `specs/_system/backlog.md` | RISK-4 status `open` → `done` (index row and section header). |
| `specs/_system/overview.md` | `frontmatter.rs` row (line 25) and the risk row (line 110): drop "unknown keys silently ignored". |

## Approach
1. **Parse twice, check keys in between.** In `parse`, replace the line `serde_json::from_str::<MangoFrontmatter>(&json).map_err(...)` with `parse_json(&json)`:
   ```rust
   fn parse_json(json: &str) -> Result<MangoFrontmatter, MangoError> {
       if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) {
           check_keys(&map)?;
       }
       serde_json::from_str::<MangoFrontmatter>(json)
           .map_err(|e| MangoError::Frontmatter(e.to_string()))
   }
   ```
   - When the value parse fails, or gives a non-object, the code goes straight to today's typed parse. That parse returns today's exact error (AC-5.1), with line and column.
   - When the value is an object with no key errors, the typed parse reports wrong types and duplicate known keys exactly as before (AC-5.4). `Page::new` then checks the date, the tags and the slug in its current order (AC-5.4).
   - When there are key errors, `check_keys` returns first, so wrong types, dates, tags and file names are never seen (AC-5.2, AC-5.3). This doesn't depend on document order, because the key check reads the whole map before reporting anything.
2. **Every key counts once.** `serde_json::Map` keeps one entry per key, with the last duplicate winning. That holds for both the default `BTreeMap` and the `preserve_order` `IndexMap`, so each key is named at most once (AC-4.6). A key counts as present whatever its value, including `null` (REQ-4 intro).
3. **Fixed order, whatever map is underneath.** Unknown keys are collected into a `Vec<&str>` and sorted explicitly with `sort_unstable()`, which is byte-wise for `str` (AC-4.4). The sort doesn't rely on the map's iteration order, which a transitive dependency could change through feature unification (`preserve_order`). Missing keys are listed by iterating `REQUIRED_KEYS` in order: `title`, `author`, `description`, `draft`.
4. **Exact comparison.** `ACCEPTED_KEYS.contains(&key)` compares strings exactly, so case and whitespace count (AC-3.2).
5. **Drafts.** The check runs in `parse`, before `draft` is read, so drafts are checked too (AC-3.3). The loader filters drafts only after all pages load.
6. **Guard against drift.** With `#[serde(deny_unknown_fields)]`, a name in `ACCEPTED_KEYS` that isn't a struct field fails the typed parse. A unit test builds a document from `ACCEPTED_KEYS` and parses it with the typed parser alone, so adding a field in one place but not the other breaks a test. At runtime the attribute is unreachable, because `check_keys` runs first. It adds no code, so the dead-code rule doesn't apply.

## Interfaces and data
- New private items in `src/content/frontmatter.rs`:
  - `const ACCEPTED_KEYS: [&str; 6] = ["title", "author", "description", "date", "tags", "draft"];` (the field table's order)
  - `const REQUIRED_KEYS: [&str; 4] = ["title", "author", "description", "draft"];`
  - `fn check_keys(map: &serde_json::Map<String, serde_json::Value>) -> Result<(), MangoError>`
  - `fn parse_json(json: &str) -> Result<MangoFrontmatter, MangoError>`
- `pub fn parse` keeps its signature. `MangoFrontmatter` keeps its fields and gains `#[serde(deny_unknown_fields)]`.
- **Error message** (a `MangoError::Frontmatter` payload; the loader adds `<path>: ` in front and `Display` adds `Mango Frontmatter Error: `):
  - Problems are joined with `, `. Each one is `unknown '<key>'` or `missing '<key>'`, in the AC-4.4 order.
  - The payload starts with `invalid frontmatter keys: `.
  - If there is at least one unknown key, it ends with `; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`. With only missing keys, this suffix is left out.
  - Example (AC-4.2): `invalid frontmatter keys: unknown 'tag', unknown 'titel', missing 'title'; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`
  - Example (AC-4.5, `{}`): `invalid frontmatter keys: missing 'title', missing 'author', missing 'description', missing 'draft'`
  - Keys are printed raw inside single quotes, the same way dates and tags are.
- No schema, CLI, config or template changes.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Only add `#[serde(deny_unknown_fields)]` | serde stops at the first problem, in document order. That reports one key, puts a wrong-typed value ahead of a later unknown key (breaks AC-5.2), and uses serde's backtick wording. It fails AC-4.2 to AC-4.5. |
| Parse only into `Value`, then `serde_json::from_value::<MangoFrontmatter>` | `from_value` errors lose line and column, and it would silently accept duplicate known keys that today are a `duplicate field` error. Both change AC-5.4 behavior that the spec keeps. |
| Hand-written `Deserialize` / `#[serde(flatten)] extra: Map` field | `flatten` gets in the way of `deny_unknown_fields` and changes how serde reports errors. A custom visitor would still fail on a wrong type before it had seen every key (AC-5.2). It's more code for a worse result. |
| Check keys in `loader.rs` or `Page::new` | By then the typed parse has already failed on wrong types or duplicates, so key errors couldn't take precedence. `frontmatter.rs` is where the JSON is parsed. |
| Enable `serde_json`'s `preserve_order` to report keys in document order | The spec fixes a sorted order (AC-4.4), and turning on a feature needs the maintainer's approval. |

## Risks
- **Two parses of every frontmatter block.** The blocks are small, so the cost is negligible compared with rendering.
- **Does a failed `Value` parse imply a failed typed parse?** Both use the same tokenizer and the same recursion limit, so JSON that `Value` rejects can't satisfy the typed parser either. In any case, that branch simply runs today's code, so behavior is identical by construction. The AC-5.1 tests compare against the typed parser's own error text.
- **A quirk that stays.** serde's derived struct deserializer also accepts a JSON *array* in field order (e.g. `["t","a","d",null,null,false]`), and today such a page builds. The design runs today's path for every non-object (AC-5.1 "as it does today"), so the quirk stays. An array has no keys, so REQ-3 doesn't apply. It's not in scope. It could be a follow-up backlog item: "frontmatter must be an object".
- **Breaking change.** A site that uses extra keys stops building. This is accepted by the maintainer and recorded in `CHANGELOG.md` (AC-7.2). The example site uses only the six keys (checked: every `example/site/**/*.md` frontmatter). The block in `blog/how-i-write.md` lines 21-25 sits inside a code fence in the body, not in the frontmatter.
- **Keys with control characters or quotes** are printed raw. That matches how tags and dates are reported today. Acceptable.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- Tag new tests with spec-scoped IDs: `// AC-risk-4.<REQ>.<n>` (AC-3.1 → `AC-risk-4.3.1`).
- **Safety net:** I searched `src/` and `tests/` for `missing field`, `invalid type`, `duplicate field` and `expected struct` and found nothing.
  - Already covered:
    - AC-1.1: `parses_valid_frontmatter_and_returns_body`, `optional_fields_may_be_omitted`
    - AC-1.2, malformed JSON: `malformed_json_is_an_error`, `subdirectory_malformed_frontmatter_is_an_error_naming_the_file`
    - AC-1.3: `malformed_draft_is_still_an_error`, `draft_with_bad_frontmatter_date_or_tag_fails_planning`
  - Uncovered parts of AC-1.2, pinned by T-1 before any production change:
    - a missing required key
    - a wrong-typed value
    - a duplicate known key
    - valid JSON that isn't an object
  - AC-2.1 is the behavior being replaced, so it gets no pin. The T-2 tests fail against it.
- **New and changed behavior:**
  - `frontmatter.rs` unit tests (T-2):
    - AC-3.1, AC-3.2, AC-3.4
    - AC-4.2 to AC-4.6
    - AC-5.1, AC-5.2
    - the drift guard
  - `loader.rs` unit tests (T-3):
    - AC-3.3
    - AC-5.3, with the file path in the message (AC-4.2)
    - AC-5.4: the existing `frontmatter_error_wins_over_invalid_file_name` plus the T-1 pins
  - `tests/plan.rs` (T-3): AC-3.3 through the public API.
  - `tests/build.rs` E2E (T-4): AC-4.1, AC-4.2, AC-4.3, AC-3.3 and AC-6.1 (snapshot).
  - AC-3.5: the existing `build_generates_site_from_fixture`, `fixture_build_is_deterministic` and `fixture_*` tests must stay green with no change to the fixture or the manifest.

## Tasks
### T-1 Pin current frontmatter error behavior
- Kind: safety-net
- Satisfies: AC-1.2, AC-5.4
- Files: `src/content/frontmatter.rs`, `src/content/loader.rs`
- Done when:
  - These tests exist and pass on the **unchanged** production code:
    - (a) `frontmatter.rs`: a block with `"title": 5` (all other keys valid) gives `Err(Frontmatter(msg))`, where `msg` equals the typed parser's error text and contains `invalid type`.
    - (b) `frontmatter.rs`: a duplicate `"title"` (all keys otherwise valid) gives a `Frontmatter` error containing `duplicate field`.
    - (c) `frontmatter.rs`: the blocks `42` and `"text"` each give a `Frontmatter` error equal to `serde_json::from_str::<MangoFrontmatter>(block)`'s error text.
    - (d) `loader.rs`: a page without `title` fails with a `Frontmatter` error whose message starts with `Mango Frontmatter Error: <file path>: `.
  - None of them asserts the wording of the missing-key message, because that wording changes in T-2.

### T-2 Key check in `frontmatter::parse`
- Kind: implementation
- Satisfies: AC-1.1, AC-3.1, AC-3.2, AC-3.4, AC-4.2, AC-4.3, AC-4.4, AC-4.5, AC-4.6, AC-5.1, AC-5.2, AC-5.4
- Files: `src/content/frontmatter.rs`
- Done when:
  - `ACCEPTED_KEYS`, `REQUIRED_KEYS`, `check_keys`, `parse_json` and `#[serde(deny_unknown_fields)]` are in place as described under Interfaces.
  - New unit tests pass:
    - an unknown `tag` → exact message including the accepted-keys suffix
    - `Title`, `TAGS` and `"title "` are each reported as unknown
    - `titel` + `tag` with no `title` → the exact AC-4.2 message, identical for two different key orders in the document
    - `{}` → the exact AC-4.5 message, without the suffix
    - an unknown key repeated twice, and a known key repeated alongside an unknown key → each named once, and no `duplicate field`
    - `"title": null` plus an unknown key → `title` not reported missing
    - `"title": 5` / `"draft": "no"` placed both before and after an unknown key → key-error message, no `invalid type`
    - the drift guard: an object built from all of `ACCEPTED_KEYS` parses with the typed parser, removing any one of `REQUIRED_KEYS` fails it, and adding any other key fails it
  - All existing `frontmatter.rs` tests, including the CRLF and BOM tests, and the T-1 pins still pass.

### T-3 Loader and in-process precedence and draft tests
- Kind: test
- Satisfies: AC-3.3, AC-4.2, AC-5.3, AC-5.4
- Files: `src/content/loader.rs`, `tests/plan.rs`
- Done when:
  - `loader.rs` has a test where `my posts/x.md` has frontmatter with `"date": "2026-02-30"`, `"tags": ["Rust"]` and an extra `"tag": []`. The load fails with a `Frontmatter` error that contains the file path and `unknown 'tag'`, and does not contain `invalid date`, `'Rust'` or `invalid file name`.
  - `loader.rs` has a test where a draft page with an unknown key fails the load.
  - `draft_with_bad_frontmatter_date_or_tag_fails_planning` has an `"unknown key"` case (a raw draft page with an extra `"dates"` key) asserting `Frontmatter`, the path and `unknown 'dates'`.
  - `frontmatter_error_wins_over_invalid_file_name` still passes unchanged.

### T-4 End-to-end: unknown keys fail the build safely
- Kind: test
- Satisfies: AC-3.1, AC-3.3, AC-4.1, AC-4.2, AC-4.3, AC-6.1
- Files: `tests/build.rs`
- Done when:
  - A new test `build_fails_on_unknown_frontmatter_keys_keeping_output` passes:
    - It first builds a temp site successfully, writes `marker.txt` into `dist/` and takes a `snapshot`.
    - It then adds `posts/bad_keys.md` with keys `titel` and `tag` and no `title`, and rebuilds with `build_temp_site`.
    - The rebuild fails (`assert_failure`: non-zero exit, empty stdout).
    - stderr contains `bad_keys.md` and the exact text `unknown 'tag', unknown 'titel', missing 'title'; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`.
    - The snapshot is unchanged.
  - The same test, or a sibling, shows that a draft page (`"draft": true`) with an unknown key also exits non-zero and names the file.
  - All `fixture_*` tests and `build_generates_site_from_fixture` still pass with no fixture change (AC-3.5).

### T-5 User-facing docs and changelog
- Kind: docs
- Satisfies: AC-7.1, AC-7.2
- Files: `README.md`, `CHANGELOG.md`
- Done when:
  - In `README.md`, directly under the frontmatter field table, a sentence says:
    - any other key is an error, which catches typos such as `tag` for `tags`, drafts included
    - a page with bad keys gets one error that lists every unknown and every missing key, and the accepted keys
  - `CHANGELOG.md` has, under `## [Unreleased]`, a `### Changed` group with an entry for site authors. It says that unknown frontmatter keys now fail the build (drafts included), with one error per page listing every unknown and missing key, that this is a **breaking** change, and that a site that used extra keys must remove them.

### T-6 Developer docs and tracking
- Kind: docs
- Satisfies: AC-7.3
- Files: `CLAUDE.md`, `specs/_system/backlog.md`, `specs/_system/overview.md`
- Done when:
  - In `CLAUDE.md`, the "Frontmatter is JSON" paragraph states:
    - the six accepted keys, with any other key an error, compared exactly, drafts included
    - one `Frontmatter` error per page listing unknown keys (sorted byte-wise), then missing required keys (in field order), plus the accepted keys when any key is unknown
    - precedence: an unterminated block, invalid JSON or a non-object keeps its current error; otherwise key errors come first, then wrong types (serde), date, tags and file name
  - The `frontmatter.rs` module-map entry in `CLAUDE.md` mentions the key check.
  - RISK-4 is marked `done` in both places in `backlog.md`.
  - `overview.md` no longer says unknown keys are silently ignored (row at line 25 and the risk row at line 110).

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-2 (drift guard; existing tests) |
| AC-1.2 | T-1 |
| AC-1.3 | existing tests (no code change in the draft filter); T-3 |
| AC-2.1 | replaced by REQ-3 (T-2, T-4); no pin, by design |
| AC-3.1 | T-2, T-4 |
| AC-3.2 | T-2 |
| AC-3.3 | T-3, T-4 |
| AC-3.4 | T-2 |
| AC-3.5 | T-4 |
| AC-4.1 | T-4 |
| AC-4.2 | T-2, T-3, T-4 |
| AC-4.3 | T-2, T-4 |
| AC-4.4 | T-2 |
| AC-4.5 | T-2 |
| AC-4.6 | T-2 |
| AC-5.1 | T-2 (with the T-1 pins) |
| AC-5.2 | T-2 |
| AC-5.3 | T-3 |
| AC-5.4 | T-1, T-2, T-3 |
| AC-6.1 | T-4 |
| AC-7.1 | T-5 |
| AC-7.2 | T-5 |
| AC-7.3 | T-6 |

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260925-182336-risk-4-unknown-frontmatter-keys | Initial design. The frontmatter JSON is parsed first as a generic value and its keys are checked against a fixed list, with one error per page listing every problem in a fixed order; then today's typed parse runs unchanged. Safety-net pins first; 6 tasks. |
