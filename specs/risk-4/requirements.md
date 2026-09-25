<!-- Published by dev-pipeline run 20260925-182336-risk-4-unknown-frontmatter-keys, stage spec v2, approved 2026-09-25T19:05:00Z. -->

# Strict frontmatter keys: Requirements

Spec ID: `risk-4`

## Summary
A page's JSON frontmatter currently accepts any extra key and drops it without a word. A typo such as `"tag"` for `"tags"` or `"dates"` for `"date"` still builds, and the page silently loses its tags or date. This change makes unknown frontmatter keys a build error. Each failing page gets one error that names the file, every unknown key and every missing required key, so the author can fix them all in one pass. The change is breaking for any site that uses extra keys, and it ships in the next minor release (0.2.0).

## Context
- Site authors write frontmatter by hand. A misspelled optional key is the hardest mistake to notice, because nothing fails. Without its date, a page drops off the home page and out of the feed, and loses its sitemap `lastmod`. Without its tags, it doesn't appear on any tag page.
- Unknown keys are not useful today anyway. Templates see only the documented page fields (`CLAUDE.md`, "Template contexts"), so an extra key can never reach the output.
- `mango.json` already rejects unknown fields (`src/config.rs:19`, `README.md:76`). The project's "Strict inputs" convention (`specs/constitution.md`, Conventions) says a bad value fails the build with an error that names the file and the value, drafts included.
- The error message this spec asks for is **new**. It is not the message `mango.json` produces for an unknown field, which is the JSON library's own text, reports one field at a time and puts names in backticks. The new message follows mango's own style for date, tag and file-name errors (values in single quotes) and lists every key problem on the page. The two "unknown field" errors will therefore read differently. Changing the `mango.json` message is out of scope.
- The maintainer has confirmed three things: a hard error with no opt-out, one error per page listing all key problems, and the breaking change with a minor version bump (next release 0.2.0; nobody uses mango yet).
- Backlog item: RISK-4 in `specs/_system/backlog.md`. It asks the spec to settle which fields exist, what the error says, and whether drafts are checked.
- Related spec: `specs/arch-3/requirements.md`. Its AC-1.3 says a frontmatter, date or tag error wins over a file-name error. This spec keeps that rule and puts key errors ahead of all of them.

## Current behavior

### REQ-1 [baseline] The accepted frontmatter fields
Evidence: `src/content/frontmatter.rs:8-16`, `src/content/frontmatter.rs:37-38`, `src/content/loader.rs:62-70`, `README.md:61-66`; tests `parses_valid_frontmatter_and_returns_body`, `optional_fields_may_be_omitted`, `malformed_json_is_an_error`
Status: inferred, confirm at approval
- AC-1.1 [baseline] THE content loader SHALL recognize exactly six frontmatter keys: `title`, `author`, `description` and `draft`, which are required, and `date` and `tags`, which are optional.
- AC-1.2 [baseline] IF a page's frontmatter is not valid JSON, lacks a required key, or has a value of the wrong JSON type THEN THE content loader SHALL fail the build with a `Frontmatter` error whose message starts with the page's file path. Today only the first such problem the JSON parser meets is reported.
- AC-1.3 [baseline] THE content loader SHALL parse and validate draft pages the same way as published pages, and only then leave them out of the output.

### REQ-2 [baseline] Unknown keys are silently ignored
Evidence: `src/content/frontmatter.rs:8` has no `deny_unknown_fields`, unlike `src/config.rs:19`. No test pins this behavior.
Status: inferred, confirm at approval
- AC-2.1 [baseline] WHEN a page's frontmatter contains a key other than the six in AC-1.1 THE content loader SHALL ignore that key, and THE build SHALL succeed with no message. REQ-3 replaces this behavior.

## User stories
- As a site author, I want a misspelled frontmatter key to fail the build, so that I don't publish a page that has silently lost its date or tags.
- As a site author, I want one error that lists every bad and every missing key on the page, so that I can fix them all without rebuilding after each one.
- As a site author, I want the error to show the keys that are allowed, so that I can fix a typo without looking up the docs.
- As a site author, I want drafts checked too, so that a draft doesn't start failing only on the day I publish it.

## Requirements

### REQ-3 Unknown frontmatter keys fail the build
Every top-level key in a page's frontmatter must be one of the six keys in AC-1.1. Any other key is treated as a mistake.
- AC-3.1 IF a page's frontmatter contains a top-level key that is not `title`, `author`, `description`, `date`, `tags` or `draft` THEN THE content loader SHALL fail the build with a `Frontmatter` error.
- AC-3.2 THE content loader SHALL compare keys exactly, including letter case. For example, `Title`, `TAGS` and `title ` (with a trailing space) are unknown keys.
- AC-3.3 IF the unknown key is on a page with `"draft": true` THEN THE content loader SHALL still fail the build.
- AC-3.4 WHEN a page's frontmatter contains only keys from AC-1.1, with every value valid, THE content loader SHALL accept it exactly as before this change. This includes pages that leave out one or both optional keys.
- AC-3.5 WHEN the example site (`example/site`) is built THE build SHALL still succeed, and its output SHALL not change.

