---
{
    "title": "Building a Corne keyboard",
    "author": "Wren Calloway",
    "description": "My first split keyboard: 42 keys, hot-swap sockets and a weekend of soldering",
    "date": "2025-03-10",
    "tags": ["hardware", "keyboards"],
    "draft": false
}
---

**Status:** daily driver · **Layout:** 3×6 + 3 thumb keys per half · **Firmware:** QMK

The Corne is a split keyboard with 42 keys. That sounds like too few until you learn to
use layers: numbers and symbols live under the thumbs, and my hands never leave the home
row.

## The build

1. Solder the diodes. Forty-two of them, all facing the same way. Check twice.
2. Solder the hot-swap sockets, so switches can be swapped without desoldering.
3. Add the microcontrollers and the TRRS jacks that connect the halves.
4. Flash the firmware and test every key before closing the case.

It took two evenings. The only mistake was one diode soldered backwards, which is why
step 1 says to check twice.

## Six weeks later

My typing speed dropped from 85 to 40 words per minute in the first week and was back
above 80 by week six. My wrists feel better, and I no longer reach for the mouse to hit
the arrow keys.

## Keymap

The layout is plain QWERTY on the base layer, with a symbol layer tuned for Rust:
`&`, `|`, `<` and `>` all sit on the home row.
