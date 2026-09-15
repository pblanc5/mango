---
{
    "title": "How I write these posts",
    "author": "Wren Calloway",
    "description": "The markdown features this site uses, and the list every post goes through before it goes live",
    "date": "2026-02-14",
    "tags": ["writing", "markdown", "writing"],
    "draft": false
}
---

Every page on this site is a markdown file with a small block of JSON at the top. This
post doubles as my reference for the formatting I actually use.

## Frontmatter

The header of this very file looks like this:

```json
{
    "title": "How I write these posts",
    "author": "Wren Calloway",
    "date": "2026-02-14",
    "tags": ["writing", "markdown"],
    "draft": false
}
```

Setting `draft` to `true` keeps a post out of the site, the feed and the sitemap until
it is ready.

## Tables {#tables}

I use tables for anything I would otherwise explain in three paragraphs:

| Tool          | What I use it for          | Cost   |
|:--------------|:---------------------------|-------:|
| Helix         | Writing and code           | free   |
| Vale          | Catching wordy sentences   | free   |
| A notebook    | Outlines, away from a screen | $12  |

## Footnotes and edits

Footnotes keep side notes out of the way.[^origin] When I change my mind about something
after publishing, I strike it out rather than delete it: the post used to recommend
~~writing every morning~~ writing when there is something to say.

[^origin]: I picked this habit up from reading too many academic papers.

## The checklist {#checklist .no-toc}

Before a post goes live:

- [x] Read it out loud once
- [ ] Cut the first paragraph if it is only warming up
- [ ] Check every link

Everything else follows [CommonMark](https://commonmark.org), plus the extensions above.
The spec lives at <https://spec.commonmark.org>.
