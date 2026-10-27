#!/usr/bin/env bash
# Build a self-contained AppDir and pack it into an AppImage.
#
# This build targets x86_64 Arch Linux and relies on the host GTK4 runtime
# (gtk4 package). Bundling the full GTK stack is intentionally left to a later
# release; see README.md.

set -euo pipefail

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
APP_ID="io.github.ArmanIOL.Dolphy"
APP_NAME="Dolphy"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT_DIR/Cargo.toml" | head -n1)
DIST_DIR="$ROOT_DIR/dist"
APPDIR="$DIST_DIR/$APP_NAME.AppDir"
OUTPUT="$DIST_DIR/$APP_NAME-$VERSION-x86_64.AppImage"

if ! pkg-config --exists gtk4 2>/dev/null; then
    echo "error: GTK4 development files are missing. Install them with:" >&2
    echo "  sudo pacman -S --needed gtk4 base-devel" >&2
    exit 1
fi

echo "Building Dolphy $VERSION…"
(cd "$ROOT_DIR" && cargo build --release --locked)

rm -rf "$APPDIR"
mkdir -p \
    "$APPDIR/usr/bin" \
    "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/icons/hicolor/scalable/apps" \
    "$APPDIR/usr/share/metainfo"

install -m755 "$ROOT_DIR/target/release/dolphy" "$APPDIR/usr/bin/dolphy"
install -m644 "$ROOT_DIR/data/$APP_ID.desktop" "$APPDIR/usr/share/applications/$APP_ID.desktop"
install -m644 "$ROOT_DIR/data/$APP_ID.svg" "$APPDIR/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
install -m644 "$ROOT_DIR/data/$APP_ID.metainfo.xml" "$APPDIR/usr/share/metainfo/$APP_ID.metainfo.xml"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE=$(dirname "$(readlink -f "$0")")
export PATH="$HERE/usr/bin:$PATH"
export XDG_DATA_DIRS="$HERE/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
exec "$HERE/usr/bin/dolphy" "$@"
EOF
chmod +x "$APPDIR/AppRun"

# appimagetool expects the desktop file and icon in the AppDir root as well.
ln -sf "usr/share/applications/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
ln -sf "usr/share/icons/hicolor/scalable/apps/$APP_ID.svg" "$APPDIR/$APP_ID.svg"

if command -v appimagetool >/dev/null 2>&1; then
    APPIMAGETOOL=$(command -v appimagetool)
else
    APPIMAGETOOL="$DIST_DIR/appimagetool-x86_64.AppImage"
    if [ ! -x "$APPIMAGETOOL" ]; then
        echo "Downloading appimagetool…"
        curl -fL --retry 3 \
            "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" \
            -o "$APPIMAGETOOL"
        chmod +x "$APPIMAGETOOL"
    fi
fi

export ARCH=x86_64
export APPIMAGE_EXTRACT_AND_RUN=1

echo "Packing AppImage…"
"$APPIMAGETOOL" "$APPDIR" "$OUTPUT"

echo "AppImage created: $OUTPUT"
