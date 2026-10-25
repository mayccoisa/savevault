# Roots
Roots are folders that Ludusavi can check for additional game data. When you
first run Ludusavi, it will try to find some common roots on your system, but
you may end up without any configured. These are listed on the "other" screen,
where you can use the plus button in the roots section to configure as many as you need,
along with the root's type:

* For a Steam root, this should be the folder containing the `steamapps` and
  `userdata` subdirectories. Here are some common/standard locations:
  * Windows: `C:/Program Files (x86)/Steam`
  * Linux: `~/.steam/steam`

  On Linux, for games that use Proton, Ludusavi will back up the `*.reg` files
  if the game is known to have registry-based saves.

  On Linux, if you've used Steam's "add a non-Steam game" feature,
  then Ludusavi will also back up any Proton save data for those games.
  This requires the shortcut name in Steam to match the title by which Ludusavi knows the game
  (i.e., the title of its PCGamingWiki article).
* For a Heroic root, this should be the folder containing the `gog_store`
  and `GamesConfig` subdirectories.

  Ludusavi can find GOG, Epic, Amazon, and sideloaded game saves in Heroic's game install folders.
  On Linux, Ludusavi can also find saves in Heroic's Wine, Proton, and Lutris prefixes.

  When using Wine prefixes with Heroic, Ludusavi will back up the `*.reg` files
  if the game is known to have registry-based saves.
* For a Legendary root, this should be the folder containing `installed.json`.
  Currently, Ludusavi cannot detect Wine prefixes for Legendary roots.
* For a Lutris root, this should be the folder containing the `games` subdirectory.

  Ludusavi expects the game YAML files to contain a few fields,
  particularly `name` and either `game.working_dir` or `game.exe`.
  Games will be skipped if they don't have the necessary fields.
* For the "other" root type and the remaining store-specific roots,
  this should be a folder whose direct children are individual games.
  For example, in the Epic Games store, this would be what you choose as the
  "install location" for your games (e.g., if you choose `D:/Epic` and it
  creates a subfolder for `D:/Epic/Celeste`, then the root would be `D:/Epic`).
* For a home folder root, you may specify any folder. Whenever Ludusavi
  normally checks your standard home folder (Windows: `%USERPROFILE%`,
  Linux/Mac: `~`), it will additionally check this root. This is useful if
  you set a custom `HOME` to manipulate the location of save data.
* For a Wine prefix root, this should be the folder containing `drive_c`.
  Currently, Ludusavi does not back up registry-based saves from the prefix,
  but will back up any file-based saves.
* The Windows, Linux, and Mac drive roots can be used
  to make Ludusavi scan external hard drives with a separate OS installation.
  For example, let's say you had a Windows laptop that broke,
  but you recovered the hard drive and turned it into an external drive.
  You could add it as a Windows drive root to make Ludusavi scan it.

  In this case, Ludusavi can only look for normal/default locations of system folders.
  Ludusavi will not be able to use the Windows API or check `XDG` environment variables
  to detect alternative folder locations (e.g., if you've moved the `Documents` folder).

You may use [globs] in root paths to identify multiple roots at once.
If you have a folder name that contains a special glob character,
you can escape it by wrapping it in brackets (e.g., `[` becomes `[[]`).

You may also use the placeholder `<game>` as part of a root path.
In this case, for each game, Ludusavi will try replacing `<game>`
with the game's title and any known install folder names from the manifest.
For example, you may want to use this feature if you have a collection of
game-specific Wine prefixes, so you could add a single Wine root entry
with a path like `~/prefixes/<game>`.
This has an advantage over the glob version `~/prefixes/*`
because the glob would cause every prefix to be scanned for every game
(resulting in a slower scan and potentially finding false positives),
whereas `<game>` would only scan the prefix that applies to each game.

The order of the configured roots is not significant.
The only case where it may make a difference is if Ludusavi finds secondary manifests (`.ludusavi.yaml` files)
*and* those manfiests contain overlapping entries for the same game,
in which case Ludusavi will merge the data together in the order that it finds them.

## Emulator roots

An emulator root is that emulator's **data folder**, not necessarily the folder that holds the
program. SaveVault tells them apart with each emulator's folder signature:

* PPSSPP's data folder is the memory stick (`memstick`, with `PSP` inside it), not the folder
  that contains the executable. A non-portable install keeps that memory stick in Documents.
* A portable shadPS4 keeps data in the `user` folder next to the executable.
* RPCS3 keeps data in a `portable` folder when that folder exists, and otherwise in the
  folder that contains the executable.
* DuckStation, PCSX2, Xenia, Eden, and Sudachi are recognized when the folder you point at
  (or a folder inside it) matches their signature. Eden and Sudachi share a layout; the folder
  name (`eden`, `sudachi`) is what tells them apart.

On the Emulators screen, **Scan a folder** accepts a parent directory (for example `D:/Emulators`)
or a single emulator's folder. The scan is bounded: it does not walk the whole drive. It then
asks before writing anything.

When one data folder is found for an emulator, confirming adds it as an emulator root.
When none is found, nothing is invented. When two or more match the same emulator, or one
folder matches two emulators and its name does not say which, nothing is chosen — add the
folder you want with **Add folder** on that emulator's card.

The same walk is available from the command line:

```
ludusavi emulators --scan D:/Emulators
ludusavi emulators --scan D:/Emulators --apply
```

`--apply` writes only the unambiguous folders. Ambiguous matches are printed and left untouched.
Add `--api` for JSON.

[globs]: https://en.wikipedia.org/wiki/Glob_(programming)