### REQ-4 One error per page lists every key problem
A page's "key errors" are its unknown keys (REQ-3) and its missing required keys (`title`, `author`, `description` or `draft` not present). A key that is present counts as present whatever its value, including `null` or a wrong type. The message format is new (see Context).
- AC-4.1 WHEN the build fails because of key errors THE CLI SHALL print the error to stderr and exit with status 1.
- AC-4.2 IF a page has at least one key error THEN THE content loader SHALL report one `Frontmatter` error for that page. The message SHALL contain the page's file path and SHALL name every unknown key and every missing required key on that page, each in single quotes and each labelled as unknown or missing. For example, a page with the keys `titel` and `tag` and no `title` reports unknown `'tag'`, unknown `'titel'` and missing `'title'`.
- AC-4.3 IF a page has at least one unknown key THEN THE error message SHALL also list all six accepted key names.
- AC-4.4 THE error message SHALL list the key errors in a fixed order: first the unknown keys, sorted byte-wise by key text, then the missing required keys, in the order `title`, `author`, `description`, `draft`. The same frontmatter SHALL always produce the same message, whatever order its keys appear in.
- AC-4.5 IF a page has no unknown keys but lacks more than one required key THEN THE error message SHALL name every missing required key. For example, the frontmatter `{}` reports all four as missing.
- AC-4.6 THE error message SHALL name each key only once, even if a key appears more than once in the frontmatter.

### REQ-5 Precedence of frontmatter errors
Some problems stop the keys from being read at all. Others only matter once the keys are right. This requirement fixes which error the author sees first, whatever order the keys appear in.
- AC-5.1 IF the frontmatter block is unterminated, is not valid JSON, or is valid JSON but not an object THEN THE content loader SHALL report that error as it does today, with no key errors.
- AC-5.2 IF a page has key errors and also a value of the wrong JSON type on a known key (for example `"title": 5` or `"draft": "no"`) THEN THE error message SHALL report the key errors (REQ-4) and SHALL NOT report the wrong-typed value. This holds whether the wrong-typed key comes before or after the unknown key in the document.
- AC-5.3 IF a page has key errors and also an invalid `date`, an invalid tag or an invalid file name THEN THE error message SHALL report the key errors (REQ-4) and SHALL NOT report the date, tag or file-name problem.
- AC-5.4 IF a page has no key errors THEN THE content loader SHALL report wrong-typed values, then invalid dates, tags and file names, the same way and in the same order as before this change.

### REQ-6 Failure is safe
- AC-6.1 IF the build fails because of key errors THEN THE build SHALL leave the previous contents of the output folder untouched.

### REQ-7 Documentation
- AC-7.1 THE user guide (`README.md`) SHALL state, next to the frontmatter field table, that unknown frontmatter keys are an error, and that one error lists every unknown and missing key on a page.
- AC-7.2 THE changelog (`CHANGELOG.md`) SHALL record the change under `## [Unreleased]` in the `Changed` group, worded for site authors. It SHALL say that the change is breaking and that a site that used extra keys must remove them.
- AC-7.3 THE developer guide (`CLAUDE.md`) SHALL list unknown keys and the per-page key error, with its precedence, among the frontmatter rules.

## Out of scope
- Collecting errors across pages. The build still stops at the first page that fails. Listing every failing page in one run would be a separate change to how content is loaded.
- Listing more than one wrong-typed value, or combining wrong-typed values, invalid dates or invalid tags into the key error. Only key errors are collected. Other problems follow AC-5.4.
- Keys nested inside values. `tags` is a list of strings, and no frontmatter value is an object.
- Custom keys, for example an `extra` object passed to templates. That would be a separate feature.
- Warnings, or an option to turn the check off (confirmed by the maintainer).
- Changing how duplicate keys are handled (for example `"title"` twice) beyond AC-4.6.
- `null` values for optional keys (`"date": null`). Their current handling is unchanged.
- Changing `mango.json` validation or its error message.
- Cutting the 0.2.0 release itself. This spec only requires the changelog entry.

## Open questions
None. The three questions from attempt 1 are resolved:
1. Error or warning: a hard error with no opt-out (REQ-3, Out of scope).
2. First unknown key only: no. Each page's error lists every unknown and every missing required key (REQ-4), and errors across pages are not collected (Out of scope).
3. Release impact: a breaking change with a minor bump to 0.2.0 is accepted (AC-7.2).

## Changelog
| Date | Run | Change |
|---|---|---|
| 2026-09-25 | 20260925-182336-risk-4-unknown-frontmatter-keys | Initial requirements. Records the current frontmatter fields and the silent-ignore behavior as baseline. Unknown keys become a build error, drafts included. One error per page lists every unknown and missing required key in a fixed order, and key errors take precedence over type, date, tag and file-name errors. Breaking; ships in 0.2.0. |
