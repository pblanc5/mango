<!-- Published by dev-pipeline run 20260927-063534-risk-9-non-object-frontmatter, stage design v1, approved 2026-09-27T07:08:52Z. -->

# Frontmatter must be a JSON object: Design

Spec ID: `risk-9` · Requirements: `specs/risk-9/requirements.md`

## Overview
`frontmatter::parse_json` gets a new first step. It reads the block as a top-level `serde::de::IgnoredAny`. If that works, the block is valid JSON. The kind of its top-level value is then read from the first byte after JSON whitespace. Anything other than `{` fails with `frontmatter must be a JSON object, found <kind>`. The loader's existing `with_path` wrapper adds the file path in front. Invalid JSON fails the `IgnoredAny` read and falls through to today's path unchanged. The same goes for any object, which still gets the RISK-4 key check and then the typed parse. No dependency, type or module is added.

## Affected components
| Component / file | Change |
|---|---|
| `src/content/frontmatter.rs` | New private `non_object_kind(json) -> Option<&'static str>`. `parse_json` runs the object check first. Its doc comment is updated. Unit tests: the old `non_object_json_is_the_typed_parse_error` is replaced, and new tests cover REQ-2, REQ-3 and AC-2.6. |
| `src/content/loader.rs` | Tests only: the file-path prefix, drafts, precedence over a bad file name, and hidden files. No production change, because `with_path` (loader.rs:68-73) already prefixes `Frontmatter` errors. |
| `tests/build.rs` | New temp-site E2E test: array frontmatter gives exit 1, the stderr message, and output kept (`snapshot`). |
| `README.md` | Content section: the block must hold a JSON object, and any other JSON value, an array included, fails the build. |
| `CHANGELOG.md` | `**Breaking:**` entry under `## [Unreleased]` / `### Changed`. |
| `CLAUDE.md` | Frontmatter paragraph: the precedence includes the non-object rule and no longer says a non-object "keeps its current error". Module-map line for `frontmatter.rs` updated. |
| `specs/_system/overview.md` | The `frontmatter.rs` risk row no longer lists RISK-9 as open. New Changelog row. |
| `specs/risk-4/requirements.md` | Note under AC-5.1 that its "valid JSON but not an object" part is superseded by `specs/risk-9/requirements.md`, with the wording kept. New Changelog row. |
| `specs/_system/backlog.md` | FEAT-4 is deferred, not dropped (status stays `open`), and its "Done when" loses "RISK-9 is closed as moot". The RISK-9 text is rewritten and the item marked `done` (Index row and section). The Recommended-order sentence is replaced. |

## Approach
New `parse_json`, in this order:

1. `serde_json::from_str::<IgnoredAny>(json)`. If it fails, skip to step 3, so the typed parse reports exactly today's error (AC-2.6). This covers every invalid-JSON case, including `["t",`, trailing characters and an empty block.
2. If it succeeds, `non_object_kind(json)` looks at the first byte after the JSON whitespace `' ' '\n' '\t' '\r'`:
   - `[` gives `an array`
   - `"` gives `a string`
   - `t` or `f` gives `a boolean`
   - `n` gives `null`
   - `-` or `0`–`9` gives `a number`
   - anything else (only `{` can occur once the text is valid) gives `None`

   For `Some(kind)`, return `MangoError::Frontmatter(format!("frontmatter must be a JSON object, found {kind}"))`.
3. Unchanged: the `BTreeMap<String, IgnoredAny>` key check, then the typed parse of `MangoFrontmatter`.

