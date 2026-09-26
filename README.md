# mango

A small static site generator written in Rust. It loads markdown files with JSON frontmatter, renders them through [Tera](https://keats.github.io/tera/) templates, and writes a static site with section indexes, tag pages, an RSS feed and a sitemap.

## Quick start

Each [release](https://github.com/pblanc5/mango/releases) has prebuilt binaries for Linux (x86_64, statically linked, `.tar.gz`) and Windows (x86_64, `.zip`), with a `SHA256SUMS` file to check them against. Unpack the archive and put `mango` (or `mango.exe`) on your `PATH`. What changed in each version is in [`CHANGELOG.md`](CHANGELOG.md).

To build from source instead, you need the Rust toolchain pinned in `rust-toolchain.toml` (1.92):

```sh
cargo install --path .

# build the sample site in this repo into example/dist
cd example && mango build
```

To look at the result, serve `example/dist` with any static file server, for example `python3 -m http.server -d example/dist 8080`, and open http://localhost:8080/.

## Project layout

All paths are relative to the directory you run mango in.

```text
mango.json          optional site config
site/               markdown content; folders become sections
meta/templates/     Tera templates: page.html, section.html, home.html, tags.html, tag.html
meta/assets/        copied as-is into dist/assets/
dist/               build output (emptied on every build)
```

## Commands

| Command | What it does |
|---|---|
| `mango build` | Build the site. Flags: `--site <dir>` (default `site`), `--templates <dir>` (default `meta/templates`), `--assets <dir>` (default `meta/assets`), `-o, --output <dir>` (default `dist`), `--config <file>` (default `mango.json`). |
| `mango clean` | Remove the output folder (`-d, --dist <dir>`, default `dist`). |
| `mango run` | Not implemented yet; it exits with an error. |

Errors go to stderr and exit with status 1.

## Content

Every `.md` or `.markdown` file (any letter case) under `site/` is a page. It must start with a JSON frontmatter block:

```markdown
---
{
    "title": "Post One",
    "author": "tester",
    "description": "my first post",
    "date": "2026-01-24",
    "tags": ["blog", "rust"],
    "draft": false
}
---

Markdown body.
```

| Field | Required | Rules |
|---|---|---|
| `title`, `author`, `description` | yes | strings |
| `draft` | yes | `true` pages are validated but never published |
| `date` | no | strict `YYYY-MM-DD`; an invalid or impossible date fails the build |
| `tags` | no | each tag is lowercase ASCII letters and digits separated by single hyphens (`static-site`); duplicates are dropped |

Any other key is an error, drafts included, which catches typos such as `tag` for `tags` or `dates` for `date`. Keys are compared exactly, so `Title` is not `title`. A page with bad keys gets one error that lists every unknown and every missing key, followed by the accepted keys, so you can fix them all at once:

```text
Mango Frontmatter Error: site/posts/one.md: invalid frontmatter keys: unknown 'tag', unknown 'titel', missing 'title'; accepted keys are 'title', 'author', 'description', 'date', 'tags', 'draft'
```

Content files may use LF or CRLF line endings, or a mix of the two: a page written on Windows and its Unix twin build to byte-identical output. Templates and assets are not normalized — whatever line endings they have is what lands in `dist/`.

File and folder names may only use ASCII letters, digits, `-`, `_` and `.`, so every URL is valid without encoding. `site/posts/post_one.md` becomes `/posts/post_one/`. A folder name or file name (without its extension) made only of dots, such as `...md`, is rejected, since it would publish the page outside its folder. A backslash (`\`) is not allowed in a file or folder name either: on Linux and macOS a name such as `a\b.md` fails the build instead of being split into folders. On Windows `\` is simply the folder separator.

Markdown supports tables, footnotes, strikethrough, task lists and explicit heading IDs (`## Title {#my-id}`).

## Config (`mango.json`)

All fields are optional. Unknown fields are an error, which catches typos.

| Field | Default | Meaning |
|---|---|---|
| `title`, `author`, `description` | unset | Available to templates as `config.*` |
| `base_url` | unset | Must start with `http://` or `https://`. Needed for the feed and sitemap; page links stay root-relative. |
| `recent_count` | `10` | How many dated pages the home page and the feed list |

## Output

| URL | Source |
|---|---|
| `/` | `home.html`: the most recent dated pages and the top-level sections |
| `/<path>/` | `page.html`, one per page |
| `/<folder>/` | `section.html`, one per folder at every level: its pages (newest first, undated last) and subfolders |
| `/tags/` and `/tags/<tag>/` | `tags.html` and `tag.html` |
| `/feed.xml`, `/sitemap.xml` | RSS 2.0 feed and sitemap, only when `base_url` is set |
| `/assets/...` | copied from `meta/assets/` |

`/tags/` is always generated, so a `tags/` folder or `tags.md` page is an error. So is any other pair of outputs that would write the same file, or a file where another output needs a folder.

## Templates

All five templates are required. Every template gets `config`; `page.html` gets `page`, `section.html` gets `section`, `home.html` gets `home`, `tags.html` gets `tags` and `tag.html` gets `tag`. HTML autoescaping is on, so print URLs with `{{ page.url | safe }}`. See `example/meta/templates` for a working set, and `CLAUDE.md` for every context field.

## Safe builds

A build that fails on bad content, config, templates, assets or conflicting outputs stops before `dist/` is touched, so the previous output stays intact. mango also refuses to clean an output folder that is, or contains, the current directory, the site, templates or assets folder, or the config file.

## Known limitations

- No dev server (`run`) yet. There is no `publish` command: deploying `dist/` is left to whatever tool you already use.
- No pagination, section intro pages, syntax highlighting or automatic heading IDs.
- `clean` has no `--config` option, so it does not protect a config file stored inside the output folder.
- A symlink inside `meta/assets/` may point at a file (its contents are copied), but a symlink to a folder is a build error: use a real folder. A broken link or a link loop is an error too. The `--assets` folder itself may be a symlink.
- A symlink inside `site/` may point at a markdown file (it is read, and its URL comes from the link's path), but a symlink to a folder is a build error: use a real folder. As in `meta/assets/`, a broken link or a link loop under `site/` is a build error too, whatever the link's name. The `--site` folder itself may be a symlink.

## Development

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

`CLAUDE.md` describes the architecture and the definition of done every change must meet.

## License

mango is licensed under the GNU General Public License v3.0 or later. See [LICENSE](LICENSE).
