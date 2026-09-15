---
{
    "title": "Markdown Extensions",
    "author": "tester",
    "description": "Tables, footnotes, strikethrough, task lists and heading ids",
    "date": "2026-03-01",
    "tags": ["markdown", "blog", "markdown"],
    "draft": false
}
---

Every markdown extension mango enables, in one page. The duplicate `markdown`
tag in the frontmatter is dropped.

## Tables {#tables}

| Feature       | Status |
|:--------------|-------:|
| Tables        | yes    |
| Footnotes     | yes    |

## Footnotes {#footnotes}

Mango renders footnotes.[^fn]

[^fn]: This is the footnote text.

## Strikethrough {#strikethrough}

This is ~~removed~~ text.

## Task lists {#task-lists}

- [x] done item
- [ ] open item

## Heading attributes {#custom-id .fancy}

Code blocks keep their language class:

```rust
fn main() {
    println!("hello from mango");
}
```

Autolinks work too: <https://example.org>
