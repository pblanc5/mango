# mango

A static site generator CLI written in Rust (edition 2024, toolchain pinned in `rust-toolchain.toml`).

Pipeline: load `.md` files → parse frontmatter → load the site config → render markdown with pulldown-cmark → render through Tera templates → write `dist/<slug>/index.html` per page (plus a section index page for every folder level, and the home page at `dist/index.html`) → copy assets into `dist/assets/`.

**Site config is JSON too** (`src/config.rs`, `SiteConfig`). `build --config <path>` (default `mango.json`, cwd-relative). The default file is optional (missing → defaults); an explicitly given `--config` that doesn't exist is an error naming the path. `#[serde(deny_unknown_fields)]`: malformed JSON, unknown fields (typos) or a bad `recent_count` fail with `MangoError::Config` naming the file. Fields, all optional: `title`, `author`, `description`, `base_url` (strings), `recent_count` (non-negative integer, default 10). `base_url` is only exposed to templates — generated URLs stay root-relative. The config file path is protected from cleaning (see below). See `test/mango.json`.

**Template contexts** (Tera autoescape is on for `.html`, so print URLs with `| safe`):
- every template: `config` — always has all five keys; unset strings are JSON `null` (renders empty, falsy, and triggers `default(...)`; `""` would not).
- `page.html`: `page` — `title`, `author`, `description`, `date`, `tags`, `url` (`/<slug>/`), `content`.
- `section.html`: `section` — `slug`, `url`, `pages` (direct child page summaries: `title`, `date`, `slug`, `url`), `subsections` (direct child sections: `slug`, `url`, sorted by slug).
- `home.html`: `home` — `recent` (up to `config.recent_count` dated pages site-wide, section ordering, undated dropped) and `sections` (top-level sections: `slug`, `url`, `page_count` = direct child pages).
- `page.html`, `section.html` and `home.html` are all required (names hardcoded in `render/template.rs`); a missing one fails at render time, before cleaning.

**Markdown extensions** enabled in `render/markdown.rs`: tables, footnotes, strikethrough, task lists, explicit heading attributes (`# Title {#my-id}`). No auto-generated heading IDs.

**Frontmatter is JSON, not YAML** — a `---`-delimited block containing a JSON object (see `test/site/posts/post_one.md`). Required fields: `title`, `author`, `description`, `draft`; optional: `date`, `tags`. `date` must be a strict `YYYY-MM-DD` string (parsed into `Option<chrono::NaiveDate>` by `page::parse_date`); a present but invalid or impossible date — including `""`, and on draft pages — is a build error naming the file and the value. Templates still see `page.date` / `section.pages[].date` as `"YYYY-MM-DD"`, or `""` when unset. A markdown file without frontmatter (or with malformed frontmatter), at any depth, is a build error naming the file. Pages with `"draft": true` are still parsed but excluded from output and section indexes (filtered in `loader::load`).

## Commands

```sh
cargo build
cargo test          # unit tests + tests/build.rs integration tests (fixture build + temp-site error cases)
cargo clippy
cargo fmt

# Run against the fixture site (paths are cwd-relative!):
cd test && cargo run -- build
# or from the repo root:
cargo run -- build --site test/site --templates test/meta/templates --assets test/meta/assets -o test/dist
```

**Path gotcha:** all CLI paths resolve relative to the current working directory, with defaults `site/`, `meta/templates/`, `meta/assets/`, `dist/`. The repo root has none of these — the sample site lives under `test/` (`test/site`, `test/meta`; generated `test/dist` is gitignored).

## CLI surface (`src/cli.rs`)

- `build` — the only real subcommand; `build()` in cli.rs is the orchestration point wiring the whole pipeline. Order: load pages → load config → load templates → resolve assets destination (an assets path with no final name, e.g. `.`/`..`, is an error) → build render items (pages, section indexes, home page) → collision check → render **every** page, section and the home page to strings in memory → safety check → empty the output folder → write rendered files → copy assets. Nothing is deleted or written until loading and rendering have succeeded, so a bad page, bad config, missing templates or a Tera render error leaves the previous output untouched (an I/O failure after the clean can still leave partial output).
  - **Collisions:** two render items mapping to the same file (e.g. top-level `posts.md` and the `posts/` section index — including an ancestor index: `a.md` next to `a/b/c.md`) fail with `MangoError::General` naming the path and both sources (`page '<slug>'`, `section index '<slug>'`, `home page`). `posts/index.md` → `posts/index/index.html` is not a collision.
  - **Cleaning:** the output folder's contents are removed (folder kept, symlinks removed not followed; a symlink `remove_file` can't delete falls back to `remove_dir`, for Windows directory links). `ensure_safe_to_clean` refuses an output path that is the cwd, a parent of the cwd, or equal to / a parent of the site, templates or assets folder or the config file (canonicalized comparison).
- `clean` — removes the dist dir; a missing dir is success. Same safety check, protecting the cwd, its parents, and the default `site`, `meta/templates`, `meta/assets` (not the config file; `clean` has no `--config`).
- `run` (dev server) and `publish` — not implemented; they return an error and exit 1. The `tide` dependency and `ServerOpts` fields are reserved for the future `run` server; don't remove them as "unused".

## Module map

- `src/main.rs` — entry; calls `cli::run()`. On error prints to **stderr** and exits 1; exits 0 on success.
- `src/config.rs` — `SiteConfig` and `load(path, explicit)`.
- `src/error.rs` — `MangoError` (thiserror): `Io`, `IoPath { path, source }`, `Frontmatter`, `General`, `Config`, `Template`. Display includes the underlying cause (for `Template`, the full Tera `source()` chain). Use `MangoError::io_at(path, e)` for I/O failures on a specific file/dir.
- `src/content/` — `loader.rs` (recursive site traversal; propagates errors at every depth; drops drafts), `frontmatter.rs` (JSON frontmatter parse), `page.rs` (`Page`, `generate_slug`, `slug_url`), `summary.rs` (`PageSummary`, incl. `url`).
- `src/render/` — `markdown.rs` (pulldown-cmark → HTML with extensions), `template.rs` (Tera glob load; `page.html`, `section.html` and `home.html` template names are hardcoded here; `PageTemplate`/`SectionTemplate`/`HomeTemplate` contexts, each render item also gets `config`).
- `src/build/` — `output.rs` (`check_collisions`, `render` to in-memory `RenderedFile`s, `write` to `dist/<slug>/index.html`; slug `""` → `dist/index.html`), `index/section.rs` (a `Section { pages, subsections }` for every ancestor folder of every page, in a `BTreeMap`, so sections are generated in slug order; pages within a section sorted by `compare_summaries`: newest date first, undated last, ties by title then slug), `generate/{content,section,assets,home}.rs` (`home.rs` builds the home item from pages + section index; `assets.rs` copies recursively; any copy failure fails the build).

## Conventions

- Errors: return `MangoError` via `?`; add variants to `src/error.rs` rather than ad-hoc strings where a category exists. Path-specific I/O errors go through `MangoError::io_at`.
- CLI: clap derive style.
- Tests: unit tests live in `#[cfg(test)] mod tests` next to the code (filesystem fixtures are created per test under `target/unit-fixtures/`); the end-to-end tests (`tests/build.rs`) run the compiled binary — against `test/site` + `test/meta` into `target/integration-dist`, and against per-test temp sites under `CARGO_TARGET_TMPDIR` for error/draft cases. They assert on exit status, stderr and output files.
- `test/site` and `test/meta` are committed fixtures — keep them small and update tests if you change them. Never put broken/error-case files there; build those in test code. `test/dist` is generated output and gitignored.
