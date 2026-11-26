#!/usr/bin/env bash
# Rebuilds the contents of release/: the GitHub release tarball and
# SHA256SUMS.txt. RELEASE_NOTES.md and README.md in the folder are left alone.
set -euo pipefail

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="$(sed -n 's/^  "version": "\([^"]*\)",$/\1/p' package.json | head -n1)"
if [ -z "$version" ]; then
  echo "Could not read the version from package.json." >&2
  exit 1
fi

binary="src-tauri/target/release/seedfinder"
if [ ! -x "$binary" ]; then
  echo "Missing: $binary" >&2
  echo "Build it first: npm run build && cd src-tauri && cargo build --release" >&2
  exit 1
fi

name="seedfinder-$version-linux-x86_64"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
pkg="$stage/$name"
mkdir -p "$pkg"

install -m755 "$binary" "$pkg/seedfinder"
install -m644 src-tauri/icons/icon.png "$pkg/seedfinder.png"
mkdir -p "$pkg/screenshots"
install -m644 release/screenshots/*.png "$pkg/screenshots/"
install -m644 release/LICENSES.md "$pkg/LICENSES.md"
# The repository README points at release/screenshots/, the package keeps the
# images next to the README.
sed 's|release/screenshots/|screenshots/|g' README.md > "$pkg/README.md"
chmod 644 "$pkg/README.md"

cat > "$pkg/seedfinder.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=SeedFinder
GenericName=Minecraft Seed Finder
Comment=Local Minecraft Java seed finder
Exec=seedfinder
Icon=seedfinder
Terminal=false
Categories=Utility;
Keywords=minecraft;seed;world;
StartupNotify=true
StartupWMClass=SeedFinder
EOF

cat > "$pkg/install.sh" <<'EOF'
#!/usr/bin/env bash
# Installs SeedFinder for the current user under ~/.local, without root.
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"

install -Dm755 "$here/seedfinder" "$bin_dir/seedfinder"
install -Dm644 "$here/seedfinder.png" "$data_dir/icons/hicolor/512x512/apps/seedfinder.png"
install -Dm644 "$here/seedfinder.desktop" "$data_dir/applications/seedfinder.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$data_dir/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -q -t "$data_dir/icons/hicolor" || true
fi

echo "SeedFinder installed: $bin_dir/seedfinder"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "Note: $bin_dir is not on your PATH." ;;
esac
EOF
chmod 755 "$pkg/install.sh"

tar -C "$stage" --sort=name --owner=0 --group=0 --numeric-owner --mtime=@0 \
  -czf "release/$name.tar.gz" "$name"

(cd release && sha256sum "$name.tar.gz" > SHA256SUMS.txt)

echo "Done: release/$name.tar.gz"
cat release/SHA256SUMS.txt
