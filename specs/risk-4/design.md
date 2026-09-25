<!-- Published by dev-pipeline run 20260925-182336-risk-4-unknown-frontmatter-keys, stage design v3, approved 2026-09-25T21:25:00Z. -->

# Strict frontmatter keys: Design

Spec ID: `risk-4` · Requirements: `specs/risk-4/requirements.md`

## Overview
`frontmatter::parse` checks the keys before the typed parse runs.
- **Key read.** The JSON block is read once as a `BTreeMap<String, serde::de::IgnoredAny>`. This reads every key string and skips every value.
- **Key check.** If that read succeeds, `check_keys` compares the keys with the six accepted keys and the four required ones. Every unknown and missing key goes into one `Frontmatter` error, in a fixed order.
- **Fall-through.** In three cases today's `serde_json::from_str::<MangoFrontmatter>` runs unchanged, so its error reads exactly as it does now:
  - the block is invalid JSON (the key read fails)
  - the block is valid JSON but not an object (the key read fails)
  - the keys are fine (the key check passes)
- **File path.** The loader already puts the file path in front of the message.
- **Precedence.** `Page::new` checks the date, then the tags, then the slug, and it runs only after `parse` returns. So key errors come before everything else, with no change to `Page::new` or the loader.

`MangoFrontmatter` does **not** get `#[serde(deny_unknown_fields)]`, because the attribute would break AC-5.1 (see Approach 6).

## Affected components
| Component / file | Change |
|---|---|
| `src/content/frontmatter.rs` | Add the `ACCEPTED_KEYS` and `REQUIRED_KEYS` constants, `check_keys(&BTreeMap<String, IgnoredAny>)` and a private `parse_json` helper. `parse_json` reads the keys, calls `check_keys`, then runs the existing typed parse. `MangoFrontmatter` is unchanged: it has no `deny_unknown_fields`, and a comment says why. New unit tests, including the drift guard `accepted_keys_match_the_struct_fields`. |
| `src/content/loader.rs` | No production change. New unit tests: a baseline pin for a missing key, precedence (AC-5.3) and drafts (AC-3.3). |
| `tests/plan.rs` | Add an `"unknown key"` case to `draft_with_bad_frontmatter_date_or_tag_fails_planning`. |
| `tests/build.rs` | New E2E tests: stderr, exit status and an unchanged output snapshot, plus the draft case. |
| `README.md` | A paragraph under the frontmatter field table. |
| `CHANGELOG.md` | A `### Changed` entry under `## [Unreleased]`, marked breaking. |
| `CLAUDE.md` | The frontmatter rules paragraph (including the `BTreeMap<String, IgnoredAny>` key read) and the `frontmatter.rs` module-map entry. |
| `specs/_system/backlog.md` | RISK-4 status changes from `open` to `done`, in the index row and the section header. |
| `specs/_system/overview.md` | The `frontmatter.rs` row of the module table and the `frontmatter.rs` risk row no longer say "unknown keys silently ignored". Stale line and test counts are removed from the module-table row. |

## Approach
Evidence below cites the locked crate versions (`Cargo.lock:689-720`: serde, serde_core and serde_derive 1.0.228, serde_json 1.0.149). Paths are under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

1. **Read the keys, check them, then run the typed parse.** `parse` calls `parse_json(&json)` (`src/content/frontmatter.rs:75-81`):
   ```rust
   fn parse_json(json: &str) -> Result<MangoFrontmatter, MangoError> {
       if let Ok(map) = serde_json::from_str::<BTreeMap<String, IgnoredAny>>(json) {
           check_keys(&map)?;
       }
       serde_json::from_str::<MangoFrontmatter>(json)
           .map_err(|e| MangoError::Frontmatter(e.to_string()))
   }
   ```
   - **Invalid JSON or not an object:** the key read fails and today's typed parse runs, returning today's exact error with line and column (AC-5.1).
     - A map accepts only `{` (`serde_json-1.0.149/src/de.rs:1799-1811`, `deserialize_map`).
     - A struct also accepts `[` (`de.rs:1836-1859`, `deserialize_struct`).
     - So an array block still goes to the typed parse, as before.
   - **Keys are fine:** the typed parse reports wrong types and duplicate known keys exactly as before. `Page::new` then checks the date, tags and slug in its current order (AC-5.4).
   - **Key errors:** `check_keys` returns first, so wrong types, dates, tags and file names are never reached (AC-5.2, AC-5.3). This doesn't depend on document order, because the whole map is read before anything is reported.

