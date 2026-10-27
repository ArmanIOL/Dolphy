# Dolphy

A GTK4 file manager for Linux, inspired by KDE Dolphin.

Dolphy is an independent GTK4 implementation, not a port of Dolphin's Qt/KDE code.
It uses GTK4 and GIO, so it integrates with the GNOME stack while keeping the
navigation model familiar to Dolphin users.

App ID: `io.github.ArmanIOL.Dolphy`

## Current status

Early prototype. The window opens in the home folder and can browse directories.

Implemented:

- Path bar with an editable location field (`Ctrl+L`)
- File and folder list for the current directory
- Folders first, case-insensitive name sorting
- Back / forward / up / home navigation
- History per window
- Hidden files toggle (`Ctrl+H`)
- Open folders on double-click or `Enter`
- Open files with the default application through `GtkFileLauncher`
- Item count and file sizes in the status bar

## Build

Requirements: Rust toolchain, GTK4 development files.

On Arch Linux:

```bash
sudo pacman -S --needed rust gtk4
```

Build and run:

```bash
cargo build
cargo run
```

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+L` | Focus the location bar |
| `Alt+Left` | Back |
| `Alt+Right` | Forward |
| `Alt+Up` | Parent folder |
| `Alt+Home` | Home folder |
| `Ctrl+H` | Show/hide hidden files |
| `F5` | Refresh |

## Roadmap

- Icons and details views with `GtkColumnView` / `GtkGridView`
- Tabs and split view
- File operations: copy, move, rename, trash, new folder
- Preview and properties panels
- Search and filtering
- Places sidebar with bookmarks and mounted volumes

## License

GPL-3.0-or-later
