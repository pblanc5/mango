# mango

A static site generator CLI written in Rust (edition 2024, toolchain pinned in `rust-toolchain.toml`).

Pipeline: load `.md` files → parse frontmatter → render markdown with pulldown-cmark → render through Tera templates → write `dist/<slug>/index.html` per page (plus per-section index pages) → copy assets into `dist/assets/`.

**Frontmatter is JSON, not YAML** — a `---`-delimited block containing a JSON object (see `test/site/posts/post_one.md`). Required fields: `title`, `author`, `description`, `draft`; optional: `date`, `tags`. A markdown file without frontmatter (or with malformed frontmatter), at any depth, is a build error naming the file. Pages with `"draft": true` are still parsed but excluded from output and section indexes (filtered in `loader::load`).

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

- `build` — the only real subcommand; `build()` in cli.rs is the orchestration point wiring the whole pipeline.
- `clean` — `rm -rf` the dist dir (errors, naming the path, if it doesn't exist).
- `run` (dev server) and `publish` — **stubs**. The `tide` dependency is reserved for the future `run` server; don't remove it as "unused".

## Module map

- `src/main.rs` — entry; calls `cli::run()`. On error prints to **stderr** and exits 1; exits 0 on success.
- `src/error.rs` — `MangoError` (thiserror): `Io`, `IoPath { path, source }`, `Frontmatter`, `General`, `Template`. Display includes the underlying cause (for `Template`, the full Tera `source()` chain). Use `MangoError::io_at(path, e)` for I/O failures on a specific file/dir.
- `src/content/` — `loader.rs` (recursive site traversal; propagates errors at every depth; drops drafts), `frontmatter.rs` (JSON frontmatter parse), `page.rs` (`Page`, `generate_slug`), `summary.rs` (`PageSummary`).
- `src/render/` — `markdown.rs` (pulldown-cmark → HTML), `template.rs` (Tera glob load; `page.html` and `section.html` template names are hardcoded here; `PageTemplate` is the `page` context, incl. `description`).
- `src/build/` — `output.rs` (writes `dist/<slug>/index.html`), `index/section.rs` (groups pages by parent slug), `generate/{content,section,assets,home}.rs` (`assets.rs` copies recursively; any copy failure fails the build).

## Known issues / in-progress (don't "fix" without asking)

- `src/build/generate/home.rs` is an intentionally empty stub — home page generation is in-progress work (paired with the empty `test/meta/templates/home.html`).
- Templates reference `config.title` / `config.author` (`test/meta/templates/base.html`) but nothing in Rust supplies a `config` context yet — a site config file is implied but unimplemented.
- Clap doc comments use `//` instead of `///`, so per-flag help text doesn't reach `--help`.

## Conventions

- Errors: return `MangoError` via `?`; add variants to `src/error.rs` rather than ad-hoc strings where a category exists. Path-specific I/O errors go through `MangoError::io_at`.
- CLI: clap derive style.
- Tests: unit tests live in `#[cfg(test)] mod tests` next to the code (filesystem fixtures are created per test under `target/unit-fixtures/`); the end-to-end tests (`tests/build.rs`) run the compiled binary — against `test/site` + `test/meta` into `target/integration-dist`, and against per-test temp sites under `CARGO_TARGET_TMPDIR` for error/draft cases. They assert on exit status, stderr and output files.
- `test/site` and `test/meta` are committed fixtures — keep them small and update tests if you change them. Never put broken/error-case files there; build those in test code. `test/dist` is generated output and gitignored.
