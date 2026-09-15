---
{
    "title": "Mango Task Tracker",
    "author": "tester",
    "description": "tracks the progress on the mango SSG project",
    "date": "2026-02-07",
    "tags": ["mango", "progress"],
    "draft": false
}
---

## Static Site Generator – Task Breakdown

> Goal: A Rust-based static site generator using Tide, pulldown-cmark, and Tera  
> Non-goals (for now): plugins, hot reload, incremental builds, multiple content types

---

## Milestone 0 — Project Skeleton & Guardrails

**Purpose:** Prevent chaos and overengineering later.

- [ ] Create base repo structure
- [ ] Add CLI subcommands:
    - `build`
    - `serve`
- [ ] Define a single hardcoded config struct
- [ ] Print active mode (`build` or `serve`) on startup
- [ ] Write README documenting:
- project goals
- non-goals
- directory layout

**Done when:**  
Running the binary clearly indicates which mode is active.

---

## Milestone 1 — Markdown + JSON Frontmatter Parsing

**Purpose:** Turn files into structured data.

- [ ] Define `Page` struct:
- slug
- title
- tags
- raw_markdown
- rendered_html (placeholder)
- [ ] Detect JSON frontmatter block
- [ ] Deserialize JSON frontmatter into struct
- [ ] Load all `.md` files from `/site`
- [ ] Error loudly on malformed frontmatter
- [ ] Unit test frontmatter parsing

**Done when:**  
Parsed `Page` structs can be printed to stdout.

---

## Milestone 2 — Markdown → HTML Rendering

**Purpose:** Convert content, no templates yet.

- [ ] Integrate `pulldown-cmark`
- [ ] Convert markdown body to HTML
- [ ] Attach rendered HTML to `Page`
- [ ] Normalize or sanitize HTML output
- [ ] Snapshot test markdown → HTML output

**Done when:**  
Each page has valid HTML content.

---

## Milestone 3 — Template Rendering (Tera)

**Purpose:** Produce real HTML pages.

- [ ] Initialize Tera from `/meta/templates`
- [ ] Create base layout template
- [ ] Create page template
- [ ] Render a single `Page` with Tera
- [ ] Pass metadata (title, tags, content) to template
- [ ] Gracefully handle missing templates

**Done when:**  
One markdown file renders into one HTML page.

---

## Milestone 4 — Static Build Output

**Purpose:** Generate a browsable static site.

- [ ] Decide URL structure (`/slug/index.html`)
- [ ] Create output directories under `/public`
- [ ] Write rendered HTML files
- [ ] Clean output directory before build
- [ ] Copy static assets (if present)

**Done when:**  
Opening `/public/index.html` works in a browser.

---

## Milestone 5 — Tags System

**Purpose:** Generate derived content.

- [ ] Build tag index from pages
- [ ] Create tag listing template
- [ ] Generate one page per tag
- [ ] Link tags from individual pages
- [ ] Add tests for tag grouping

**Done when:**  
Clicking a tag shows related pages.

---

## Milestone 6 — RSS Feed & Sitemap

### RSS
- [ ] Select latest pages for feed
- [ ] Generate RSS XML
- [ ] Write `/public/rss.xml`
- [ ] Validate RSS output

### Sitemap
- [ ] Collect all site URLs
- [ ] Generate sitemap XML
- [ ] Write `/public/sitemap.xml`
- [ ] Validate sitemap output

**Done when:**  
Both files pass standard validators.

---

## Milestone 7 — Tide Servers (Dev & Prod)

### Dev Server
- [ ] Serve `/public` directory with Tide
- [ ] Trigger full rebuild on startup
- [ ] Add manual rebuild endpoint

### Production Server
- [ ] Serve static files only
- [ ] Disable rebuild logic
- [ ] Make port configurable

**Done when:**  
Dev previews and prod serving work using the same output.

---

## Workflow Rules (Read This)

- Work on **one milestone at a time**
- Keep only **3 tasks max** in `NEXT.md`
- Everything else lives here
- A task is done when:
- it compiles, or
- a test passes, or
- a file renders

Perfection is out of scope.
