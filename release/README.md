# release/

Everything that belongs to a GitHub release. The folder is not embedded in the
app; it is assembled by `scripts/build-release.sh` (Linux) and
`scripts/build-windows.ps1` / `.github/workflows/release.yml` (Windows).

The end-user README with screenshots lives at [`../README.md`](../README.md).

## Contents

| File | Purpose | Upload to GitHub? |
|---|---|---|
| `seedfinder-0.2.2-linux-x86_64.tar.gz` | Program, icon, menu entry, `install.sh`, README, screenshots, licenses | yes |
| `windows/SeedFinder_0.2.2_x64-setup.exe` | Windows installer, built on Windows or by CI | yes |
| `SHA256SUMS.txt` | SHA-256 of the Linux tarball | yes |
| `windows/SHA256SUMS-windows.txt` | SHA-256 of the Windows installer | yes |
| `RELEASE_NOTES.md` | Text for the release body (Markdown) | no, paste it into the form |
| `LICENSES.md` | License and attribution notes, also inside the tarball | optional |
| `screenshots/` | Images used by `README.md` and the release body | no |
| `README.md` | this overview | no |

`windows/` only exists after a Windows build. This checkout is built on Linux,
so the installer itself cannot be produced here; see below.

## Publish a release

1. Tag and push, e.g. `v0.2.2`.
2. GitHub → Releases → “Draft a new release”: tag `v0.2.2`, title
   `SeedFinder 0.2.2`.
3. Paste the content of `RELEASE_NOTES.md` into the release body.
4. Attach `seedfinder-0.2.2-linux-x86_64.tar.gz`, `SHA256SUMS.txt`, and the
   Windows installer with its checksum file.

The included workflow (`.github/workflows/release.yml`) does steps 1–4 for you:
on a `v*` tag it builds the Linux tarball and the Windows NSIS installer and
opens a draft release with all assets.

## Rebuild

Linux:

```bash
npm run build
cd src-tauri && cargo build --release && cd ..
bash scripts/build-release.sh
```

The script reads the version from `package.json`, packs the binary with the
icon, desktop entry, `install.sh`, `README.md`, `screenshots/` and
`LICENSES.md`, and rewrites `SHA256SUMS.txt`. It does not touch
`RELEASE_NOTES.md` or this file, so update the version, highlights and checksum
there by hand.

Windows (on Windows):

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-windows.ps1
```

The installer lands in `release/windows/` together with its SHA-256.
