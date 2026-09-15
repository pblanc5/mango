---
{
    "title": "Why I still reach for make",
    "author": "Wren Calloway",
    "description": "A forty-year-old tool that still makes a good front door for a project",
    "date": "2025-11-08",
    "tags": ["tools"],
    "draft": false
}
---

Every project I start gets a `Makefile` in the first hour, even the Rust ones where Cargo
does most of the work.

## A front door, not a build system

I do not use make to build anything clever. I use it so that every project answers the
same three questions the same way:

```make
check:
	cargo fmt --check && cargo clippy -- -D warnings && cargo test

serve:
	python3 -m http.server -d dist 8080

release:
	cargo build --release
```

Six months later, `make check` still works, and I do not have to remember which flags
this particular project wanted.

## Why not a script folder?

Scripts drift. They grow arguments, then options, then their own help text. A Makefile
stays a list of names, and `make` with no arguments shows me the first one.

## When I skip it

For one-file experiments, and for anything where the team already has a task runner. The
point is one obvious entry point, not make itself.