Why this works:
- **The first byte is the kind.** `serde_json::from_str` is `from_trait` (de.rs:2699-2704). It deserializes one value and then calls `end()`, which allows only trailing whitespace (de.rs:146-151, 2497-2508). So a text that passes step 1 is exactly one JSON value, optionally surrounded by whitespace. The value's kind is set by its first byte, per the dispatch in `ignore_value` (de.rs:1114-1149), which accepts only `n t f - 0-9 " [ {` there. The whitespace set is the one in `parse_whitespace` (de.rs:255-266).
- **`IgnoredAny` accepts everything in AC-2.3.** `IgnoredAny::deserialize` calls `deserialize_ignored_any` (serde_core-1.0.228 `de/ignored_any.rs:236`). In serde_json that is `ignore_value()` then `visit_unit` (de.rs:1910-1916). `ignore_value` is iterative, with an explicit `scratch` stack of frames (de.rs:1102-1215), and never touches `remaining_depth`, so 200 nested arrays pass. Only `deserialize_any` and friends use the recursion-limit macro (de.rs:1366-1384). `ignore_integer`, `ignore_decimal` and `ignore_exponent` check syntax only and never build an `f64` (de.rs:1217-1282), so `1e400` passes. `ignore_str`/`ignore_escape` read a `\u` escape with `decode_hex_escape` and do not check whether it is a valid code point (read.rs:598-618, 1025-1048), so `"\udc00"` passes. The existing test `unknown_key_is_reported_whatever_its_value` already shows the same `IgnoredAny` path accepting all three as object values.
- **Objects are untouched (AC-2.5).** For a `{` text, step 2 returns `None` and steps 3 onward are byte-for-byte today's code.
- **Precedence (AC-4.1, AC-4.2).** The unterminated-block check in `parse` still runs before `parse_json`. Step 2 runs before the key check and the typed parse. `Page::new` (date, tags, then file name) only runs after `parse` succeeds.
- **Hidden files (AC-4.3).** `loader::traverse` skips hidden names before reading them (loader.rs:33-35), so nothing changes there.
- **Message (AC-3.2 to AC-3.4).** The message depends only on `kind`. `with_path` adds `<file>: `, and `Display` adds `Mango Frontmatter Error: `, as the existing test `missing_required_key_is_a_frontmatter_error_naming_the_file` shows. The final text is `Mango Frontmatter Error: <file>: frontmatter must be a JSON object, found an array`. This uses the wording proposed in Open question 1 and applies it to scalars too (Open question 3).

## Interfaces and data
No public or crate-level interface changes. New private item in `src/content/frontmatter.rs`:

```rust
/// The kind of a valid JSON text's top-level value when it is not an
/// object, read from its first non-whitespace byte; `None` for an object.
fn non_object_kind(json: &str) -> Option<&'static str>
```

