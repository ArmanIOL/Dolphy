use std::path::PathBuf;

/// Custom entries store an icon name from the edit dialog. Dolphin offers an
/// icon picker, so an unknown name cannot normally reach the model; fall back
/// to a folder instead of rendering the "missing image" icon when the config
/// was edited by hand or the selected theme lacks the icon.
pub(crate) fn resolved_icon_name(name: &str) -> String {
    let Some(display) = gtk::gdk::Display::default() else {
        return String::from("folder");
    };
    if gtk::IconTheme::for_display(&display).has_icon(name) {
        name.to_string()
    } else {
        String::from("folder")
    }
}

pub(crate) fn icon_theme_root_exists(theme: &str) -> bool {
    let mut roots = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").filter(|dir| !dir.is_empty()) {
        roots.push(PathBuf::from(data_home));
    } else {
        roots.push(glib::home_dir().join(".local").join("share"));
    }
    roots.push(glib::home_dir().join(".icons"));
    let data_dirs = std::env::var_os("XDG_DATA_DIRS")
        .filter(|dirs| !dirs.is_empty())
        .map(|dirs| dirs.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("/usr/local/share:/usr/share"));
    roots.extend(
        data_dirs
            .split(':')
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from),
    );

    roots
        .into_iter()
        .any(|root| root.join(theme).join("index.theme").is_file())
}

/// Dolphin uses the system icon theme. When Breeze is installed, use it so
/// that the Places panel and file view match the original appearance.
pub(crate) fn preferred_icon_theme() -> Option<&'static str> {
    if icon_theme_root_exists("breeze-dark") {
        Some("breeze-dark")
    } else if icon_theme_root_exists("breeze") {
        Some("breeze")
    } else {
        None
    }
}