2. **Why `IgnoredAny` rather than `serde_json::Value` (parity with the typed parse).** The key read has to succeed on every object the typed parse accepts. Otherwise an unknown key could fall through to the typed parse, which ignores it silently (AC-3.1).
   - **Values are skipped by the same routine.**
     - The derived struct deserializer skips an unknown field's value with `next_value::<IgnoredAny>` (`serde_derive-1.0.228/src/de/struct_.rs:287-291`).
     - `IgnoredAny::deserialize` calls `deserialize_ignored_any` (`serde_core-1.0.228/src/de/ignored_any.rs:236`).
     - serde_json implements that as `ignore_value` (`de.rs:1910-1916`).
     - The key read skips every value with this same `ignore_value`.
   - **Keys are read by the same routine.** `MapKey` forwards both `string` (the map's `String` key) and `identifier` (the struct's field name) to `deserialize_any`, which calls `parse_str` (`de.rs:2205-2215`, `de.rs:2322-2325`). So both reads accept or reject a key string in the same way.
   - **The typed parse checks a known field's value more strictly, never less.** So any object the typed parse accepts, the key read also accepts.
   - **A `Value` read would not be equivalent.** It validates values that `ignore_value` only skims:
     - **Number range:** `ignore_integer`, `ignore_decimal` and `ignore_exponent` check syntax only (`de.rs:1217-1282`). The `Value` read fails `1e400` with `NumberOutOfRange` (`de.rs:859-868`).
     - **Lone surrogates:** `ignore_escape` consumes `\uXXXX` without validating the code point (`read.rs:1025-1048`, comment at `:1034-1038`). The `Value` read validates it and rejects `"\udc00"` (`read.rs:709-716` passes `validate = true`, and `read.rs:911-913` returns the error).
     - **Nesting depth:** `ignore_value` is iterative, with its own stack in `scratch` and no recursion check (`de.rs:1102-1215`). The `Value` read stops at depth 128 (`de.rs:63`, `check_recursion!` at `de.rs:1372-1387`).
     - With a `Value` read, a page with an unknown key carrying any of these values built silently. That was review v1 finding 1.
     - The regression test is `unknown_key_is_reported_whatever_its_value` (`src/content/frontmatter.rs:343-356`), which covers all three values.

3. **Every key counts once.** serde's `BTreeMap` visitor does `values.insert(key, value)` for each entry, so a repeated key overwrites the earlier one without an error (`serde_core-1.0.228/src/de/impls.rs:1536-1547`). The typed parse, by contrast, raises `duplicate_field` (`struct_.rs:269`).
   - So each key is named at most once, and a known key repeated next to an unknown key is reported as key errors only (AC-4.6). This is pinned by `a_repeated_key_is_named_once`.
   - A key counts as present whatever its value, including `null` (REQ-4 intro), because `IgnoredAny` accepts any value. This is pinned by `null_valued_key_counts_as_present`.
   - `BTreeMap` comes from `std`, so none of this depends on serde_json's `preserve_order` feature.

4. **Fixed order.** Unknown keys are collected into a `Vec<&str>` and sorted with `sort_unstable()`, which is byte-wise for `str` (AC-4.4).
   - `BTreeMap<String, _>` already iterates in that order. The explicit sort keeps the rule visible where it matters and costs nothing.
   - Missing keys are listed by iterating `REQUIRED_KEYS`: `title`, `author`, `description`, `draft`.

5. **Exact comparison and drafts.**
   - `ACCEPTED_KEYS.contains(&key)` compares strings exactly, so case and whitespace count (AC-3.2).
   - The check runs in `parse`, before `draft` is read, so drafts are checked too (AC-3.3). The loader filters out drafts only after every page has loaded.

