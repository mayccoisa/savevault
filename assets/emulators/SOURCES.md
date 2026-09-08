# Emulator logos

Each file is the emulator project's own icon, taken from that project's own repository. Nothing
here was drawn by us and nothing came from a search result. Every licence below was read from the
project's own `LICENSE` file on the date at the bottom, not recalled.

| File | Emulator | Source | Licence that covers it |
|---|---|---|---|
| `duckstation.png` | DuckStation | `github.com/stenzek/duckstation` → `data/resources/images/duck.png` | **CC BY-NC-ND 4.0** (repository `LICENSE`) |
| `pcsx2.png` | PCSX2 | `github.com/PCSX2/pcsx2` → `bin/resources/icons/AppIconLarge.png` | GPL-3.0 |
| `xenia.png` | Xenia | `github.com/xenia-project/xenia` → `assets/icon/256.png` | **CC BY-SA 4.0** (`assets/icon/LICENSE`, which covers the icon specifically, not the BSD code around it) |
| `eden.svg` | Eden | `git.eden-emu.dev/eden-emu/eden` → `dist/dev.eden_emu.eden.svg` | GPL-3.0 |
| `ppsspp.svg` | PPSSPP | `github.com/hrydgard/ppsspp` → `icons/icon-512.svg` | GPL-2.0-or-later (`LICENSE.TXT` also carries a PSPSDK BSD notice for other parts) |
| `rpcs3.svg` | RPCS3 | `github.com/RPCS3/rpcs3` → `rpcs3/rpcs3.svg` | GPL-2.0 |
| `shadps4.svg` | shadPS4 | `github.com/shadps4-emu/shadPS4` → `src/dist/net.shadps4.shadPS4.svg` | GPL-2.0 |

Downloaded on 2026-09-07.

## Read this before publishing a release with these files in it

Save Vault ships under MIT. These seven files do not, and two of them are the awkward ones:

- **DuckStation is CC BY-NC-ND.** NonCommercial, and NoDerivatives. Putting it inside a binary that
  gets distributed is a derivative-work and distribution question, and "NonCommercial" is a
  condition MIT does not carry.
- **Five of the seven are GPL** (2.0 or 3.0), which attaches conditions to redistribution that an
  MIT project does not otherwise have.
- Xenia is the cleanest of the set: CC BY-SA, and the project put that licence on the icon folder
  deliberately, so the terms for the artwork are explicit.

None of this is a technical problem and none of it changes how the screen works. It is a decision
about what goes into a published build, and it belongs to whoever publishes it.

**Swapping to marks of our own costs one function**: `logo()` in `src/gui/emulator_art.rs`. The
tile, the layout and the fallback glyph stay exactly as they are.

## Sudachi has no file here

Sudachi's repository was taken down, so there is no upstream artwork left to point at. Its card
falls back to the generic controller glyph that any emulator without artwork gets. Drawing a logo
for someone else's project would be inventing an identity for them.

## What these logos are, and are not

They identify the emulator whose folder the user is pointing Save Vault at. They are not Save
Vault's marks and they are not endorsements. Each one stays under its own project's licence and
trademark. Replacing a file here means going back to that project's repository, not redrawing it.
