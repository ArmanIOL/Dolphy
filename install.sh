#!/usr/bin/env sh
# Dolphy installer.
#
# Default: install for the current user into ~/.local (no sudo required).
# System-wide: ./install.sh --system  (uses sudo when needed).
# Remove:      ./install.sh --uninstall [--system]

set -eu

APP_ID="io.github.ArmanIOL.Dolphy"
BIN_NAME="dolphy"
PREFIX="${PREFIX:-$HOME/.local}"
MODE="install"
DO_BUILD=1
USE_SYSTEM=0

print_help() {
    cat <<'EOF'
Usage: ./install.sh [OPTIONS]

Options:
  --system      Install into /usr/local (uses sudo if necessary)
  --prefix DIR  Install into a custom prefix (default: ~/.local)
  --no-build    Use an existing target/release/dolphy build
  --uninstall   Remove previously installed files
  -h, --help    Show this help

Examples:
  ./install.sh
  ./install.sh --system
  ./install.sh --uninstall
EOF
}

while [ $# -gt 0 ]; do
    case "$1" in
        --system)
            USE_SYSTEM=1
            PREFIX="/usr/local"
            shift
            ;;
        --prefix)
            if [ $# -lt 2 ]; then
                echo "error: --prefix requires a directory argument" >&2
                exit 2
            fi
            PREFIX="$2"
            shift 2
            ;;
        --prefix=*)
            PREFIX="${1#--prefix=}"
            shift
            ;;
        --no-build)
            DO_BUILD=0
            shift
            ;;
        --uninstall)
            MODE="uninstall"
            shift
            ;;
        -h|--help)
            print_help
            exit 0
            ;;
        *)
            echo "error: unknown option: $1" >&2
            print_help >&2
            exit 2
            ;;
    esac
done

ROOT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

if [ "$USE_SYSTEM" -eq 1 ] && [ "$(id -u)" -ne 0 ]; then
    SUDO="sudo"
else
    SUDO=""
fi

install_file() {
    src="$1"
    dst="$2"
    mode="$3"
    $SUDO install -Dm"$mode" "$src" "$dst"
}

remove_file() {
    $SUDO rm -f "$1"
}

if [ "$MODE" = "uninstall" ]; then
    remove_file "$PREFIX/bin/$BIN_NAME"
    remove_file "$PREFIX/share/applications/$APP_ID.desktop"
    remove_file "$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg"
    remove_file "$PREFIX/share/metainfo/$APP_ID.metainfo.xml"
    remove_file "$PREFIX/share/licenses/$BIN_NAME/LICENSE"

    if command -v update-desktop-database >/dev/null 2>&1; then
        $SUDO update-desktop-database -q "$PREFIX/share/applications" 2>/dev/null || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        $SUDO gtk-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi

    echo "Dolphy removed from $PREFIX"
    exit 0
fi

if [ "$DO_BUILD" -eq 1 ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo is not installed. Install it with: sudo pacman -S rust" >&2
        exit 1
    fi
    if ! pkg-config --exists gtk4 2>/dev/null; then
        echo "error: GTK4 development files are missing. Install them with:" >&2
        echo "  sudo pacman -S --needed gtk4 base-devel" >&2
        exit 1
    fi

    echo "Building Dolphy (release)…"
    (cd "$ROOT_DIR" && cargo build --release --locked)
fi

BINARY="$ROOT_DIR/target/release/$BIN_NAME"
if [ ! -x "$BINARY" ]; then
    echo "error: $BINARY not found. Run ./install.sh without --no-build first." >&2
    exit 1
fi

install_file "$BINARY" "$PREFIX/bin/$BIN_NAME" 755
install_file "$ROOT_DIR/data/$APP_ID.desktop" "$PREFIX/share/applications/$APP_ID.desktop" 644
install_file "$ROOT_DIR/data/$APP_ID.svg" "$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg" 644
install_file "$ROOT_DIR/data/$APP_ID.metainfo.xml" "$PREFIX/share/metainfo/$APP_ID.metainfo.xml" 644
install_file "$ROOT_DIR/LICENSE" "$PREFIX/share/licenses/$BIN_NAME/LICENSE" 644

if command -v update-desktop-database >/dev/null 2>&1; then
    $SUDO update-desktop-database -q "$PREFIX/share/applications" 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    $SUDO gtk-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

echo "Dolphy installed into $PREFIX"
echo "Launch it with: $PREFIX/bin/$BIN_NAME"
