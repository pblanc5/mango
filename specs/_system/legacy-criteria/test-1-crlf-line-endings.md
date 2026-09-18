# Pin CRLF line-ending behavior with tests (TEST-1)

**Legacy acceptance criteria, recovered 2026-09-17 (SPEC-2).** Recovered verbatim from `.dev-pipeline/runs/20260917-002445-specs-system-backlog-md-test-1-no-test/01-plan.v1.md`, which is gitignored and exists only in the archive outside this repository. These predate the spec-scoped `AC-<spec-id>.<n>` convention and use numeric groups that **collide with other batches** — see [the index](README.md) for which groups belong to which run.

- Run: `20260917-002445-specs-system-backlog-md-test-1-no-test`
- Groups defined here: `AC-12`
- Criteria: 16

## Requirements

### REQ-1 Pin CRLF frontmatter parsing with unit tests
Add unit tests to the existing `#[cfg(test)] mod tests` in `src/content/frontmatter.rs` that assert what `parse` does today with `\r\n`. All criteria are **[baseline]**: they must pass against unmodified production code.

- AC-12.1 [baseline] A document whose every line ends with `\r\n` — opening `---`, a multi-line JSON object including `date` and `tags`, closing `---`, and a multi-line body with a blank line and a trailing `\r\n` — parses: every frontmatter field has the expected value with no `\r` in it, and the returned body equals the LF text `"# Hello\n\nworld"` (CRLF normalized to LF, final newline dropped) and contains no `\r`.
- AC-12.2 [baseline] For the same logical document written once with LF and once with CRLF, `parse` returns equal bodies and equal field values (`title`, `author`, `description`, `date`, `tags`, `draft`). The test constructs the CRLF input from the LF input, so the two cannot drift apart.
- AC-12.3 [baseline] A leading UTF-8 BOM combined with CRLF throughout still parses: the frontmatter is returned and the body is the LF-normalized text. (Pins that BOM stripping happens before line splitting.)
- AC-12.4 [baseline] A document that mixes both endings in one file — at least one CRLF delimiter line and one LF delimiter line, mixed endings inside the JSON block, and mixed endings in the body — parses, and the returned body is entirely LF.
- AC-12.5 [baseline] A CRLF document with an opening `---` and no closing `---` is still `Err(MangoError::Frontmatter(_))`, matching the LF case (`unterminated_block_is_an_error`).
- AC-12.6 [baseline] A CRLF document with **no** frontmatter returns `(None, body)` where the body is the input **verbatim, with its `\r\n` preserved** — the one path where CRLF is not normalized. The test carries a comment saying this path is unreachable from the CLI today (the loader turns `None` into the `missing frontmatter` error) and that the assertion records current behavior, not a guarantee.

### REQ-2 Pin CRLF end-to-end through a real build
Add temp-site E2E tests to `tests/build.rs` that prove a CRLF content file builds and produces the same output as its LF twin. **[baseline]** throughout.

- AC-12.7 [baseline] A build of a temp site containing one page whose text is entirely CRLF (including the final newline), with a `date` and a `tags` array in its frontmatter, exits 0 with empty stderr-failure (`assert_success`) and writes `dist/<slug>/index.html`, the section/tag outputs the page implies, and nothing unexpected. The HTML contains the heading and paragraphs from the body, `<time datetime="2026-01-24">` (proving no `\r` leaked into a JSON date value, which `parse_date` would have rejected), and the tag link `/tags/<tag>/` (proving the same for tags, which `validate_tags` would have rejected).
- AC-12.8 [baseline] Two temp sites that differ **only** in line endings (same file names, same templates, same assets, no config) build to byte-identical output: the two `snapshot` manifests are equal, and for every relative path the file bytes are equal. The test comment states why this is the discriminating assertion (pulldown-cmark accepts CRLF, so HTML-shape checks alone would pass even if normalization were removed).
- AC-12.9 [baseline] In the CRLF build, the rendered content region of the page HTML — the text between `</header>` and `</article>`, via the existing `between` helper — contains no `\r`. Scoping the check to the content region keeps it independent of the line endings of the committed template files, which a Windows checkout with `core.autocrlf=true` would rewrite.

