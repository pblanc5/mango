<!-- Published by dev-pipeline run 20260927-063534-risk-9-non-object-frontmatter, stage spec v2, approved 2026-09-27T07:04:21Z. -->

# Frontmatter must be a JSON object: Requirements

Spec ID: `risk-9`

## Summary
A page's frontmatter is documented as a JSON object, but the parser also accepts a JSON array whose values are in field order. So a page whose frontmatter is `["t", "a", "d", null, null, false]` builds today, with no keys at all. This is a second, undocumented format that nobody writes on purpose. RISK-4's key check can't see it, because an array has no keys. With this change, any frontmatter whose top-level JSON value is not an object fails the build with one error that names the file and says what was found. That includes arrays, strings, numbers, booleans and `null`. The change is breaking, because it rejects input that builds today, and it ships in 0.3.0.

## Context
- **Who is affected:** site authors whose frontmatter is not a JSON object. In practice that is a page written as an array by mistake, or a block that holds a bare value. Today an array in field order builds silently. Other non-object values already fail, but with the JSON library's wording, which names the internal type `MangoFrontmatter` and doesn't say in the author's terms that an object was expected.
- **Backlog item:** RISK-9 in `specs/_system/backlog.md`. Its proposal is to reject any frontmatter that is not a JSON object, with an error that names the file. The backlog marks it `/spec-feature` because it rejects input that builds today.
- **Maintainer decision (2026-09-27):** FEAT-4 (plain `key: value` frontmatter) is **deferred, not dropped**. JSON stays the frontmatter format for now, to keep options open. RISK-9 is therefore no longer moot and ships in v0.3.0. The backlog records this decision as part of this change (AC-7.6).
- **Related specs:**
  - `specs/risk-4/requirements.md`. Its AC-5.1 says that frontmatter which is "valid JSON but not an object" reports its error "as it does today". This spec replaces that part of AC-5.1 (AC-7.5). The rest of RISK-4 is unchanged: key errors, their message, their order and their precedence for JSON objects. `specs/risk-4/design.md` (Risks, "A quirk that stays") records the array quirk and suggests this follow-up.
  - `specs/arch-3/requirements.md`. Its AC-1.3 says a frontmatter error wins over a file-name error. This spec keeps that rule (AC-4.2).
- **"Strict inputs" convention** (`specs/constitution.md`, Conventions): bad input fails the build with an error that names the file and the value, drafts included.
- **Release:** under **Releasing** in the constitution, rejecting content that builds today is a breaking change to the content rules. Before 1.0 that bumps the minor version. `CHANGELOG.md`'s `## [Unreleased]` section already holds the breaking RISK-10 entry, so the next release is 0.3.0.

## Current behavior

### REQ-1 [baseline] Frontmatter that is valid JSON but not an object
Evidence: `src/content/frontmatter.rs:75-81` (`parse_json`: the key check runs only when the block reads as a map, and everything else goes to the typed parse of `MangoFrontmatter`, whose derived deserializer also accepts a sequence in field order); `src/content/loader.rs:68-90` (`with_path` prefixes the file path to `Frontmatter` errors); `specs/risk-4/design.md:127-129`; test `non_object_json_is_the_typed_parse_error` (covers `42` and `"text"` only); backlog RISK-9.
Status: inferred, confirm at approval. No test pins AC-1.1 or AC-1.2.

- AC-1.1 [baseline] WHEN a page's frontmatter block is a JSON array of exactly six elements whose values are valid, in order, for `title`, `author`, `description`, `date`, `tags` and `draft` THE content loader SHALL accept it as if those were the page's keys, and THE build SHALL succeed. For example, `["t", "a", "d", "2026-01-24", ["rust"], false]` is published with that date and tag, and `["t", "a", "d", null, null, true]` is a draft that is parsed and then left out of the output.
- AC-1.2 [baseline] IF a page's frontmatter block is a JSON array with the wrong number of elements or an element of the wrong type THEN THE content loader SHALL fail the build with a `Frontmatter` error that starts with the page's file path and carries the JSON library's own message (for example an "invalid length" or "invalid type" message). No key errors are reported.
- AC-1.3 [baseline] IF a page's frontmatter block is a JSON string, number, boolean or `null` THEN THE content loader SHALL fail the build with a `Frontmatter` error that starts with the page's file path and carries the JSON library's "invalid type" message, which names the internal type `struct MangoFrontmatter`. No key errors are reported (risk-4 AC-5.1).

## User stories
- As a site author, I want frontmatter that isn't a JSON object to fail the build, so that there is one frontmatter format and not a second, undocumented one.
- As a site author, I want the error to tell me that frontmatter must be a JSON object and what it found instead, so that I can fix the page without reading mango's internals.
- As a site author, I want drafts checked too, so that a draft doesn't start failing only on the day I publish it.

