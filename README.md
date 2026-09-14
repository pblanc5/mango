# mango

A small static site generator written in Rust. It loads markdown files with JSON frontmatter, renders them through [Tera](https://keats.github.io/tera/) templates, and writes a static site to `dist/`.

## Usage

```sh
# from a site directory containing site/, meta/templates/, meta/assets/
mango build

# custom paths
mango build --site site --templates meta/templates --assets meta/assets -o dist

# remove the build output
mango clean
```

Try it against the sample site in this repo:

```sh
cd test && cargo run -- build
```

Early-stage project — the `run` (dev server) and `publish` subcommands are not implemented yet. See `CLAUDE.md` for architecture notes.