New error text (a `MangoError::Frontmatter` payload, before the loader's path prefix):
`frontmatter must be a JSON object, found {an array | a string | a number | a boolean | null}`.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Parse into `serde_json::Value` and match on the variant | `Value` goes through the recursion limit (de.rs:1366-1384), range-checks numbers (`NumberOutOfRange`, e.g. de.rs:378) and rejects lone surrogates (read.rs:913/959). `[1e400]`, `"\udc00"` and 200 nested arrays would fail as invalid JSON with the library's text, which violates AC-2.3. |
| A custom visitor through `deserialize_any` that reports which `visit_*` was called | Scalars are fully parsed before the visitor sees them, so the same range, surrogate and depth errors apply to a bare `1e400` or `"\udc00"` (AC-2.3). More code for no gain. |
| Look at the first byte alone, without validating the JSON | `["t",` would report "must be an object" instead of today's syntax error, which violates AC-2.6. |
| A hand-written `Deserialize` for `MangoFrontmatter` that accepts only maps | It replaces the derive, so wrong-typed and duplicate-key messages could drift (AC-2.5), and `accepted_keys_match_the_struct_fields` would need rework. Much larger for the same result. |
| A new `MangoError` variant | `Frontmatter` is the category the constitution tells us to use, and the loader's `with_path` already handles it. |

## Risks
- **An extra parse per page.** The frontmatter is parsed with `IgnoredAny` once more. The block is small and `ignore_value` does not allocate values, so the cost is negligible.
- **The wording of a scalar's error changes** (AC-1.3 becomes AC-2.1). The build still fails as before. The spec's Open question 3 accepts this, and the changelog entry covers it.
- **Drift between step 1 and the typed parse.** If `IgnoredAny` accepted a non-object text that the typed parse rejects with a better error, the author would now see the new message instead. For non-objects that is the intended result. For objects step 2 returns `None`, so nothing changes.

**Unverified assumptions:** None. Every claim about serde and serde_json above cites the locked sources (`serde_json-1.0.149`, `serde_core-1.0.228` under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`), or the existing test `unknown_key_is_reported_whatever_its_value`. T-2's AC-2.3 cases re-prove it at the top level.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- Safety net:
  - The baseline criteria AC-1.1 to AC-1.3 are replaced by REQ-2, so they are not pinned. T-2's tests fail before the change, which is the proof that behavior changed.
  - AC-2.5 (objects unchanged) is already covered by the RISK-4 tests in `frontmatter.rs` and `loader.rs`. They stay untouched and must stay green.
  - AC-2.6 is covered today only for `{`-shaped invalid JSON and unterminated blocks (`invalid_json_is_the_typed_parse_error`, `malformed_json_is_an_error`, `unterminated_block_is_an_error`, `subdirectory_malformed_frontmatter_is_an_error_naming_the_file`). Invalid JSON starting with a non-object byte is the case the new step could break, so T-1 pins it first.
- New and changed behavior:
  - AC-2.1, AC-2.2, AC-3.2, AC-3.3 and AC-3.4 are covered by T-2 unit tests (every kind, the field-order array, `[]`, wrong length, draft array; exact strings).
  - AC-2.3 is covered by T-2 (`[1e400]`, `["\udc00"]`, 200 nested arrays, bare `1e400` and bare `"\udc00"`).
  - AC-2.4, AC-4.1, AC-4.2 and AC-4.3 are covered by T-3 loader tests.
  - AC-3.1 and AC-5.1 are covered by the T-4 E2E test.
  - AC-5.2 is covered by the existing fixture tests (`build_generates_site_from_fixture`, the `fixture_*` tests, `fixture_build_is_deterministic`). The fixture uses only object frontmatter and is not edited.
  - AC-5.3 needs no change to `Cargo.toml` or `Cargo.lock`, and neither is in any task's Files.

## Tasks
### T-1 Pin today's error for invalid JSON of every shape
- Kind: safety-net
- Satisfies: AC-2.6, AC-6.3
- Files: `src/content/frontmatter.rs`
- Done when:
  - New unit test `invalid_json_keeps_the_typed_parse_error_whatever_it_starts_with` (tagged `// AC-risk-9.2.6, AC-risk-9.6.3`) exists.
  - For each of `["t",`, `[1,]`, `tru`, `"abc`, `-`, `1e`, `42 x`, `[] []`, the empty block `` and a whitespace-only block `   `, it asserts `frontmatter_error(json) == typed_parse_error(json)` and that the message has no `must be a JSON object`.
  - It passes on the unchanged code.

### T-2 Reject non-object frontmatter in `parse_json`
- Kind: implementation
- Satisfies: AC-2.1, AC-2.2, AC-2.3, AC-2.5, AC-2.6, AC-3.2, AC-3.3, AC-3.4, AC-4.1, AC-6.1, AC-6.2, AC-6.3, AC-6.6
- Files: `src/content/frontmatter.rs`
- Done when:
  - `non_object_kind` exists and `parse_json` runs the steps described under Approach. Its doc comment describes the new order: object check, key check, typed parse. The old comment's "a non-object ... goes to the typed parse" is gone.
  - `non_object_json_is_the_typed_parse_error` is deleted along with its `AC-risk-4.5.1` tag. That criterion's invalid-JSON part stays tagged on `invalid_json_is_the_typed_parse_error`.
  - New unit tests, tagged `// AC-risk-9.<req>.<m>`, assert the **exact** string `frontmatter must be a JSON object, found <kind>` for:
    - arrays (tests: `non_object_frontmatter_is_rejected`, `array_frontmatter_is_rejected_whatever_it_holds`):
      - `["t", "a", "d", null, null, false]`
      - `["t", "a", "d", "2026-01-24", ["rust"], false]`
      - the draft `["t", "a", "d", null, null, true]`
      - `[]`
      - `["t"]` (wrong length)
      - `[1, 2, 3, 4, 5, 6, 7]`
      - `[5, "a", "d", null, null, false]` (wrong type)
      - leading whitespace or newlines before `[`
    - `"text"` gives `a string`, `42`, `-1` and `0.5` give `a number`, `true` and `false` give `a boolean`, `null` gives `null`.
    - AC-2.3 (test `non_object_frontmatter_is_rejected_whatever_its_values`):
      - `[1e400]`, `["\udc00"]` and `"[".repeat(200) + "]".repeat(200)` give `an array`
      - bare `1e400` gives `a number`
      - bare `"\udc00"` gives `a string`
  - Each test also asserts the message has none of `MangoFrontmatter`, `invalid type`, `invalid length` or `invalid frontmatter keys` (AC-3.3). The two field-order arrays with different contents give the same text (AC-3.4).
  - All the existing RISK-4 and CRLF tests still pass unchanged.

### T-3 Loader-level tests: path prefix, drafts, precedence, hidden files
- Kind: test
- Satisfies: AC-2.4, AC-3.2, AC-4.1, AC-4.2, AC-4.3, AC-6.1, AC-6.4
- Files: `src/content/loader.rs`
- Done when these tests exist and pass:
  - `non_object_frontmatter_fails_the_load_even_for_drafts` (`// AC-risk-9.2.4, AC-risk-9.3.2`): `posts/draft.md` holds `["t", "a", "d", null, null, true]`, next to a valid page. The load fails with `MangoError::Frontmatter` and the message equals `format!("Mango Frontmatter Error: {}: frontmatter must be a JSON object, found an array", file.display())`, with `file = site.join("posts").join("draft.md")` (one segment per `join`).
  - `non_object_frontmatter_wins_over_invalid_file_name` (`// AC-risk-9.4.2, AC-risk-9.6.4`): `my posts/x.md` (built with `site.join("my posts").join("x.md")`) holds an array whose date and tag would also be invalid (`["t", "a", "d", "2026-02-30", ["Rust"], false]`). The error has `found an array` and none of `invalid file name`, `'Rust'` or `2026-02-30`.
  - `hidden_entries_are_not_checked` gains a `site.join(".array.md")` case with array frontmatter (`// AC-risk-9.4.3` added to its tags), and still loads only `posts/one`.

### T-4 E2E: array frontmatter fails the build and keeps the previous output
- Kind: test
- Satisfies: AC-3.1, AC-3.2, AC-5.1, AC-6.5
- Files: `tests/build.rs`
- Done when there is a new `build_fails_on_array_frontmatter_keeping_output` (`// AC-risk-9.3.1, AC-risk-9.5.1, AC-risk-9.6.5`), written in the style of `build_fails_on_unknown_frontmatter_keys_keeping_output`. It:
  1. builds `posts/one.md`, writes `marker.txt` and takes a `snapshot`
  2. adds `posts/array.md` with `["Array", "tester", "desc", null, null, false]`
  3. asserts exit code `Some(1)`, that stderr contains `site.join("posts").join("array.md").display()` followed by `: frontmatter must be a JSON object, found an array`, and that stderr has no `MangoFrontmatter`
  4. asserts `snapshot(&out) == before`

### T-5 Documentation, specs and backlog
- Kind: docs
- Satisfies: AC-7.1, AC-7.2, AC-7.3, AC-7.4, AC-7.5, AC-7.6, AC-7.7
- Files: `README.md`, `CHANGELOG.md`, `CLAUDE.md`, `specs/_system/overview.md`, `specs/risk-4/requirements.md`, `specs/_system/backlog.md`
- Done when:
  - **`README.md`**, Content section, after the field-table paragraph: the frontmatter block must hold a JSON object, and any other JSON value (an array, a string, a number, `true`/`false`, `null`) fails the build with an error naming the file. Include an example error line in the style of the existing one: `Mango Frontmatter Error: site/posts/one.md: frontmatter must be a JSON object, found an array`.
  - **`CHANGELOG.md`**: a second bullet under `## [Unreleased]` / `### Changed`, starting `**Breaking:**`, written for site authors. It says that frontmatter must be a JSON object, that a page whose frontmatter is an array (values in field order), which used to build, now fails with an error naming the file, and that such a page must be rewritten as an object with named keys.
  - **`CLAUDE.md`**, frontmatter paragraph: the precedence reads "an unterminated block or invalid JSON keeps its current error; valid JSON that is not an object fails with `frontmatter must be a JSON object, found <kind>` (`an array`, `a string`, `a number`, `a boolean`, `null`), drafts included; otherwise key errors come first, …". Describe the mechanism briefly: a top-level `IgnoredAny` parse, then the first byte (`non_object_kind`). The module-map line for `frontmatter.rs` mentions the object check before `check_keys`.
  - **`specs/_system/overview.md`**: the `frontmatter.rs` risk row says non-objects are rejected since RISK-9 (done) and no longer describes the array quirk as open. Add a Changelog row for this run.
  - **`specs/risk-4/requirements.md`**: AC-5.1's wording is kept verbatim, followed by a note that its "or is valid JSON but not an object" part is superseded by `specs/risk-9/requirements.md`. Add a Changelog row dated 2026-09-27 for this run.
  - **`specs/_system/backlog.md`**:
    - FEAT-4: "**Undecided:** …" is replaced by "deferred, not dropped (maintainer, 2026-09-27): JSON stays the frontmatter format for now, to keep options open". Its status stays `open`. "Done when" no longer says RISK-9 is closed as moot.
    - RISK-9: the section no longer says FEAT-4 would make it moot. It records that RISK-9 went ahead because FEAT-4 was deferred, and has a "Resolved by `specs/risk-9/`: …" note (the object check, the message, `build_fails_on_array_frontmatter_keeping_output`, breaking, in `CHANGELOG.md`). The status is `done` in both the section header and the Index row.
    - The Recommended-order sentence about FEAT-4 and RISK-9 is replaced by one that says FEAT-4 is deferred and JSON stays for now, so RISK-9 no longer waits on it.
  - All prose is proofread (the repository is public).

## Coverage
| AC | Tasks |
|---|---|
| AC-2.1 | T-2 |
| AC-2.2 | T-2 |
| AC-2.3 | T-2 |
| AC-2.4 | T-3 |
| AC-2.5 | T-2 (existing RISK-4 tests kept green) |
| AC-2.6 | T-1, T-2 |
| AC-3.1 | T-4 |
| AC-3.2 | T-2, T-3, T-4 |
| AC-3.3 | T-2 |
| AC-3.4 | T-2 |
| AC-4.1 | T-2, T-3 |
| AC-4.2 | T-3 |
| AC-4.3 | T-3 |
| AC-5.1 | T-4 |
| AC-5.2 | T-2 (fixture not edited; existing fixture tests) |
| AC-5.3 | T-2 (no `Cargo.toml` change) |
| AC-6.1 | T-2, T-3 |
| AC-6.2 | T-2 |
| AC-6.3 | T-1, T-2 |
| AC-6.4 | T-3 |
| AC-6.5 | T-4 |
| AC-6.6 | T-2 |
| AC-7.1 | T-5 |
| AC-7.2 | T-5 |
| AC-7.3 | T-5 |
| AC-7.4 | T-5 |
| AC-7.5 | T-5 |
| AC-7.6 | T-5 |
| AC-7.7 | T-5 |

AC-1.1 to AC-1.3 are baseline criteria that REQ-2 replaces. They get no preserving task.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-27 | 20260927-063534-risk-9-non-object-frontmatter | Initial design. `parse_json` validates the block with a top-level `IgnoredAny` parse, then rejects any non-object value by its first byte with `frontmatter must be a JSON object, found <kind>`, ahead of the RISK-4 key check. Invalid JSON and objects are unchanged. 5 tasks: a safety net for invalid-JSON errors, the implementation with unit tests, loader tests, an E2E test, and docs and backlog. |