### REQ-3 Stop and report if any pinned behavior does not hold
- AC-12.10 If any criterion in REQ-1 or REQ-2 fails against unmodified production code, the parser is **not** changed to make it pass. The Developer stops, leaves the repository with no production change, and reports the observed behavior (input, expected, actual) so the maintainer can decide whether it becomes its own backlog item. A test that is merely mis-written (wrong expected string, bad escaping) is fixed in the test, which is not the same thing — the report is for a genuine behavioral difference.
- AC-12.11 No file under `src/` changes outside a `#[cfg(test)] mod tests` block. In particular `src/content/frontmatter.rs` is unchanged above its `#[cfg(test)]` line. The Developer states this explicitly in the hand-off so the Reviewer can check it with a diff.

### REQ-4 Document what the tests prove
- AC-12.12 `README.md`, "Content" section, gains one sentence stating that source files may use LF or CRLF line endings and that the two produce identical output. It is added only for the behaviors the passing tests actually establish; if REQ-3 triggers, no doc claim is written.
- AC-12.13 `CLAUDE.md`, in the "Frontmatter is JSON, not YAML" paragraph (next to the existing BOM clause), gains a short clause: CRLF is accepted on the delimiters, the JSON block and the body, and the body is normalized to LF (with the no-frontmatter early return noted as the exception if REQ-1's AC-12.6 holds).

### REQ-5 Close the backlog item
- AC-12.14 `specs/_system/backlog.md` marks TEST-1 `done` in both the index table row and the item body, and the item body is rewritten in the style of RISK-1/RISK-2: what was pinned, which behaviors, and the names of the tests that pin them. Any behavior found not to hold under REQ-3 is recorded there as a new open item only if the maintainer asks for it in review — the Developer does not invent one.

### REQ-6 The gate passes
- AC-12.15 `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` passes with no new warnings and no `#[allow]` added.
- AC-12.16 Every new test carries an `// AC-12.<n>` comment naming the criterion it covers, per the project's existing convention.

## Verified by

From the run's test report (`03-test.v3.md`), mapping each criterion to the test that verified it. Test names are as of that run and may have moved since.

| AC | Kind | Verified by | Result |
|---|---|---|---|
| AC-12.1 – AC-12.9 | baseline | Unchanged tests, byte-identical to what I validated in attempt 2 (`git diff 37dbe78 a8af22d -- tests/ src/` empty); all pass in `cargo test`. Attempt-2's mutation evidence carries per the user's instruction | pass |
| AC-12.10 | new | Every REQ-1/REQ-2 criterion still passes against production code proven byte-identical to baseline — "stop and report" correctly never fired | pass |
| AC-12.11 | new | 0 deletions under `src/`; sole hunk starts at line 118, well below `#[cfg(test)]` at 51; shipped parser byte-identical to `004a4b5`; attempt 3 touched `src/` not at all | pass |
| AC-12.12 | changed | `README.md:66`, now two sentences. **Both halves measured by me**: content half — CRLF twin builds byte-identical output (probe B, and `crlf_and_lf_sites_build_identical_output`); template/asset half — CRLF templates give 6 CR lines in an 8-line page while the content-derived lines stay LF, and the CRLF asset is a byte-for-byte copy (probe A). The false universal ("Source files … a file written on Windows") is gone | pass |
| AC-12.13 | changed | `CLAUDE.md` frontmatter paragraph. Every element the criterion names survives the trim, grepped individually: delimiters (`str::lines` drops the `\r`, "so the delimiters match"), JSON block ("re-joined with `\n`"), body ("normalized to LF (its final newline dropped)"), and the no-frontmatter early return "which hands the content back verbatim, CRLF included" with its CLI-unreachability. Plus the new, measured scope clause. Nothing required was lost | pass |
| AC-12.14 | changed | TEST-1 `done` in the index row (line 25) and the body header (line 180); RISK-style body naming all six unit tests, both E2E tests, the mutation result, the degradation mode and the symmetry limit; the universal now hedged to "almost everything it parses … the evidence is broad, not exhaustive, so treat the list as open" | pass |
| AC-12.15 | new | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` all exit 0; no `#[allow]` anywhere in `git diff 004a4b5 a8af22d` | pass |
| AC-12.16 | new | Unchanged: 6 `// AC-12.n` comments in `frontmatter.rs`, 3 in `tests/build.rs` | pass |