## Requirements

### REQ-2 Only a JSON object is accepted as frontmatter
Replaces AC-1.1 to AC-1.3. "Top-level value" means the single JSON value that the frontmatter block contains.
- AC-2.1 IF a page's frontmatter block is valid JSON whose top-level value is not an object (an array, a string, a number, a boolean or `null`) THEN THE content loader SHALL fail the build with a `Frontmatter` error.
- AC-2.2 THE content loader SHALL apply AC-2.1 to every array, whatever its length and contents. That includes the six-element array in field order that builds today (AC-1.1), the empty array `[]`, and an array of the wrong length or with wrong-typed elements (AC-1.2).
- AC-2.3 THE content loader SHALL apply AC-2.1 whatever the top-level value contains, including values the JSON parser cannot otherwise represent, such as a number out of range (`1e400`), a lone surrogate escape (`"\udc00"`), or nesting deeper than the JSON parser's recursion limit. For example, `[1e400]`, `1e400` and 200 nested arrays each get the AC-2.1 error.
- AC-2.4 IF the non-object frontmatter is on a page that would be a draft (for example an array whose last element is `true`) THEN THE content loader SHALL still fail the build.
- AC-2.5 WHEN a page's frontmatter block is a JSON object THE content loader SHALL handle it exactly as before this change. That covers the RISK-4 key check and its message, wrong-typed values, dates, tags, file names and drafts.
- AC-2.6 IF the frontmatter block is unterminated or is not valid JSON THEN THE content loader SHALL report that error exactly as it does today (risk-4 AC-5.1). For example, `["t",` is invalid JSON and keeps the JSON library's syntax error.

### REQ-3 The error message
- AC-3.1 WHEN the build fails because frontmatter is not a JSON object THE CLI SHALL print the error to stderr and exit with status 1.
- AC-3.2 THE error message SHALL start with the page's file path, SHALL say that frontmatter must be a JSON object, and SHALL name the kind of value it found: an array, a string, a number, a boolean or `null`. Proposed text: `<file>: frontmatter must be a JSON object, found an array` (Open question 1).
- AC-3.3 THE error message SHALL NOT contain the name `MangoFrontmatter`, the JSON library's "invalid type" or "invalid length" text, or any RISK-4 key error.
- AC-3.4 THE error message SHALL depend only on the file path and the kind of the top-level value. Two pages with different arrays report the same text apart from the path.

### REQ-4 Precedence
- AC-4.1 THE content loader SHALL check whether the frontmatter is an object after the block is found to be terminated and valid JSON (AC-2.6), and before every other frontmatter check: RISK-4 key errors, wrong-typed values, dates and tags.
- AC-4.2 IF a page has non-object frontmatter and also an invalid file name THEN THE content loader SHALL report the non-object error (AC-2.1) and SHALL NOT report the file-name problem, in keeping with arch-3 AC-1.3.
- AC-4.3 THE content loader SHALL NOT change how files and folders whose names start with `.` are handled: they are skipped without being read (risk-10), so a hidden page with non-object frontmatter does not fail the build.

### REQ-5 Failure is safe, and nothing else changes
- AC-5.1 IF the build fails because frontmatter is not a JSON object THEN THE build SHALL leave the previous contents of the output folder untouched.
- AC-5.2 WHEN the example site (`example/site`, `example/meta`, `example/mango.json`) is built THE build SHALL succeed, and its output SHALL be byte-identical to the output before this change.
- AC-5.3 THE change SHALL add no dependency.

### REQ-6 Tests
New tests are tagged `// AC-risk-9.<req>.<m>`.
- AC-6.1 THE test suite SHALL include unit tests showing that each non-object kind (an array, a string, a number, a boolean, `null`) gets the AC-2.1 error with the AC-3.2 content. The array cases SHALL include the six-element array in field order that builds today, a draft array, `[]` and an array of the wrong length (AC-2.2, AC-2.4).
- AC-6.2 THE test suite SHALL include unit tests for the values of AC-2.3: a number out of range, a lone surrogate and nesting deeper than the recursion limit, each inside an array. It SHALL also include a test in which at least one of these values is the bare top-level frontmatter, not wrapped in an array (for example, frontmatter that is just `1e400`).
- AC-6.3 THE test suite SHALL include tests showing that invalid JSON and an unterminated block keep today's error (AC-2.6), and that a JSON object is handled as before, key errors included (AC-2.5).
- AC-6.4 THE test suite SHALL include a test showing that non-object frontmatter wins over an invalid file name (AC-4.2).
- AC-6.5 THE test suite SHALL include a temp-site E2E test in `tests/build.rs` in which a page with array frontmatter fails the build with exit status 1, stderr names the file and carries the AC-3.2 message, and a `snapshot` shows the previous output untouched (AC-3.1, AC-5.1).
- AC-6.6 THE test `non_object_json_is_the_typed_parse_error` SHALL be replaced by a test of the new behavior. Its `// AC-risk-4.5.1` tag SHALL either be removed with it or moved to a test that still checks the invalid-JSON part of risk-4 AC-5.1.

