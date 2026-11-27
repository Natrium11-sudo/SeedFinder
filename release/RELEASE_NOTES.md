# SeedFinder 0.2.1

Find Minecraft Java seeds locally – a Chunkbase-style map and your own rules,
with a multithreaded search over random 64-bit seeds. No cloud, no Minecraft
installation, no network access at runtime.

## Highlights

- **Surface map from above:** the world is sampled at Y 300 (Y 255 before
  1.18), so the map shows the naturally generated surface without caves.
- **Underground layer:** switch to cave biomes (lush caves, dripstone caves,
  deep dark) at any Y height, with a height slider.
- **Structure layer:** villages, monuments, mansions, outposts, temples, huts,
  igloos, portals, ancient cities, trial chambers, camps, shipwrecks and
  treasure – each type individually toggled on the map, plus strongholds and
  slime chunks.
- **Custom scan size:** check any number of seeds up to 1 trillion (presets up
  to 1 billion), with rules compiled once per scan for less per-seed overhead.
- **New rules:** approximate spawn surface category and end portal eye count,
  with 4+/5+/6+ eye searches and verified 12-eye seed presets for the
  single-seed checker.
- **Coordinates and seeds copy in one click:** spawn coordinates, the position
  under the cursor, and the seed itself.
- **English UI**, all fonts bundled locally.
- Versions 1.6.4 through 26.3, including 1.21.5–1.21.11 and 26.1–26.3.

## Assets

| File | Contents |
|---|---|
| `SeedFinder_0.2.1_amd64.AppImage` | Portable Linux x86_64 application (needs FUSE 2) |
| `SHA256SUMS-appimage.txt` | SHA-256 of the AppImage |
| `seedfinder-0.2.1-linux-x86_64.tar.gz` | Program, icon, menu entry, installer, README with screenshots, licenses |
| `SHA256SUMS.txt` | SHA-256 of the Linux tarball |
| `SeedFinder_0.2.1_x64-setup.exe` | Windows x86_64 NSIS installer |
| `SHA256SUMS-windows.txt` | SHA-256 of the Windows installer |

The source is MIT-licensed (see `LICENSE`); third-party notices are in
`release/LICENSES.md`. Screenshots are embedded in `README.md` and bundled with
the Linux tarball.

Screenshots are in `release/screenshots/` in the repository and inside the
Linux tarball.

## Install (Linux x86_64)

```bash
tar -xzf seedfinder-0.2.1-linux-x86_64.tar.gz
cd seedfinder-0.2.1-linux-x86_64
./install.sh
```

This installs without root:

| What | Path |
|---|---|
| Program | `~/.local/bin/seedfinder` |
| Icon | `~/.local/share/icons/hicolor/512x512/apps/seedfinder.png` |
| Menu entry | `~/.local/share/applications/seedfinder.desktop` |

Uninstall by deleting those three files.

## Install (Windows x86_64)

Run the installer and follow the wizard. Requirements: Windows 10/11 with the
WebView2 runtime (included in current Windows builds).

## Verify

```bash
sha256sum -c SHA256SUMS.txt
```

Windows: `Get-FileHash .\SeedFinder_0.2.1_x64-setup.exe -Algorithm SHA256`.

**Linux requirements:** x86_64 Linux with glibc plus WebKitGTK 4.1, GTK 3 and
libsoup 3 – Debian/Ubuntu `libwebkit2gtk-4.1-0 libgtk-3-0 libsoup-3.0-0`,
Fedora `webkit2gtk4.1 gtk3 libsoup3`, Arch `webkit2gtk-4.1 gtk3 libsoup3`.

## Known limits

- The spawn and the heights are cubiomes approximations.
- The underground layer samples one fixed Y height; at 1:16 and 1:64 cubiomes
  rounds it to 16 or 64 blocks. Cave biomes start at 1.18 – older versions show
  the surface on that layer.
- Structures follow cubiomes, not a running game. Dense kinds are thinned out
  on very large map extents.
- At most 100 hits per search.
- Only the Linux build is verified here; the Windows installer is built on
  Windows/CI and has not been tested on this machine.
- Not included: Nether/End maps, 3D relief, PNG export, further UI languages.

## Licenses

cubiomes and its loot library are MIT-licensed. Map marker sprites come from
the Minecraft Wiki and are Mojang property; they are bundled locally only.
Full texts in `LICENSES.md`. SeedFinder is not affiliated with Mojang or
Microsoft.
