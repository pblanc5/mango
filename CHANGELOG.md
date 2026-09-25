# Changelog

All notable changes to mango are recorded here, for the people who use it to build sites. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and mango uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html) for its CLI, content rules, `mango.json`, template contexts and output URLs.

## [Unreleased]

## [0.1.0] - 2026-09-25

The first release. Prebuilt binaries are available for Linux (x86_64, statically linked) and Windows (x86_64).

### Added

- `mango build` turns a folder of markdown pages into a static site through Tera templates, with `--site`, `--templates`, `--assets`, `-o/--output` and `--config` flags (defaults `site`, `meta/templates`, `meta/assets`, `dist` and `mango.json`).
- `mango clean` removes the output folder.
- Pages are `.md` or `.markdown` files with a JSON frontmatter block: `title`, `author`, `description` and `draft` are required; `date` (strict `YYYY-MM-DD`) and `tags` are optional. Draft pages are validated but never published. Content may use LF or CRLF line endings.
- Markdown supports tables, footnotes, strikethrough, task lists and explicit heading IDs (`## Title {#my-id}`).
- Every folder becomes a section with its own index page listing its pages (newest first, undated last) and subfolders; the home page lists the most recent dated pages and the top-level sections.
- Tags: a tag index at `/tags/` and one page per tag at `/tags/<tag>/`.
- An optional `mango.json` site config with `title`, `author`, `description`, `base_url` and `recent_count`. Unknown fields are an error, which catches typos.
- An RSS 2.0 feed (`/feed.xml`) and a sitemap (`/sitemap.xml`), generated when `base_url` is set.
- Assets are copied from `meta/assets/` into `/assets/`.
- Safe builds: invalid content, file names, tags, dates, config, templates, assets or conflicting outputs fail with an error naming the file and the value, before the output folder is touched. mango refuses to clean a folder that is, or contains, the current directory, the site, templates or assets folder, or the config file.
- Output is deterministic: two builds of the same input are byte-identical.
- An example site with a complete theme in `example/`.

[Unreleased]: https://github.com/pblanc5/mango/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/pblanc5/mango/releases/tag/v0.1.0
