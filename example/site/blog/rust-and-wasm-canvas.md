---
{
    "title": "Rust & WebAssembly: drawing on a <canvas>",
    "author": "Wren Calloway",
    "description": "Pixels, \"quotes\" & a 60 fps render loop without a framework",
    "date": "2026-03-02",
    "tags": ["rust", "webassembly"],
    "draft": false
}
---

I rebuilt the renderer for [Tidepool](/projects/software/tidepool/) this winter, and the
most useful thing I learned is how little glue you need between Rust and a `<canvas>`
element. No framework, no bundler, one `wasm-bindgen` export and about eighty lines of
JavaScript.

## Own the pixels in Rust

The trick is to keep the frame buffer on the Rust side and hand the browser a view of it.
The simulation writes RGBA bytes into a `Vec<u8>`, and JavaScript wraps that memory in an
`ImageData` without copying:

```rust
#[wasm_bindgen]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

#[wasm_bindgen]
impl Frame {
    pub fn pixels_ptr(&self) -> *const u8 {
        self.pixels.as_ptr()
    }

    pub fn step(&mut self) {
        // update the sand, then write colors into self.pixels
    }
}
```

Each animation frame calls `step()`, builds a `Uint8ClampedArray` over the wasm memory at
`pixels_ptr()`, and passes it to `putImageData`. At 320×200 that holds a steady 60 fps on
my five-year-old laptop.

## Guard the size

Resizing was the only crash I hit. If the canvas collapses to zero pixels, the view is
empty and the browser throws. The loop now only redraws while width > 0 && height < 4096,
and it rebuilds the buffer when either changes.

## What I would do differently

- Start with the frame buffer design, not the drawing API.
- Measure in release builds only; debug wasm is ten times slower and tells you nothing.
- Keep the JavaScript boring. Mine never grew past one file.

The full source is in the Tidepool repository if you want to poke at it.
