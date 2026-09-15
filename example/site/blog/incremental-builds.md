---
{
    "title": "Notes on incremental builds",
    "author": "Wren Calloway",
    "description": "Half-formed thoughts on caching rendered pages between builds",
    "date": "2026-04-01",
    "tags": ["rust", "wip"],
    "draft": true
}
---

Not ready yet. Drafts are validated on every build but never published, listed, tagged,
added to the feed or put in the sitemap.

- Hash each page's frontmatter and body
- Skip rendering when the hash and the templates are unchanged
- Figure out what to do when a tag list changes