6. **No `deny_unknown_fields`; a separate drift guard.**
   - **Why not the attribute.** With it, the derived field visitor returns `unknown_field` as soon as it reads an unknown key (`serde_derive-1.0.228/src/de/identifier.rs:267-268`, chosen at `struct_.rs:285`). That happens in document order. On invalid JSON, an unknown key read before the syntax error would be reported instead of today's syntax error, which breaks AC-5.1.
     - The proof is `invalid_json_is_the_typed_parse_error` (`src/content/frontmatter.rs:222-228`). Its input `{"titel": "t", "tag": []` must give the typed parser's own EOF error, with no `unknown` in it.
     - A comment on `MangoFrontmatter` (`frontmatter.rs:19-22`) records this.
   - **The drift guard.** The unit test `accepted_keys_match_the_struct_fields` (`frontmatter.rs:376-420`) keeps `ACCEPTED_KEYS`, `REQUIRED_KEYS` and the struct in step:
     - (a) It runs `MangoFrontmatter::deserialize` against a small test-only `Deserializer`. That deserializer's `deserialize_struct` returns the derived `fields` list (in declaration order) as the error text, and the list must equal `ACCEPTED_KEYS.join(",")`.
     - (b) Starting from an object with all six keys, it removes each key in turn. Parsing must fail exactly when the removed key is in `REQUIRED_KEYS`.
     - Adding, removing or renaming a field in one place but not the others fails this test.
     - The guard is test code only, so no production code sits unused.

