---
{
    "title": "Fixing a flaky test in 40 lines",
    "author": "Wren Calloway",
    "description": "A test that failed one run in fifty, and the directory listing that caused it",
    "date": "2026-01-24",
    "tags": ["rust", "testing"],
    "draft": false
}
---

[Lantern](/projects/software/lantern/) had a test that failed roughly once every fifty
runs in CI and never on my machine. The failure message was a list of files in the wrong
order.

## The cause

The test created a few log files, asked Lantern to follow the directory, and compared the
output with a fixed list. Lantern read the directory with `fs::read_dir`, which makes no
promise about order. My laptop's filesystem happened to return entries alphabetically;
the CI runner's did not, sometimes.

## The fix

Sort once, at the boundary, and never rely on the filesystem again:

```rust
let mut entries: Vec<_> = fs::read_dir(dir)?
    .collect::<Result<_, _>>()?;
entries.sort_by_key(|entry| entry.file_name());
```

Then I added a test that builds the same output twice and compares the bytes. It would
have caught this on day one.

## The lesson

If output order matters, decide it explicitly. "It works on my machine" is often just "my
machine sorts things".