### REQ-7 Documentation and backlog
- AC-7.1 THE user guide (`README.md`) SHALL say, in its Content section, that the frontmatter block must hold a JSON object, and that any other JSON value, an array included, fails the build.
- AC-7.2 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]` in the `### Changed` group, as an entry that starts with `**Breaking:**` and is written for site authors. It SHALL say that frontmatter must be a JSON object, that a page whose frontmatter is an array (values in field order), which used to build, now fails with an error naming the file, and that such a page must be rewritten as an object with named keys.
- AC-7.3 THE developer guide (`CLAUDE.md`) SHALL describe the non-object rule and its place in the frontmatter precedence. It SHALL no longer say that a non-object "keeps its current error".
- AC-7.4 THE system overview (`specs/_system/overview.md`) SHALL no longer describe the array quirk as open in its frontmatter risk row.
- AC-7.5 THE RISK-4 requirements (`specs/risk-4/requirements.md`) SHALL note that the "valid JSON but not an object" part of AC-5.1 is superseded by `specs/risk-9/requirements.md`, keeping the original criterion's wording, and SHALL get a Changelog row.
- AC-7.6 THE backlog (`specs/_system/backlog.md`) SHALL record the maintainer's decision of 2026-09-27 in three places:
  - **FEAT-4:** its section SHALL say the item is deferred, not dropped, and that JSON stays the frontmatter format for now, to keep options open. This replaces "Undecided". Its status SHALL stay `open` (Open question 2). Its "Done when" SHALL no longer say that RISK-9 is closed as moot.
  - **RISK-9:** its section SHALL no longer say that FEAT-4 would make the item moot. It SHALL record that RISK-9 went ahead because FEAT-4 was deferred.
  - **Recommended order:** the sentence "FEAT-4 would replace JSON frontmatter altogether, so decide on it before RISK-9, which it would make moot." SHALL be replaced by one that records the decision: FEAT-4 is deferred and JSON stays for now, so RISK-9 no longer waits on it.
- AC-7.7 In the change that lands RISK-9, THE backlog SHALL mark RISK-9 `done`, in both its Index row and its section, with a note on how it was resolved. The pull request title SHALL end with `(RISK-9)`.

## Out of scope
- FEAT-4 itself, or any other change to the frontmatter format. JSON stays.
- An empty or whitespace-only frontmatter block (`---` directly followed by `---`). It is not valid JSON and keeps today's error (AC-2.6).
- Changing the RISK-4 key rules, their message or their order, or how `null` values for optional keys are handled.
- Collecting errors across pages. The build still stops at the first page that fails.
- Rewording other JSON-library messages (invalid JSON, wrong-typed values, duplicate keys) or the `mango.json` errors.
- Conversion help for pages that use array frontmatter.
- Cutting the 0.3.0 release itself. This spec only requires the changelog entry.

## Open questions
1. **Wording of the error, and what it names.** Proposed: `<file>: frontmatter must be a JSON object, found <kind>`, where `<kind>` is `an array`, `a string`, `a number`, `a boolean` or `null` (AC-3.2). It names the kind of value rather than echoing the value, which could be long or deeply nested. That follows the spirit of "name the file and the value" without printing the frontmatter back. The alternative is a message without the kind, or one that includes a short excerpt of the value.
2. **FEAT-4's status.** The backlog's statuses are `open`, `done` and `dropped`. There is no `deferred`. Proposed: keep FEAT-4 `open` and record "deferred, not dropped (2026-09-27)" in its section (AC-7.6). The alternative is to add a `deferred` status to the backlog's status list and use it.
3. **Scalars get the new message too.** Strings, numbers, booleans and `null` already fail today, with the JSON library's "invalid type" text (AC-1.3). Proposed: give them the same new message as arrays, so that there is one rule and one message (AC-2.1, AC-3.3). This changes the wording of an existing error but not whether the build fails. The alternative is to change arrays only and leave scalars as they are.

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-27 | 20260927-063534-risk-9-non-object-frontmatter | New spec from backlog RISK-9. Records as baseline that array frontmatter in field order builds, and that other non-object frontmatter fails with the JSON library's text. From now on, any frontmatter whose top-level JSON value is not an object fails the build with one error naming the file and the kind of value found, drafts included, before key, type, date, tag and file-name checks. Supersedes part of risk-4 AC-5.1. Records the maintainer's 2026-09-27 decision to defer FEAT-4 in the backlog. Breaking; ships in 0.3.0. |
