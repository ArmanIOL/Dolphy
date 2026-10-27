# Dolphy

A GTK4 file manager for Linux, inspired by KDE Dolphin.

Dolphy is an independent GTK4 implementation, not a port of Dolphin's Qt/KDE
code. It reproduces Dolphin's navigation model and layout while using the
GTK4/GIO stack.

App ID: `io.github.ArmanIOL.Dolphy` · Version: `0.2.0`

## Current status

This release focuses on a high-fidelity Dolphin-style layout:

- Dark Breeze-like palette
- Toolbar with back / forward / up buttons
- Clickable breadcrumb path (`endeavouros ▸ opt ▸ zapret2` style)
- “Две панели”, search and hamburger menu buttons
- Sidebar with Dolphin groups:
  - Точки входа: Домашняя папка, Рабочий стол, Документы, Загрузки, Музыка,
    Изображения, Видеофайлы, Корзина, Steam
  - В сети: Сеть
  - Недавно изменённые: Последние файлы, Последние расположения
  - Устройства: Basic data partition, endeavouros
- `GtkColumnView` with three columns: Имя, Размер, Дата изменения
- Folder contents counted as `N объектов`, file sizes as `5.4 КиБ`
- Status bar: selected item details on the left, item count and total size on
  the right
- Right-click context menu on file rows: open in new tab / new window, cut,
  copy, move to trash
- Default application launch through `GtkFileLauncher`

The context-menu actions and a few sidebar entries are intentionally
interactive stubs. Split view, tabs, search and real file operations are the
next milestones.

## Requirements

- Rust toolchain (`rustc`, `cargo`)
- GTK4 development files (GTK 4.20 or newer)

On Arch Linux:

```bash
sudo pacman -S --needed rust base-devel gtk4
```

## Build and run

```bash
cargo build
cargo run
```

The application opens in the current working directory.

## Install

The helper script installs into `~/.local` by default:

```bash
./install.sh
```

System-wide installation:

```bash
./install.sh --system
```

Remove an installation:

```bash
./install.sh --uninstall
```

Options: `--no-build`, `--prefix=/some/prefix`, `--help`.

## Pacman package

A PKGBUILD is provided in `packaging/`. It builds the current checkout, so it
works with private repositories and does not need a release tag.

```bash
cd packaging
makepkg -f          # build dolphy-0.2.0-1-x86_64.pkg.tar.zst
sudo pacman -U dolphy-0.2.0-1-x86_64.pkg.tar.zst
```

Or build and install in one step:

```bash
cd packaging && makepkg -si
```

## AppImage

```bash
./packaging/build-appimage.sh
```

The result is written to `dist/Dolphy-0.2.0-x86_64.AppImage`.

This AppImage bundles the Dolphy binary, desktop file, icon and AppStream
metadata, but it uses the host GTK4 runtime. Install `gtk4` on the target
system before running it. A fully self-contained GTK bundle is planned for a
later release.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Alt+Left` | Back |
| `Alt+Right` | Forward |
| `Alt+Up` | Parent folder |
| `Ctrl+H` | Show/hide hidden files |
| `Ctrl+F` | Search (stub) |
| `Ctrl+N` | New window |
| `F5` | Refresh |

## Project layout

```
src/main.rs                 monolithic GTK4 UI and file listing logic
data/                       desktop file, icon, AppStream metadata
packaging/PKGBUILD          local pacman package
packaging/build-appimage.sh AppImage builder
install.sh                  user/system installer
```

## Roadmap

- Icons and compact view modes
- Tabs and real split view
- File operations: copy, move, rename, trash, new folder, progress and conflicts
- Preview and properties panels
- Search and filtering
- Places editing, recent files, mounted volumes from GIO
- Fully self-contained AppImage and Flatpak builds

## License

GPL-3.0-or-later. See `LICENSE`.
