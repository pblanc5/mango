---
{
    "title": "Tidepool",
    "author": "Wren Calloway",
    "description": "A falling-sand toy written in Rust that runs in the browser with WebAssembly",
    "date": "2025-09-15",
    "tags": ["rust", "webassembly"],
    "draft": false
}
---

**Status:** finished, occasional updates · **Language:** Rust + WebAssembly

Tidepool is a sandbox of sand, water, oil and fire. Every element follows a few local
rules, and the interesting behavior comes from the interactions: water sinks through
sand, oil floats on water, fire spreads along oil and turns water into steam.

## How it works

The world is a 320×200 grid updated in a single pass per frame. Each cell looks at its
neighbors below and to the sides and swaps places if its rules allow it. Alternating the
scan direction every frame stops everything from drifting to the left.

Rendering is a straight copy from the Rust frame buffer to the canvas. I wrote up the
details in [Rust & WebAssembly: drawing on a canvas](/blog/rust-and-wasm-canvas/).

## Numbers

| Grid size | Cells  | Update time |
|-----------|-------:|------------:|
| 160×100   | 16,000 | 0.4 ms      |
| 320×200   | 64,000 | 1.6 ms      |
| 640×400   | 256,000| 6.9 ms      |

## Ideas I have not built

Wind, plants that grow toward water, and a way to share a drawing as a link.
