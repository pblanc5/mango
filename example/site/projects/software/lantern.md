---
{
    "title": "Lantern",
    "author": "Wren Calloway",
    "description": "A small terminal log viewer that follows files and highlights the lines that matter",
    "date": "2026-02-20",
    "tags": ["rust", "tools"],
    "draft": false
}
---

**Status:** usable daily, version 0.4 · **Language:** Rust · **License:** GPL-3.0

Lantern follows one or more log files, like `tail -f`, and colors lines by rules you write
in a small config file. I built it because every log viewer I tried either did too much
or could not keep up with a busy service.

## Features

- Follows files through rotation and truncation
- Highlight rules with regular expressions, e.g. `level=error` in red
- Pauses the stream while you scroll, then catches up
- Starts in under 10 ms and uses about 4 MB of memory

## Example

```toml
[[rule]]
match = "level=(error|fatal)"
color = "red"

[[rule]]
match = "took=\\d{4,}ms"
color = "yellow"
```

```sh
lantern --rules rules.toml /var/log/app/*.log
```

## What's next

Filtering by time range, and a JSON mode that pretty-prints structured logs. The story of
its most annoying bug is in [Fixing a flaky test in 40 lines](/blog/fixing-a-flaky-test/).
