# Save Vault's visual system

The whole foundation lives in one file: **`src/gui/design.rs`**. Colours stay in `src/gui/style.rs`,
where the theme already held a coherent palette taken from the v2 mockup; this system did not repaint
it. What it added is everything the palette never covered — type, space, radius, elevation, control
geometry — and what it removed is the habit of each screen choosing those numbers for itself.

## What was measured, before anything was changed

| Axis | Before | After |
|---|---|---|
| Font size | 8 explicit values (11, 12, 13, 14, 15, 16, 20, 25), plus iced's implicit default of 16 | 4 |
| Padding | 19 distinct values | 6, all from the scale |
| Spacing | 9 distinct values | 6, all from the scale |
| Corner radius | 7 (0, 2, 4, 5, 9, 10, 20) | 1, plus the pill for badges and chips |
| Button height | 3, none of them declared — each helper picked its own padding and the height fell out of it | 1, declared |

None of that was ever decided. It accumulated one screen at a time, which is the usual way an
interface stops looking like one product.

## The scales

**Type** — 12 · 14 · 16 · 20. Four steps, and body is 14.

It was 16, iced's default, which on Windows at 125% display scaling draws at 20 physical pixels. For
an app whose screens are mostly folder paths and file counts, that is a step too large, and it is
what the complaint was about. The ramp is deliberately tighter than the house web ramp (which keeps
~25% between neighbours): a desktop tool has a shorter useful range, since the body must stay small
enough to fit a path on one line and the largest thing on screen is a screen title, not a headline.

Hierarchy is carried by **weight and colour**, not by size alone — which is why `NotoSans-SemiBold`
is now bundled alongside the Regular (same family, same OFL 1.1 licence). A card title is one step up
in size and one step up in weight, instead of three steps up in size.

**Space** — 4 · 8 · 12 · 16 · 24 · 32. Everything is a multiple of 4.

The rule matters more than the numbers: the space **around** a group is at least twice the space
**inside** it. Equal spacing inside and between groups makes it impossible to tell which label belongs
to which field, and that is a functional defect, not a matter of taste.

**Radius** — one value, 6. Plus a pill, which is a shape and not a second radius: a badge drawn at 6px
reads as a small rectangle rather than a tag.

**Elevation** — five steps, chosen by where a thing sits on the z axis. The old buttons carried a
shadow with an offset and no blur at all, which is a hard black step rather than a shadow, and it is
what made every button look pasted onto the screen.

**Controls** — one height, 32. A size variant changes the width and the type, never the height.

## How to go back

Version 2 was not a file. It was those literals, spread across the screens, so there is no flag to
flip: **the way back is `git revert` of the commit that introduced `src/gui/design.rs`** and the
sweep that followed it.

What the system does guarantee from here on is that the foundation moves in **one** place. No screen
reads a bare number any more, so changing `design::text::BODY` changes the whole app and nothing else
has to be hunted down. If you find yourself typing a number into a screen file, that is the bug.

## Emulator logos

`assets/emulators/` holds each emulator project's own icon, and `assets/emulators/SOURCES.md` records
the URL and the licence for every one of them. **Read that file before publishing a release**: five of
the seven are GPL and one is CC BY-NC-ND, which are conditions Save Vault's own MIT licence does not
carry. Swapping to marks of our own is a change to one function, `logo()` in
`src/gui/emulator_art.rs`.