## Interfaces and data
- New private items in `src/content/frontmatter.rs`:
  - `const ACCEPTED_KEYS: [&str; 6] = ["title", "author", "description", "date", "tags", "draft"];` (in the field table's order)
  - `const REQUIRED_KEYS: [&str; 4] = ["title", "author", "description", "draft"];`
  - `fn parse_json(json: &str) -> Result<MangoFrontmatter, MangoError>`
  - `fn check_keys(map: &BTreeMap<String, IgnoredAny>) -> Result<(), MangoError>`
  - New imports: `std::collections::BTreeMap` and `serde::de::IgnoredAny`. Both come from existing dependencies.
- `pub fn parse` keeps its signature. `MangoFrontmatter` keeps its fields and attributes: `#[derive(Deserialize, Debug)]`, with no `deny_unknown_fields`.
- **Error message.** It is a `MangoError::Frontmatter` payload. The loader adds `<path>: ` in front, and `Display` adds `Mango Frontmatter Error: `.
  - The payload starts with `invalid frontmatter keys: `.
  - Each problem is `unknown '<key>'` or `missing '<key>'`, in the AC-4.4 order, joined with `, `.
  - If at least one key is unknown, the payload ends with `; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`. If only keys are missing, the suffix is left out.
  - Example (AC-4.2): `invalid frontmatter keys: unknown 'tag', unknown 'titel', missing 'title'; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`
  - Example (AC-4.5, `{}`): `invalid frontmatter keys: missing 'title', missing 'author', missing 'description', missing 'draft'`
  - Keys are printed raw inside single quotes, the same way dates and tags are.
- No schema, CLI, config or template changes.

## Alternatives considered
| Option | Why not chosen |
|---|---|
| Only add `#[serde(deny_unknown_fields)]` | serde stops at the first problem, in document order. It would report only one key, would report a wrong-typed value that comes before an unknown key (breaks AC-5.2), and would use serde's backtick wording. It fails AC-4.2 to AC-4.5. |
| Add `deny_unknown_fields` as well, purely as a drift guard (design v1) | Design v1 claimed the attribute's error was unreachable, but it isn't. On invalid JSON the typed parse reports an unknown key read before the syntax error, which breaks AC-5.1 (Approach 6). `accepted_keys_match_the_struct_fields` guards against drift without changing runtime behavior. |
| Read the keys through `serde_json::Value` (design v1) | `Value` validates values that the typed parse skips: out-of-range numbers, lone surrogates and nesting deeper than 128. An unknown key holding such a value got past the key check, and the typed parse then dropped it silently. That breaks AC-3.1 (Approach 2, review v1 finding 1). |
| Parse only into `Value`, then `serde_json::from_value::<MangoFrontmatter>` | `from_value` errors lose line and column. A `Value` map also merges duplicate known keys, which today are a `duplicate field` error. Both change behavior that AC-5.4 keeps. It would also inherit the `Value` gaps above. |
| Hand-written `Deserialize`, or a `#[serde(flatten)] extra: Map` field | `flatten` changes how serde reports errors, and it buffers values through serde's own content type. A custom visitor would still fail on a wrong type before it had seen every key (AC-5.2). It's more code for a worse result. |
| Check keys in `loader.rs` or `Page::new` | By then the typed parse has already failed on wrong types or duplicates, so key errors couldn't take precedence. `frontmatter.rs` is where the JSON is parsed. |
| Enable serde_json's `preserve_order` to report keys in document order | The spec fixes a sorted order (AC-4.4), and enabling a feature needs the maintainer's approval. |

## Risks
- **Two parses of every frontmatter block.** The blocks are small, and the key read allocates only the key strings. The cost is negligible next to rendering.
- **Parser parity.** Approach 2 shows that the key read and the typed parse read keys with the same `parse_str` and skip values with the same `ignore_value`. So every object the typed parse accepts gets its keys checked.
  - The converse doesn't hold, and doesn't need to. The key read accepts some objects the typed parse rejects: wrong types, duplicate known keys, missing keys and odd values on known keys. These either become key errors (AC-5.2) or, when there are no key errors, go to the typed parse unchanged (AC-5.4).
  - The guarantee relies on serde_json's `deserialize_ignored_any` staying the routine the derived struct uses for unknown fields. A future serde_json could change that. If it did, `unknown_key_is_reported_whatever_its_value` would fail, so a dependency bump can't silently reopen the gap.
- **Where "not valid JSON" ends (AC-5.1).** The key read enforces JSON structure: commas, colons, brackets, key strings, literals and number syntax. Some values are structurally valid but serde_json refuses to *convert* them: a known key holding `1e400` or `"\udc00"`, or nested deeper than 128. The design treats these as value problems, not as invalid JSON. When key errors are present, the key error wins, in line with AC-5.2. When there are none, the typed parse reports the problem exactly as before (AC-5.4).
  - RFC 8259 allows all three forms syntactically (§6 for numbers, §7 for `\u` escapes, §9 for implementation limits on depth), so this reading matches the spec's intent.
  - Review v2 marks AC-5.1 as satisfied (its AC-5.1 row) and raises no finding on this point.
- **A quirk that stays.** serde's derived struct deserializer also accepts a JSON *array* in field order (`visit_seq`, `struct_.rs:72-95`), such as `["t","a","d",null,null,false]`, and such a page builds today.
  - The key read rejects arrays (it reads a map only, `de.rs:1799-1811`), so they go to today's typed parse (AC-5.1 "as it does today").
  - An array has no keys, so REQ-3 doesn't apply. This is out of scope. A possible follow-up backlog item is "frontmatter must be an object".
- **Breaking change.** A site that uses extra keys stops building. The maintainer accepted this, and `CHANGELOG.md` records it (AC-7.2).
  - The example site uses only the six keys. I checked the frontmatter of every `example/site/**/*.md` file.
  - The JSON block in `blog/how-i-write.md` lines 19-27 is in a code fence in the body, not in the frontmatter.
- **Keys with control characters or quotes** are printed raw, which matches how tags and dates are reported today. This is acceptable.

## Test strategy
- Suite command: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`
- Tests carry spec-scoped IDs: `// AC-risk-4.<REQ>.<n>` (AC-3.1 → `AC-risk-4.3.1`).
- **Safety net:** before the change, `src/` and `tests/` had no test for `missing field`, `invalid type`, `duplicate field` or `expected struct`.
  - Already covered:
    - AC-1.1: `parses_valid_frontmatter_and_returns_body`, `optional_fields_may_be_omitted`
    - AC-1.2, malformed JSON: `malformed_json_is_an_error`, `subdirectory_malformed_frontmatter_is_an_error_naming_the_file`
    - AC-1.3: `malformed_draft_is_still_an_error`, `draft_with_bad_frontmatter_date_or_tag_fails_planning`
  - T-1 pinned the uncovered parts of AC-1.2 before any production change:
    - `wrong_typed_value_is_the_typed_parse_error`
    - `duplicate_known_key_is_the_typed_parse_error`
    - `non_object_json_is_the_typed_parse_error`
    - `missing_required_key_is_a_frontmatter_error_naming_the_file`
  - AC-2.1 is the behavior being replaced, so it gets no pin. The T-2 tests fail against it.
- **New and changed behavior:**
  - `frontmatter.rs` unit tests (T-2):
    - AC-3.1: `unknown_key_is_an_error_listing_the_accepted_keys` (also AC-4.2 and AC-4.3), `unknown_key_is_reported_whatever_its_value`
    - AC-3.2: `keys_are_compared_exactly`
    - AC-3.4: `only_accepted_keys_parse_as_before`
    - AC-4.2 and AC-4.4: `every_unknown_and_missing_key_is_listed_in_a_fixed_order`, `unknown_keys_are_sorted_byte_wise_and_missing_keys_in_field_order`
    - AC-4.5: `empty_object_reports_every_missing_key_without_the_accepted_list`
    - AC-4.6: `a_repeated_key_is_named_once`
    - REQ-4 intro (a present key counts whatever its value): `null_valued_key_counts_as_present`
    - AC-5.1: `invalid_json_is_the_typed_parse_error`, together with the T-1 pin `non_object_json_is_the_typed_parse_error` and the existing `unterminated_block_is_an_error`
    - AC-5.2: `key_errors_win_over_wrong_typed_values_in_any_order`
    - AC-1.1 drift guard: `accepted_keys_match_the_struct_fields`
  - `loader.rs` unit tests (T-3): `key_errors_win_over_date_tag_and_file_name_errors` (AC-5.3, and AC-4.2 with the file path) and `unknown_key_in_draft_is_still_an_error` (AC-3.3). The existing `frontmatter_error_wins_over_invalid_file_name` still covers AC-5.4.
  - `tests/plan.rs` (T-3): the `"unknown key"` case covers AC-3.3 through the public API.
  - `tests/build.rs` E2E (T-4):
    - `build_fails_on_unknown_frontmatter_keys_keeping_output`: AC-3.1, AC-4.1, AC-4.2, AC-4.3 and AC-6.1 (snapshot)
    - `build_fails_on_unknown_frontmatter_key_in_draft`: AC-3.3
  - AC-3.5: `build_generates_site_from_fixture`, `fixture_build_is_deterministic` and the `fixture_*` tests stay green, with no change to the fixture or the manifest.

## Tasks
### T-1 Pin current frontmatter error behavior
- Kind: safety-net
- Satisfies: AC-1.2, AC-5.1, AC-5.4
- Files: `src/content/frontmatter.rs`, `src/content/loader.rs`
- Done when:
  - These tests exist and passed on the **unchanged** production code:
    - (a) `frontmatter.rs`: `"title": 5` (all other keys valid) gives `Err(Frontmatter(msg))`. `msg` equals the typed parser's error text and contains `invalid type`.
    - (b) `frontmatter.rs`: a duplicate `"title"` (keys otherwise valid) gives a `Frontmatter` error that equals the typed parser's text and contains `duplicate field`.
    - (c) `frontmatter.rs`: the blocks `42` and `"text"` each give a `Frontmatter` error equal to the error text of `serde_json::from_str::<MangoFrontmatter>(block)`.
    - (d) `loader.rs`: a page without `title` fails with a `Frontmatter` error whose message starts with `Mango Frontmatter Error: <file path>: `.
  - None of them asserts the wording of the missing-key message, because T-2 changes that wording.

### T-2 Key check in `frontmatter::parse`
- Kind: implementation
- Satisfies: AC-1.1, AC-3.1, AC-3.2, AC-3.4, AC-4.2, AC-4.3, AC-4.4, AC-4.5, AC-4.6, AC-5.1, AC-5.2, AC-5.4
- Files: `src/content/frontmatter.rs`
- Done when:
  - `ACCEPTED_KEYS`, `REQUIRED_KEYS`, `parse_json` (key read as `BTreeMap<String, IgnoredAny>`) and `check_keys(&BTreeMap<String, IgnoredAny>)` are in place as described under Interfaces and data.
  - `MangoFrontmatter` has **no** `#[serde(deny_unknown_fields)]`, and a comment explains why (AC-5.1).
  - These new unit tests pass:
    - an unknown `tag` → the exact message, including the accepted-keys suffix
    - `Title`, `TAGS` and `"title "` → each reported as unknown
    - `titel` + `tag` with no `title` → the exact AC-4.2 message, identical for two different key orders
    - `{"b":1,"a":1,"Z":1}` → `unknown 'Z', unknown 'a', unknown 'b'`, then all four missing keys in field order
    - `{}` → the exact AC-4.5 message, without the suffix
    - an unknown key repeated, and a known key repeated alongside an unknown key → each key named once, and no `duplicate field`
    - `"title": null` plus an unknown key → `title` is not reported missing
    - `"title": 5` or `"draft": "no"`, both before and after an unknown key → the key-error message, and no `invalid type`
    - an unknown key whose value is `1e400`, `"\udc00"` or 200 nested arrays → `unknown '<key>'`
    - invalid JSON with an unknown key before the syntax error (`{"titel": "t", "tag": []`) → the typed parser's own error, with no `unknown`
    - the drift guard `accepted_keys_match_the_struct_fields`:
      - the struct's `fields` list equals `ACCEPTED_KEYS`
      - removing a key fails the typed parse exactly when the key is in `REQUIRED_KEYS`
  - All existing `frontmatter.rs` tests (including the CRLF and BOM tests) and the T-1 pins still pass.

### T-3 Loader and in-process precedence and draft tests
- Kind: test
- Satisfies: AC-1.3, AC-3.3, AC-4.2, AC-5.3, AC-5.4
- Files: `src/content/loader.rs`, `tests/plan.rs`
- Done when:
  - `key_errors_win_over_date_tag_and_file_name_errors` passes. In this test, `my posts/x.md` has `"date": "2026-02-30"`, `"tags": ["Rust"]` and an extra `"tag": []`. The load fails with a `Frontmatter` error that:
    - contains the file path and `unknown 'tag'`
    - does not contain `invalid date`, `'Rust'` or `invalid file name`
  - `unknown_key_in_draft_is_still_an_error` passes: a draft page with an unknown key fails the load.
  - `draft_with_bad_frontmatter_date_or_tag_fails_planning` has an `"unknown key"` case: a raw draft page with an extra `"dates"` key, asserting `Frontmatter`, the path and `unknown 'dates'`.
  - `frontmatter_error_wins_over_invalid_file_name` still passes unchanged.

### T-4 End-to-end: unknown keys fail the build safely
- Kind: test
- Satisfies: AC-3.1, AC-3.3, AC-3.5, AC-4.1, AC-4.2, AC-4.3, AC-6.1
- Files: `tests/build.rs`
- Done when:
  - `build_fails_on_unknown_frontmatter_keys_keeping_output` passes:
    - It builds a temp site successfully, writes `marker.txt` into `dist/` and takes a `snapshot`.
    - It adds `posts/bad_keys.md`, with the keys `titel` and `tag` and no `title`, then rebuilds with `build_temp_site`.
    - The rebuild exits with status 1.
    - stderr contains `bad_keys.md` and the exact text `unknown 'tag', unknown 'titel', missing 'title'; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'`.
    - The snapshot is unchanged.
  - `build_fails_on_unknown_frontmatter_key_in_draft` passes: a draft page (`"draft": true`) with an unknown key also exits with status 1, and stderr names the file.
  - All `fixture_*` tests and `build_generates_site_from_fixture` still pass with no fixture change (AC-3.5).

### T-5 User-facing docs and changelog
- Kind: docs
- Satisfies: AC-7.1, AC-7.2
- Files: `README.md`, `CHANGELOG.md`
- Done when:
  - In `README.md`, directly under the frontmatter field table, the text says that:
    - any other key is an error, drafts included, which catches typos such as `tag` for `tags`
    - a page with bad keys gets one error listing every unknown and every missing key, plus the accepted keys
  - `CHANGELOG.md` has a `### Changed` entry for site authors under `## [Unreleased]`. It says that:
    - unknown frontmatter keys now fail the build, drafts included, with one error per page listing every unknown and missing key
    - this is a **breaking** change
    - a site that used extra keys must remove them

### T-6 Developer docs and tracking
- Kind: docs
- Satisfies: AC-7.3
- Files: `CLAUDE.md`, `specs/_system/backlog.md`, `specs/_system/overview.md`
- Done when:
  - The "Frontmatter is JSON" paragraph in `CLAUDE.md` states:
    - the six accepted keys, and that any other key is an error, compared exactly, drafts included
    - one `Frontmatter` error per page, listing unknown keys (sorted byte-wise), then missing required keys (in field order), plus the accepted keys when any key is unknown
    - that the key read is `BTreeMap<String, IgnoredAny>`, and why a `Value` read would let some unknown keys through
    - the precedence: an unterminated block, invalid JSON or a non-object keeps its current error; otherwise key errors come first, then wrong types (serde), the date, the tags and the file name
  - The `frontmatter.rs` module-map entry in `CLAUDE.md` mentions the key check.
  - RISK-4 is marked `done` in both places in `backlog.md`.
  - `overview.md` no longer says that unknown keys are silently ignored, in either the `frontmatter.rs` module-table row or the `frontmatter.rs` risk row. The module-table row has no stale line or test counts.

## Coverage
| AC | Tasks |
|---|---|
| AC-1.1 | T-2 (drift guard; existing tests) |
| AC-1.2 | T-1 |
| AC-1.3 | T-3; existing tests (the draft filter doesn't change) |
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
| AC-5.1 | T-1, T-2 |
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
| 2026-09-25 | 20260925-182336-risk-4-unknown-frontmatter-keys | Initial design. The frontmatter JSON is parsed first as a generic value and its keys are checked against a fixed list, with one error per page listing every problem in a fixed order. Then today's typed parse runs unchanged. Safety-net pins come first; 6 tasks. |
| 2026-09-25 | 20260925-182336-risk-4-unknown-frontmatter-keys | Amended to match the implementation (review v1 finding 2, review v2 finding 1):<br>- The key read is `BTreeMap<String, IgnoredAny>`, not `serde_json::Value`, and `check_keys` takes that map. A `Value` read let through unknown keys holding `1e400`, a lone surrogate or nesting past 128.<br>- There is no `deny_unknown_fields`, because it breaks AC-5.1.<br>- The drift guard is `accepted_keys_match_the_struct_fields`, which reads the struct's field list.<br>- The parity risk is restated with source-line evidence: values are skipped with the same `ignore_value` as in the typed parse.<br>No requirement or code change. |
| 2026-09-25 | 20260925-182336-risk-4-unknown-frontmatter-keys | Proofread at the maintainer's request:<br>- The Overview's three fall-through cases are now listed as three items.<br>- Corrected line ranges: `de.rs:1217-1282`, `struct_.rs:287-291` and `struct_.rs:72-95`. The code block in `how-i-write.md` is at lines 19-27.<br>- The review v2 citation no longer claims an explicit acceptance.<br>- The Satisfies lists now match the Coverage table: T-1 adds AC-5.1, T-3 adds AC-1.3 and T-4 adds AC-3.5.<br>- The test strategy separates new, pinned and existing tests for AC-5.1 and files `null_valued_key_counts_as_present` under the REQ-4 intro.<br>No requirement or code change. |
