use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use gio::prelude::*;
use glib::variant::ToVariant;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, ColumnView, ColumnViewColumn,
    CssProvider, Entry, EntryIconPosition, EventControllerKey, HeaderBar, Image, Label, ListBox,
    ListBoxRow, MenuButton, Orientation, Paned, PickFlags, Popover, PopoverMenu, ProgressBar,
    ScrolledWindow, SelectionMode, Separator, SingleSelection, Stack, ToggleButton,
};
use pango::EllipsizeMode;

const APP_ID: &str = "io.github.ArmanIOL.Dolphy";
const APP_NAME: &str = "Dolphy";
const VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_SIDEBAR_WIDTH: i32 = 235;
const MIN_SIDEBAR_WIDTH: i32 = 90;
const DEFAULT_WINDOW_WIDTH: i32 = 1100;
const DEFAULT_WINDOW_HEIGHT: i32 = 750;
const DEFAULT_ICON_SIZE: i32 = 22;
const RECENT_LIMIT: usize = 12;

const DEFAULT_PLACE_IDS: [&str; 8] = [
    "home",
    "desktop",
    "documents",
    "downloads",
    "music",
    "pictures",
    "videos",
    "trash",
];

/// Dark Breeze-like palette, close to the Dolphin reference screenshots.
const APP_CSS: &str = r#"
window {
    background-color: #1e1f22;
    color: #d7d8da;
}

headerbar {
    background-color: #2a2b2f;
    min-height: 36px;
}

headerbar .title {
    font-weight: 600;
}

.toolbar {
    background-color: #25262a;
    padding: 3px 6px;
}

.toolbar button {
    min-width: 26px;
    min-height: 26px;
    padding: 2px 5px;
}

.toolbar menubutton > button {
    min-width: 26px;
    min-height: 26px;
}

.pathbar {
    background-color: transparent;
    padding: 2px 0;
}

.pathbar button {
    min-height: 26px;
    padding: 2px 5px;
    border-radius: 4px;
}

.pathbar .path-edit-button {
    min-width: 26px;
}

.breadcrumb button {
    padding: 2px 6px;
    border-radius: 4px;
    min-height: 24px;
}

.breadcrumb button:hover {
    background-color: rgba(255, 255, 255, 0.08);
}

.crumb-separator {
    color: #7f8287;
    margin: 0 1px;
}

.path-entry {
    min-height: 26px;
    background-color: transparent;
    border: none;
    box-shadow: none;
}

.path-entry.error {
    box-shadow: inset 0 0 0 1px #da4453;
}

.path-field {
    background-color: #1a1b1e;
    border-radius: 4px;
    padding: 1px 2px;
}

.path-field scrolledwindow {
    background-color: transparent;
}

.pathbar .crumb-filler {
    min-width: 0;
    padding: 0;
    margin: 0;
    background-color: transparent;
    border: none;
    box-shadow: none;
}

.pathbar .crumb-filler:hover {
    background-color: transparent;
}

.sidebar {
    background-color: #25262a;
    padding: 0;
}

.sidebar row {
    min-height: 24px;
    padding: 3px 4px;
    margin: 0;
    border-radius: 0;
}

.sidebar row:hover {
    background-color: rgba(255, 255, 255, 0.05);
}

.sidebar row:selected {
    background-color: #3daee9;
    color: #ffffff;
}

.sidebar row:selected label {
    color: #ffffff;
}

.sidebar row.drop-target {
    background-color: rgba(61, 174, 233, 0.25);
}

.sidebar .group-label {
    color: #8d9095;
    font-size: 0.9em;
    font-weight: 400;
    margin: 8px 2px 2px 2px;
}

.sidebar .place-line {
    min-height: 22px;
}

.sidebar progressbar.capacity-bar {
    min-height: 3px;
    margin: 1px 4px 2px 0;
}

.sidebar progressbar.capacity-bar trough {
    min-height: 3px;
    border-radius: 2px;
    background-color: #3a3b40;
}

.sidebar progressbar.capacity-bar trough > progress {
    min-height: 3px;
    border-radius: 2px;
    background-color: #3daee9;
}

.places-popover,
.recent-popover {
    background-color: #2a2b2f;
}

.places-list row,
.recent-list row {
    min-height: 28px;
    padding: 3px 8px;
    border-radius: 4px;
}

.places-list row:hover,
.recent-list row:hover {
    background-color: rgba(255, 255, 255, 0.06);
}

.popover-section {
    color: #8d9095;
    font-size: 0.82em;
    font-weight: 700;
    margin: 8px 10px 3px 10px;
}

.menu-row {
    min-height: 28px;
    padding: 4px 10px;
    border-radius: 4px;
}

.menu-row:hover {
    background-color: rgba(255, 255, 255, 0.07);
}

.menu-row:disabled {
    opacity: 0.45;
}

.menu-row.danger label {
    color: #ef7d84;
}

.menu-row .menu-check {
    color: #3daee9;
    font-weight: 700;
}

.menu-separator {
    background-color: #3a3b40;
    min-height: 1px;
    margin: 5px 8px;
}

.menu-empty {
    color: #9a9da2;
    padding: 10px 12px;
}

.add-place-error {
    color: #ef7d84;
}

.recent-name {
    font-weight: 600;
}

.recent-path {
    color: #9a9da2;
    font-size: 0.85em;
}

columnview {
    background-color: #1e1f22;
}

columnview > header {
    background-color: #2a2b2f;
}

columnview > header button {
    border-radius: 0;
    background-color: transparent;
}

columnview row {
    min-height: 24px;
}

columnview cell {
    padding: 0 4px;
}

.statusbar {
    background-color: #25262a;
    padding: 4px 10px;
}

.dim-label {
    color: #9a9da2;
}
"#;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_interface_color_scheme(gtk::InterfaceColorScheme::Dark);
        if let Some(theme) = preferred_icon_theme() {
            settings.set_gtk_icon_theme_name(Some(theme));
        }
    }

    static CSS_INIT: std::sync::Once = std::sync::Once::new();
    CSS_INIT.call_once(install_css);

    let settings = Rc::new(RefCell::new(Settings::load()));
    Browser::new(app, settings);
}

/// Custom entries store an icon name from the edit dialog. Dolphin offers an
/// icon picker, so an unknown name cannot normally reach the model; fall back
/// to a folder instead of rendering the "missing image" icon when the config
/// was edited by hand or the selected theme lacks the icon.
fn resolved_icon_name(name: &str) -> String {
    let Some(display) = gtk::gdk::Display::default() else {
        return String::from("folder");
    };
    if gtk::IconTheme::for_display(&display).has_icon(name) {
        name.to_string()
    } else {
        String::from("folder")
    }
}

fn icon_theme_root_exists(theme: &str) -> bool {
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
fn preferred_icon_theme() -> Option<&'static str> {
    if icon_theme_root_exists("breeze-dark") {
        Some("breeze-dark")
    } else if icon_theme_root_exists("breeze") {
        Some("breeze")
    } else {
        None
    }
}

fn install_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = CssProvider::new();
    provider.load_from_string(APP_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

/// Detaches every child widget. Unparenting is used instead of container-specific
/// removal so that it stays correct even if a widget has already been reparented.
fn detach_children(container: &impl IsA<gtk::Widget>) {
    let mut child = container.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        current.unparent();
    }
}

// ---------------------------------------------------------------------------
// Persistent settings
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SectionId {
    Places,
    Remote,
    Recent,
    Devices,
}

impl SectionId {
    const ALL: [Self; 4] = [Self::Places, Self::Remote, Self::Recent, Self::Devices];

    fn key(self) -> &'static str {
        match self {
            Self::Places => "places",
            Self::Remote => "remote",
            Self::Recent => "recent",
            Self::Devices => "devices",
        }
    }

    fn title(self) -> &'static str {
        match self {
            // Exact KIO Russian translations of KFilePlacesModel's group
            // names ("Places", "Remote", "Recent", "Devices").
            Self::Places => "Точки входа",
            Self::Remote => "В сети",
            Self::Recent => "Недавно изменённые",
            Self::Devices => "Устройства",
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        match key {
            "places" => Some(Self::Places),
            "remote" => Some(Self::Remote),
            "recent" => Some(Self::Recent),
            "devices" => Some(Self::Devices),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomPlace {
    id: String,
    name: String,
    path: PathBuf,
    icon: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Settings {
    sidebar_visible: bool,
    sidebar_width: i32,
    sidebar_icon_size: i32,
    hidden_sections: Vec<SectionId>,
    places_order: Vec<String>,
    hidden_places: Vec<String>,
    custom_places: Vec<CustomPlace>,
    edited_places: Vec<CustomPlace>,
    next_custom_id: u32,
    show_hidden: bool,
    show_full_path: bool,
    window_width: i32,
    window_height: i32,
    last_location: Option<PathBuf>,
    recent_locations: Vec<PathBuf>,
    recent_files: Vec<PathBuf>,
}

impl Settings {
    fn defaults() -> Self {
        let mut settings = Self {
            sidebar_visible: true,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_icon_size: DEFAULT_ICON_SIZE,
            hidden_sections: Vec::new(),
            places_order: DEFAULT_PLACE_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect(),
            hidden_places: Vec::new(),
            custom_places: Vec::new(),
            edited_places: Vec::new(),
            next_custom_id: 1,
            show_hidden: false,
            show_full_path: false,
            window_width: DEFAULT_WINDOW_WIDTH,
            window_height: DEFAULT_WINDOW_HEIGHT,
            last_location: None,
            recent_locations: Vec::new(),
            recent_files: Vec::new(),
        };
        settings.normalize();
        settings
    }

    fn config_path() -> PathBuf {
        if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
            PathBuf::from(dir).join("dolphy").join("dolphy.conf")
        } else {
            glib::home_dir()
                .join(".config")
                .join("dolphy")
                .join("dolphy.conf")
        }
    }

    fn load() -> Self {
        Self::load_from_path(&Self::config_path())
    }

    fn load_from_path(path: &Path) -> Self {
        let key_file = glib::KeyFile::new();
        if let Err(error) = key_file.load_from_file(path, glib::KeyFileFlags::NONE) {
            if path.is_file() {
                backup_config(path);
                glib::g_warning!(
                    "Dolphy",
                    "файл настроек {} повреждён или недоступен ({}); используются значения по умолчанию",
                    path.display(),
                    error
                );
            }
            return Self::defaults();
        }

        let version = key_file_string(&key_file, "General", "version");
        if !version.is_empty() && version != "1" {
            if path.is_file() {
                backup_config(path);
            }
            glib::g_warning!(
                "Dolphy",
                "несовместимая версия настроек {} ({}); используются значения по умолчанию",
                path.display(),
                version
            );
            return Self::defaults();
        }

        let mut settings = Self::defaults();
        settings.sidebar_visible = key_file_bool(&key_file, "Sidebar", "visible", true);
        settings.sidebar_width = key_file_i32(&key_file, "Sidebar", "width", DEFAULT_SIDEBAR_WIDTH);
        settings.sidebar_icon_size =
            key_file_i32(&key_file, "Sidebar", "icon_size", DEFAULT_ICON_SIZE);
        settings.show_hidden = key_file_bool(&key_file, "General", "show_hidden", false);
        settings.show_full_path = key_file_bool(&key_file, "General", "show_full_path", false);
        settings.window_width = key_file_i32(&key_file, "Window", "width", DEFAULT_WINDOW_WIDTH);
        settings.window_height = key_file_i32(&key_file, "Window", "height", DEFAULT_WINDOW_HEIGHT);

        settings.hidden_sections = read_indexed(&key_file, "Sidebar", "hidden_section")
            .into_iter()
            .filter_map(|key| SectionId::from_key(&key))
            .collect();
        settings.places_order = read_indexed(&key_file, "Sidebar", "place_order");
        settings.hidden_places = read_indexed(&key_file, "Sidebar", "hidden_place");

        let custom_count = key_file_i32(&key_file, "Sidebar", "custom_count", 0).clamp(0, 200);
        for index in 0..custom_count {
            let id = key_file_string(&key_file, "Sidebar", &format!("custom_{index}_id"));
            let name = key_file_string(&key_file, "Sidebar", &format!("custom_{index}_name"));
            let path = key_file_string(&key_file, "Sidebar", &format!("custom_{index}_path"));
            let icon = key_file_string(&key_file, "Sidebar", &format!("custom_{index}_icon"));
            if !path.is_empty() {
                settings.custom_places.push(CustomPlace {
                    id,
                    name,
                    path: PathBuf::from(path),
                    icon: if icon.trim().is_empty() {
                        String::from("folder")
                    } else {
                        icon
                    },
                });
            }
        }
        settings.next_custom_id =
            key_file_i32(&key_file, "Sidebar", "next_custom_id", 1).max(1) as u32;

        let edited_count = key_file_i32(&key_file, "Sidebar", "edited_count", 0).clamp(0, 200);
        for index in 0..edited_count {
            let id = key_file_string(&key_file, "Sidebar", &format!("edited_{index}_id"));
            let name = key_file_string(&key_file, "Sidebar", &format!("edited_{index}_name"));
            let path = key_file_string(&key_file, "Sidebar", &format!("edited_{index}_path"));
            let icon = key_file_string(&key_file, "Sidebar", &format!("edited_{index}_icon"));
            if !path.is_empty() {
                settings.edited_places.push(CustomPlace {
                    id,
                    name,
                    path: PathBuf::from(path),
                    icon: if icon.trim().is_empty() {
                        String::from("folder")
                    } else {
                        icon
                    },
                });
            }
        }

        let last_location = key_file_string(&key_file, "General", "last_location");
        settings.last_location = if last_location.is_empty() {
            None
        } else {
            Some(PathBuf::from(last_location))
        };

        settings.recent_locations = read_indexed(&key_file, "Recents", "location")
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .collect();
        settings.recent_files = read_indexed(&key_file, "Recents", "file")
            .into_iter()
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .collect();

        settings.normalize();
        settings
    }

    fn save(&self) -> std::io::Result<()> {
        self.save_to_path(&Self::config_path())
    }

    fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        let key_file = glib::KeyFile::new();
        key_file.set_string("General", "version", "1");
        key_file.set_boolean("General", "show_hidden", self.show_hidden);
        key_file.set_boolean("General", "show_full_path", self.show_full_path);
        key_file.set_string(
            "General",
            "last_location",
            &self
                .last_location
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );

        key_file.set_boolean("Sidebar", "visible", self.sidebar_visible);
        key_file.set_integer("Sidebar", "width", self.sidebar_width);
        key_file.set_integer("Sidebar", "icon_size", self.sidebar_icon_size);
        write_indexed(
            &key_file,
            "Sidebar",
            "hidden_section",
            &self
                .hidden_sections
                .iter()
                .map(|section| section.key().to_string())
                .collect::<Vec<_>>(),
        );
        write_indexed(&key_file, "Sidebar", "place_order", &self.places_order);
        write_indexed(&key_file, "Sidebar", "hidden_place", &self.hidden_places);
        key_file.set_integer("Sidebar", "custom_count", self.custom_places.len() as i32);
        for (index, place) in self.custom_places.iter().enumerate() {
            key_file.set_string("Sidebar", &format!("custom_{index}_id"), &place.id);
            key_file.set_string("Sidebar", &format!("custom_{index}_name"), &place.name);
            key_file.set_string(
                "Sidebar",
                &format!("custom_{index}_path"),
                &place.path.to_string_lossy(),
            );
            key_file.set_string("Sidebar", &format!("custom_{index}_icon"), &place.icon);
        }
        key_file.set_integer("Sidebar", "next_custom_id", self.next_custom_id as i32);
        key_file.set_integer("Sidebar", "edited_count", self.edited_places.len() as i32);
        for (index, place) in self.edited_places.iter().enumerate() {
            key_file.set_string("Sidebar", &format!("edited_{index}_id"), &place.id);
            key_file.set_string("Sidebar", &format!("edited_{index}_name"), &place.name);
            key_file.set_string(
                "Sidebar",
                &format!("edited_{index}_path"),
                &place.path.to_string_lossy(),
            );
            key_file.set_string("Sidebar", &format!("edited_{index}_icon"), &place.icon);
        }

        key_file.set_integer("Window", "width", self.window_width);
        key_file.set_integer("Window", "height", self.window_height);

        write_indexed(
            &key_file,
            "Recents",
            "location",
            &self
                .recent_locations
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        );
        write_indexed(
            &key_file,
            "Recents",
            "file",
            &self
                .recent_files
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
        );

        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "нет родительского каталога",
            )
        })?;
        std::fs::create_dir_all(parent)?;

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dolphy.conf");
        let temporary = path.with_file_name(format!(".{file_name}.tmp"));
        std::fs::write(&temporary, key_file.to_data().as_bytes())?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    }

    fn normalize(&mut self) {
        if self.sidebar_width <= 0 {
            self.sidebar_width = DEFAULT_SIDEBAR_WIDTH;
        }
        self.sidebar_width = self.sidebar_width.clamp(MIN_SIDEBAR_WIDTH, 4000);
        if !matches!(self.sidebar_icon_size, 16 | 22 | 32 | 48) {
            self.sidebar_icon_size = DEFAULT_ICON_SIZE;
        }
        if self.window_width < 480 {
            self.window_width = DEFAULT_WINDOW_WIDTH;
        }
        if self.window_height < 320 {
            self.window_height = DEFAULT_WINDOW_HEIGHT;
        }
        self.window_width = self.window_width.min(10000);
        self.window_height = self.window_height.min(10000);

        self.custom_places
            .retain(|place| !place.path.as_os_str().is_empty());
        let mut used_ids = HashSet::new();
        let mut next_id = self.next_custom_id.max(1);
        for place in &self.custom_places {
            if let Some(number) = place
                .id
                .strip_prefix("custom:")
                .and_then(|value| value.parse::<u32>().ok())
            {
                next_id = next_id.max(number.saturating_add(1));
            }
        }
        let system_ids: HashSet<&str> = DEFAULT_PLACE_IDS.iter().copied().collect();
        for place in &mut self.custom_places {
            if place.name.trim().is_empty() {
                place.name = place
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| place.path.to_string_lossy().into_owned());
            } else {
                place.name = place.name.trim().to_string();
            }
            place.icon = place.icon.trim().to_string();
            if place.icon.is_empty() {
                place.icon = String::from("folder");
            }
            if place.id.trim().is_empty()
                || used_ids.contains(&place.id)
                || system_ids.contains(place.id.as_str())
            {
                loop {
                    let candidate = format!("custom:{next_id}");
                    next_id = next_id.saturating_add(1).max(1);
                    if !used_ids.contains(&candidate) {
                        place.id = candidate;
                        break;
                    }
                }
            }
            used_ids.insert(place.id.clone());
        }
        self.next_custom_id = next_id;

        // System bookmarks are edited in place: KFilePlacesModel modifies the
        // bookmark entry itself instead of creating a new one. Store the edited
        // values separately so the fixed defaults stay intact and can be
        // overridden per place id.
        let mut edited_ids = HashSet::new();
        self.edited_places.retain(|place| {
            !place.path.as_os_str().is_empty()
                && system_ids.contains(place.id.as_str())
                && edited_ids.insert(place.id.clone())
        });
        for place in &mut self.edited_places {
            if place.name.trim().is_empty() {
                place.name = place
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| place.path.to_string_lossy().into_owned());
            } else {
                place.name = place.name.trim().to_string();
            }
            place.icon = place.icon.trim().to_string();
            if place.icon.is_empty() {
                place.icon = String::from("folder");
            }
        }

        let mut hidden_sections = Vec::new();
        for section in &self.hidden_sections {
            if !hidden_sections.contains(section) {
                hidden_sections.push(*section);
            }
        }
        self.hidden_sections = hidden_sections;

        let mut valid_places = DEFAULT_PLACE_IDS
            .iter()
            .map(|id| (*id).to_string())
            .collect::<Vec<_>>();
        valid_places.extend(self.custom_places.iter().map(|place| place.id.clone()));
        let orderable_places = valid_places.clone();
        valid_places.extend([
            String::from("network"),
            String::from("recent-files"),
            String::from("recent-locations"),
        ]);

        let mut places = Vec::new();
        for id in &self.places_order {
            if orderable_places.contains(id) && !places.contains(id) {
                places.push(id.clone());
            }
        }
        for id in &orderable_places {
            if !places.contains(id) {
                places.push(id.clone());
            }
        }
        self.places_order = places;

        let mut hidden_places = Vec::new();
        for id in &self.hidden_places {
            let known = valid_places.contains(id) || id.starts_with("device:");
            if known && !hidden_places.contains(id) {
                hidden_places.push(id.clone());
            }
        }
        self.hidden_places = hidden_places;

        self.recent_locations = deduplicate_paths(&self.recent_locations, RECENT_LIMIT);
        self.recent_files = deduplicate_paths(&self.recent_files, RECENT_LIMIT);
        if self
            .last_location
            .as_ref()
            .is_some_and(|path| path.as_os_str().is_empty())
        {
            self.last_location = None;
        }
    }
}

fn key_file_bool(key_file: &glib::KeyFile, group: &str, key: &str, default: bool) -> bool {
    key_file.boolean(group, key).unwrap_or(default)
}

fn backup_config(path: &Path) {
    let backup = path.with_file_name(format!(
        "{}.bak",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dolphy.conf")
    ));
    let _ = std::fs::rename(path, &backup);
}

fn key_file_i32(key_file: &glib::KeyFile, group: &str, key: &str, default: i32) -> i32 {
    key_file.integer(group, key).unwrap_or(default)
}

fn key_file_string(key_file: &glib::KeyFile, group: &str, key: &str) -> String {
    key_file
        .string(group, key)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn read_indexed(key_file: &glib::KeyFile, group: &str, prefix: &str) -> Vec<String> {
    let count = key_file_i32(key_file, group, &format!("{prefix}_count"), 0).clamp(0, 500);
    (0..count)
        .filter_map(|index| {
            key_file
                .string(group, &format!("{prefix}_{index}"))
                .ok()
                .map(|value| value.to_string())
        })
        .collect()
}

fn write_indexed(key_file: &glib::KeyFile, group: &str, prefix: &str, values: &[String]) {
    key_file.set_integer(group, &format!("{prefix}_count"), values.len() as i32);
    for (index, value) in values.iter().enumerate() {
        key_file.set_string(group, &format!("{prefix}_{index}"), value);
    }
}

fn deduplicate_paths(paths: &[PathBuf], limit: usize) -> Vec<PathBuf> {
    let mut result = Vec::new();
    for path in paths {
        if path.as_os_str().is_empty() || result.contains(path) {
            continue;
        }
        result.push(path.clone());
        if result.len() == limit {
            break;
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Path handling
// ---------------------------------------------------------------------------

fn resolve_path_text(input: &str, current: &Path, home: &Path) -> Result<PathBuf, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(String::from("Путь не может быть пустым."));
    }

    let raw = if trimmed == "~" {
        home.to_path_buf()
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        home.join(rest)
    } else if trimmed.starts_with("file://") {
        gio::File::for_uri(trimmed)
            .path()
            .ok_or_else(|| String::from("Поддерживаются только локальные пути."))?
    } else if trimmed.contains("://") {
        return Err(String::from("Поддерживаются только локальные пути."));
    } else {
        PathBuf::from(trimmed)
    };

    let absolute = if raw.is_absolute() {
        raw
    } else {
        current.join(raw)
    };

    absolute.canonicalize().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => format!("Папка не найдена: {}", absolute.display()),
        std::io::ErrorKind::PermissionDenied => format!("Нет доступа: {}", absolute.display()),
        _ => format!("Не удалось открыть {}: {error}", absolute.display()),
    })
}

fn remember_path(list: &mut Vec<PathBuf>, path: &Path, limit: usize) {
    list.retain(|entry| entry != path);
    list.insert(0, path.to_path_buf());
    list.truncate(limit);
}

#[derive(Clone)]
struct BreadcrumbAnchor {
    label: String,
    path: PathBuf,
    icon: String,
}

fn path_anchors(settings: &Settings, devices: &[DeviceSpec]) -> Vec<BreadcrumbAnchor> {
    let mut anchors = Vec::new();

    // The navigator's place selector is not tied to the visibility of the
    // Places panel or to individual entries hidden from the panel.
    for place in ordered_places(settings) {
        if let PlaceTarget::Directory(path) = &place.target {
            anchors.push(BreadcrumbAnchor {
                label: place.label.clone(),
                path: path.clone(),
                icon: place.icon.clone(),
            });
        }
    }

    let network = network_place();
    if let PlaceTarget::Directory(path) = &network.target {
        if path.is_dir() {
            anchors.push(BreadcrumbAnchor {
                label: network.label.clone(),
                path: path.clone(),
                icon: network.icon.clone(),
            });
        }
    }

    for device in devices {
        anchors.push(BreadcrumbAnchor {
            label: device.label.clone(),
            path: device.mount.clone(),
            icon: String::from("drive-harddisk"),
        });
    }

    anchors
}

fn build_breadcrumbs(
    path: &Path,
    anchors: &[BreadcrumbAnchor],
    show_full_path: bool,
) -> Vec<(String, PathBuf)> {
    let mut best: Option<&BreadcrumbAnchor> = None;
    if !show_full_path {
        for anchor in anchors {
            if anchor.path == Path::new("/") || !path.starts_with(&anchor.path) {
                continue;
            }
            let deeper = best.is_none_or(|current| {
                anchor.path.components().count() > current.path.components().count()
            });
            if deeper {
                best = Some(anchor);
            }
        }
    }

    let (start_label, start_path) = best
        .map(|anchor| (anchor.label.clone(), anchor.path.clone()))
        .unwrap_or_else(|| (String::from("/"), PathBuf::from("/")));
    let mut result = vec![(start_label, start_path.clone())];
    if start_path == path {
        return result;
    }

    let mut target = start_path;
    let relative = path.strip_prefix(&target).unwrap_or(path);
    for component in relative.components() {
        if let Component::Normal(name) = component {
            target.push(name);
            result.push((name.to_string_lossy().into_owned(), target.clone()));
        }
    }
    result
}

fn current_place_icon(path: &Path, anchors: &[BreadcrumbAnchor]) -> String {
    let mut best: Option<&BreadcrumbAnchor> = None;
    for anchor in anchors {
        if !path.starts_with(&anchor.path) {
            continue;
        }
        let deeper = best.is_none_or(|current| {
            anchor.path.components().count() >= current.path.components().count()
        });
        if deeper {
            best = Some(anchor);
        }
    }
    best.map(|anchor| anchor.icon.clone())
        .unwrap_or_else(|| String::from("folder"))
}

// ---------------------------------------------------------------------------
// Navigation state
// ---------------------------------------------------------------------------

struct Navigator {
    history: Vec<PathBuf>,
    position: usize,
    show_hidden: bool,
}

impl Navigator {
    fn new(start: PathBuf, show_hidden: bool) -> Self {
        Self {
            history: vec![start],
            position: 0,
            show_hidden,
        }
    }

    fn current(&self) -> &Path {
        &self.history[self.position]
    }

    fn navigate_to(&mut self, path: PathBuf) {
        if self.current() == path {
            return;
        }
        self.history.truncate(self.position + 1);
        self.history.push(path);
        self.position = self.history.len() - 1;
    }

    fn go_back(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }
        self.position -= 1;
        true
    }

    fn go_forward(&mut self) -> bool {
        if self.position + 1 >= self.history.len() {
            return false;
        }
        self.position += 1;
        true
    }

    fn go_up(&mut self) -> bool {
        let Some(parent) = self.current().parent().map(Path::to_path_buf) else {
            return false;
        };
        self.navigate_to(parent);
        true
    }

    fn can_go_back(&self) -> bool {
        self.position > 0
    }

    fn can_go_forward(&self) -> bool {
        self.position + 1 < self.history.len()
    }

    fn can_go_up(&self) -> bool {
        self.current().parent().is_some()
    }
}

// ---------------------------------------------------------------------------
// File data
// ---------------------------------------------------------------------------

struct FileData {
    path: PathBuf,
    name: String,
    kind_text: String,
    size_text: String,
    modified_text: String,
    icon: gio::Icon,
}

fn item_data(object: &glib::Object) -> Option<Rc<FileData>> {
    let boxed = object.downcast_ref::<glib::BoxedAnyObject>()?;
    let borrowed = boxed.borrow::<Rc<FileData>>();
    Some((*borrowed).clone())
}

fn describe(data: &FileData) -> String {
    format!("{} ({}, {})", data.name, data.kind_text, data.size_text)
}

// ---------------------------------------------------------------------------
// Places, devices and breadcrumb anchors
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum PlaceTarget {
    Directory(PathBuf),
    RecentFiles,
    RecentLocations,
    Unavailable(String),
}

#[derive(Clone)]
struct PlaceSpec {
    id: String,
    label: String,
    icon: String,
    target: PlaceTarget,
    custom: bool,
    capacity: Option<f64>,
    tooltip: Option<String>,
}

fn special_dir(directory: glib::UserDirectory, fallback: &str) -> PathBuf {
    glib::user_special_dir(directory).unwrap_or_else(|| glib::home_dir().join(fallback))
}

fn trash_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/share"))
        .join("Trash")
        .join("files")
}

fn trash_is_empty() -> bool {
    std::fs::read_dir(trash_dir())
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true)
}

fn network_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // The current process always has a real uid; this is only a fallback
            // for environments where XDG_RUNTIME_DIR is not exported.
            PathBuf::from(format!("/run/user/{}", libc_uid()))
        })
        .join("gvfs")
}

fn libc_uid() -> u32 {
    // glib exposes the real user name and home directory, but not the numeric uid.
    // gvfs lives in /run/user/<uid>; retrieving the uid via /proc/self/status
    // avoids pulling in an extra dependency.
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("Uid:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|uid| uid.parse::<u32>().ok())
            })
        })
        .unwrap_or(1000)
}

fn network_place() -> PlaceSpec {
    let path = network_path();
    let target = if path.is_dir() {
        PlaceTarget::Directory(path)
    } else {
        PlaceTarget::Unavailable(String::from("Сеть недоступна: каталог gvfs не найден."))
    };
    PlaceSpec {
        id: String::from("network"),
        label: String::from("Сеть"),
        icon: String::from("folder-network"),
        target,
        custom: false,
        capacity: None,
        tooltip: None,
    }
}

fn directory_place(id: &str, label: &str, icon: &str, path: PathBuf) -> PlaceSpec {
    PlaceSpec {
        id: String::from(id),
        label: String::from(label),
        icon: String::from(icon),
        target: PlaceTarget::Directory(path),
        custom: false,
        capacity: None,
        tooltip: None,
    }
}

fn default_places(settings: &Settings) -> Vec<PlaceSpec> {
    let home = glib::home_dir();
    let trash_icon = if trash_is_empty() {
        "user-trash"
    } else {
        "user-trash-full"
    };

    // KFilePlacesModel only creates the standard bookmarks when the
    // corresponding XDG directory exists.
    let mut places = vec![directory_place("home", "Домашняя папка", "user-home", home)];
    for (id, label, icon, directory, fallback) in [
        (
            "desktop",
            "Рабочий стол",
            "user-desktop",
            glib::UserDirectory::Desktop,
            "Desktop",
        ),
        (
            "documents",
            "Документы",
            "folder-documents",
            glib::UserDirectory::Documents,
            "Documents",
        ),
        (
            "downloads",
            "Загрузки",
            "folder-downloads",
            glib::UserDirectory::Downloads,
            "Downloads",
        ),
        (
            "music",
            "Музыка",
            "folder-music",
            glib::UserDirectory::Music,
            "Music",
        ),
        (
            "pictures",
            "Изображения",
            "folder-pictures",
            glib::UserDirectory::Pictures,
            "Pictures",
        ),
        (
            "videos",
            "Видеофайлы",
            "folder-videos",
            glib::UserDirectory::Videos,
            "Videos",
        ),
    ] {
        let path = special_dir(directory, fallback);
        if path.is_dir() {
            places.push(directory_place(id, label, icon, path));
        }
    }
    places.push(directory_place("trash", "Корзина", trash_icon, trash_dir()));

    for edited in &settings.edited_places {
        if let Some(place) = places.iter_mut().find(|place| place.id == edited.id) {
            place.label = edited.name.clone();
            place.icon = edited.icon.clone();
            place.target = PlaceTarget::Directory(edited.path.clone());
            place.tooltip = Some(edited.path.display().to_string());
        }
    }

    for custom in &settings.custom_places {
        places.push(PlaceSpec {
            id: custom.id.clone(),
            label: custom.name.clone(),
            icon: custom.icon.clone(),
            target: PlaceTarget::Directory(custom.path.clone()),
            custom: true,
            capacity: None,
            tooltip: Some(custom.path.display().to_string()),
        });
    }

    places
}

fn ordered_places(settings: &Settings) -> Vec<PlaceSpec> {
    let base = default_places(settings);
    let mut result = Vec::new();
    let mut used = HashSet::new();

    for id in &settings.places_order {
        if let Some(place) = base
            .iter()
            .find(|place| &place.id == id && !used.contains(id))
        {
            result.push(place.clone());
            used.insert(id.clone());
        }
    }
    for place in &base {
        if used.insert(place.id.clone()) {
            result.push(place.clone());
        }
    }
    result
}

#[derive(Clone)]
struct DeviceSpec {
    mount: PathBuf,
    label: String,
    used_fraction: Option<f64>,
    tooltip: String,
}

#[derive(Clone)]
struct RawMount {
    source: String,
    target: PathBuf,
    fstype: String,
}

fn parse_mounts(contents: &str) -> Vec<RawMount> {
    contents
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let source = fields.next()?;
            let target = fields.next()?;
            let fstype = fields.next()?;
            Some(RawMount {
                source: source.to_string(),
                target: PathBuf::from(unescape_mount_field(target)),
                fstype: fstype.to_string(),
            })
        })
        .collect()
}

fn unescape_mount_field(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() {
            let octal = &value[index + 1..index + 4];
            if octal.bytes().all(|byte| (b'0'..=b'7').contains(&byte)) {
                if let Ok(decoded) = u8::from_str_radix(octal, 8) {
                    result.push(decoded);
                    index += 4;
                    continue;
                }
            }
        }
        result.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn decode_by_label(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() && bytes[index + 1] == b'x' {
            let hex = &value[index + 2..index + 4];
            if let Ok(decoded) = u8::from_str_radix(hex, 16) {
                result.push(decoded);
                index += 4;
                continue;
            }
        }
        result.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn filesystem_labels() -> HashMap<PathBuf, String> {
    let mut labels = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/dev/disk/by-label") {
        for entry in entries.flatten() {
            let name = decode_by_label(&entry.file_name().to_string_lossy());
            if let Ok(target) = std::fs::canonicalize(entry.path()) {
                labels.insert(target, name);
            }
        }
    }
    labels
}

fn partition_name(source: &Path) -> Option<String> {
    let canonical = std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let name = canonical.file_name()?.to_string_lossy();
    let contents = std::fs::read_to_string(format!("/sys/class/block/{name}/uevent")).ok()?;
    contents.lines().find_map(|line| {
        line.strip_prefix("PARTNAME=")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn gio_mount_names() -> HashMap<PathBuf, String> {
    let mut names = HashMap::new();
    for mount in gio::VolumeMonitor::get().mounts() {
        if let Some(path) = mount.root().path() {
            let name = mount.name().to_string();
            if !name.is_empty() {
                names.insert(path, name);
            }
        }
    }
    names
}

fn is_pseudo_filesystem(fstype: &str) -> bool {
    matches!(
        fstype,
        "proc"
            | "sysfs"
            | "devtmpfs"
            | "devpts"
            | "tmpfs"
            | "cgroup"
            | "cgroup2"
            | "pstore"
            | "efivarfs"
            | "bpf"
            | "configfs"
            | "debugfs"
            | "hugetlbfs"
            | "mqueue"
            | "tracefs"
            | "securityfs"
            | "autofs"
            | "overlay"
            | "squashfs"
            | "swap"
    )
}

fn is_system_mount(path: &Path) -> bool {
    path == Path::new("/boot") || path == Path::new("/boot/efi")
}

fn filesystem_usage(path: &Path) -> Option<(u64, u64)> {
    let file = gio::File::for_path(path);
    let info = file
        .query_filesystem_info(
            "filesystem::size,filesystem::free",
            None::<&gio::Cancellable>,
        )
        .ok()?;
    let total = info.attribute_uint64("filesystem::size");
    if total == 0 {
        return None;
    }
    let free = info.attribute_uint64("filesystem::free").min(total);
    Some((total - free, total))
}

fn device_spec(mount: PathBuf, label: String) -> DeviceSpec {
    let usage = filesystem_usage(&mount);
    let (used_fraction, tooltip) = match usage {
        Some((used, total)) => (
            Some((used as f64 / total as f64).clamp(0.0, 1.0)),
            format!(
                "{}\n{}\nИспользовано {} из {}",
                label,
                mount.display(),
                human_size(used),
                human_size(total)
            ),
        ),
        None => (None, format!("{}\n{}", label, mount.display())),
    };
    DeviceSpec {
        mount,
        label,
        used_fraction,
        tooltip,
    }
}

fn collect_devices() -> Vec<DeviceSpec> {
    let labels = filesystem_labels();
    let gio_names = gio_mount_names();
    let mut by_source: HashMap<PathBuf, RawMount> = HashMap::new();

    if let Ok(contents) = std::fs::read_to_string("/proc/self/mounts") {
        for mount in parse_mounts(&contents) {
            if !mount.source.starts_with("/dev/")
                || is_pseudo_filesystem(&mount.fstype)
                || is_system_mount(&mount.target)
            {
                continue;
            }
            let canonical = std::fs::canonicalize(&mount.source)
                .unwrap_or_else(|_| PathBuf::from(&mount.source));
            let replace = by_source.get(&canonical).is_none_or(|existing| {
                mount.target.components().count() < existing.target.components().count()
            });
            if replace {
                by_source.insert(canonical, mount);
            }
        }
    }

    let mut devices = Vec::new();
    for (canonical, mount) in by_source {
        let label = labels
            .get(&canonical)
            .cloned()
            .or_else(|| partition_name(&canonical))
            .or_else(|| gio_names.get(&mount.target).cloned());
        if label.is_none() && mount.target != Path::new("/") {
            continue;
        }
        let label = label.unwrap_or_else(|| String::from("Корень"));
        devices.push(device_spec(mount.target, label));
    }

    let existing: HashSet<PathBuf> = devices.iter().map(|device| device.mount.clone()).collect();
    for (path, name) in gio_names {
        if existing.contains(&path) || !path.is_dir() || is_system_mount(&path) {
            continue;
        }
        devices.push(device_spec(path, name));
    }

    devices.sort_by(|left, right| {
        let left_root = left.mount == Path::new("/");
        let right_root = right.mount == Path::new("/");
        left_root
            .cmp(&right_root)
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    devices
}

fn recent_files_place() -> PlaceSpec {
    PlaceSpec {
        id: String::from("recent-files"),
        label: String::from("Последние файлы"),
        icon: String::from("document-open-recent"),
        target: PlaceTarget::RecentFiles,
        custom: false,
        capacity: None,
        tooltip: None,
    }
}

fn recent_locations_place() -> PlaceSpec {
    PlaceSpec {
        id: String::from("recent-locations"),
        label: String::from("Последние расположения"),
        icon: String::from("folder-open-recent"),
        target: PlaceTarget::RecentLocations,
        custom: false,
        capacity: None,
        tooltip: None,
    }
}

fn device_places(devices: &[DeviceSpec]) -> Vec<PlaceSpec> {
    devices
        .iter()
        .map(|device| PlaceSpec {
            id: format!("device:{}", device.mount.display()),
            label: device.label.clone(),
            icon: String::from("drive-harddisk"),
            target: PlaceTarget::Directory(device.mount.clone()),
            custom: false,
            capacity: device.used_fraction,
            tooltip: Some(device.tooltip.clone()),
        })
        .collect()
}

fn places_for_section(
    settings: &Settings,
    devices: &[DeviceSpec],
    section: SectionId,
) -> Vec<PlaceSpec> {
    match section {
        SectionId::Places => ordered_places(settings),
        SectionId::Remote => vec![network_place()],
        SectionId::Recent => vec![recent_files_place(), recent_locations_place()],
        SectionId::Devices => device_places(devices),
    }
}

fn place_matches_path(place: &PlaceSpec, path: &Path) -> bool {
    match &place.target {
        PlaceTarget::Directory(place_path) => path.starts_with(place_path),
        _ => false,
    }
}

#[derive(Clone)]
struct SidebarRow {
    place: PlaceSpec,
    section: SectionId,
    header: Option<SectionId>,
    hidden: bool,
    section_hidden: bool,
}

fn compose_sidebar_rows(
    settings: &Settings,
    devices: &[DeviceSpec],
    show_all: bool,
    current: &Path,
) -> Vec<SidebarRow> {
    let mut rows = Vec::new();
    for section in SectionId::ALL {
        let section_hidden = settings.hidden_sections.contains(&section);
        let mut section_rows = Vec::new();

        for place in places_for_section(settings, devices, section) {
            let hidden = settings.hidden_places.contains(&place.id);
            let current_place = place_matches_path(&place, current);
            if section_hidden && !show_all && !current_place {
                continue;
            }
            if hidden && !show_all && !current_place {
                continue;
            }
            section_rows.push(SidebarRow {
                place,
                section,
                header: None,
                hidden,
                section_hidden,
            });
        }

        if let Some(first) = section_rows.first_mut() {
            first.header = Some(section);
        }
        rows.extend(section_rows);
    }
    rows
}

/// GTK also changes `GtkPaned::position` when the window is laid out or
/// clamped, and that must not be mistaken for a user resize. During a real
/// handle drag the primary mouse button is held down, which is visible on the
/// seat's pointer device independently of where the pointer currently is.
fn sidebar_resize_is_user_drag() -> bool {
    gtk::gdk::Display::default()
        .and_then(|display| display.default_seat())
        .and_then(|seat| seat.pointer())
        .is_some_and(|pointer| {
            pointer
                .modifier_state()
                .contains(gtk::gdk::ModifierType::BUTTON1_MASK)
        })
}

fn sidebar_width_from_paned_position(position: i32) -> Option<i32> {
    if position < MIN_SIDEBAR_WIDTH {
        None
    } else {
        Some(position.min(4000))
    }
}

/// Moves `dragged` before or after `target` in the persisted order used by
/// the Places section. IDs from the fixed sections (network, recents, devices)
/// are never part of this list.
fn reorder_place_ids(order: &mut Vec<String>, dragged: &str, target: &str, after: bool) -> bool {
    if dragged == target {
        return false;
    }
    let Some(dragged_index) = order.iter().position(|id| id == dragged) else {
        return false;
    };
    if !order.iter().any(|id| id == target) {
        return false;
    }
    let moved = order.remove(dragged_index);
    let target_index = order
        .iter()
        .position(|id| id == target)
        .expect("target checked above");
    let insert_at = if after {
        target_index + 1
    } else {
        target_index
    };
    order.insert(insert_at, moved);
    true
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

struct Sidebar {
    widget: ScrolledWindow,
    list: ListBox,
    rows: RefCell<Vec<SidebarRow>>,
}

fn build_sidebar() -> Sidebar {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .activate_on_single_click(false)
        .build();
    list.add_css_class("sidebar");

    let widget = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();
    widget.add_css_class("sidebar-scroller");

    Sidebar {
        widget,
        list,
        rows: RefCell::new(Vec::new()),
    }
}

impl Sidebar {
    fn render(&self, rows: &[SidebarRow], icon_size: i32) {
        detach_children(&self.list);

        for row in rows {
            let widget = build_sidebar_place_row(row, icon_size);
            self.list.append(&widget);
        }

        *self.rows.borrow_mut() = rows.to_vec();
    }

    fn row(&self, index: i32) -> Option<SidebarRow> {
        self.rows.borrow().get(index as usize).cloned()
    }
}

fn sidebar_click_over_header(row: &ListBoxRow, reference: &impl IsA<gtk::Widget>, y: f64) -> bool {
    let Some(content) = row.child() else {
        return false;
    };
    let Some(first) = content.first_child() else {
        return false;
    };
    if !first.has_css_class("group-label") {
        return false;
    }
    first
        .compute_bounds(reference)
        .is_some_and(|bounds| y >= bounds.y() as f64 && y < (bounds.y() + bounds.height()) as f64)
}

fn section_header_text(section: SectionId, hidden: bool) -> String {
    if hidden {
        format!("{} (скрыт)", section.title())
    } else {
        section.title().to_string()
    }
}

fn build_sidebar_header_label(section: SectionId, hidden: bool) -> Label {
    let text = section_header_text(section, hidden);
    let label = Label::new(Some(&text));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);
    label.add_css_class("group-label");
    label.set_tooltip_text(Some(section.title()));
    label
}

fn build_sidebar_place_row(row: &SidebarRow, icon_size: i32) -> ListBoxRow {
    let place = &row.place;
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);

    if let Some(section) = row.header {
        content.append(&build_sidebar_header_label(section, row.section_hidden));
    }

    let line = GtkBox::new(Orientation::Horizontal, 4);
    line.set_hexpand(true);
    line.add_css_class("place-line");

    let icon = Image::from_icon_name(&resolved_icon_name(&place.icon));
    icon.set_pixel_size(icon_size);
    icon.set_valign(gtk::Align::Center);

    let label = Label::new(Some(&place.label));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);

    line.append(&icon);
    line.append(&label);
    content.append(&line);

    if let Some(fraction) = place.capacity {
        let bar = ProgressBar::new();
        bar.set_show_text(false);
        bar.set_fraction(fraction);
        bar.set_valign(gtk::Align::Center);
        bar.add_css_class("capacity-bar");
        bar.set_margin_start(icon_size + 4);
        bar.set_margin_top(1);
        bar.set_margin_end(4);
        content.append(&bar);
    }

    let widget = ListBoxRow::new();
    widget.set_child(Some(&content));
    if row.hidden || row.section_hidden {
        widget.set_opacity(0.6);
    }
    let tooltip = place.tooltip.clone().unwrap_or_else(|| place.label.clone());
    widget.set_tooltip_text(Some(&tooltip));
    widget
}

// ---------------------------------------------------------------------------
// Path bar
// ---------------------------------------------------------------------------

struct PathBar {
    widget: GtkBox,
    places_button: MenuButton,
    places_popover: Popover,
    places_list: ListBox,
    places_targets: RefCell<Vec<Option<PlaceTarget>>>,
    stack: Stack,
    crumb_scroller: ScrolledWindow,
    crumbs: GtkBox,
    filler: Button,
    entry: Entry,
    edit_button: Button,
    editing: Cell<bool>,
}

fn build_path_bar() -> Rc<PathBar> {
    let places_list = ListBox::builder()
        .selection_mode(SelectionMode::None)
        .activate_on_single_click(true)
        .build();
    places_list.add_css_class("places-list");

    let places_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(260)
        .max_content_height(480)
        .propagate_natural_height(true)
        .child(&places_list)
        .build();

    let places_popover = Popover::new();
    places_popover.set_has_arrow(false);
    places_popover.add_css_class("places-popover");
    places_popover.set_child(Some(&places_scroller));

    let places_button = MenuButton::new();
    places_button.set_icon_name("user-home");
    places_button.set_popover(Some(&places_popover));
    places_button.set_always_show_arrow(false);
    places_button.set_tooltip_text(Some("Места"));
    places_button.add_css_class("flat");

    let crumbs = GtkBox::new(Orientation::Horizontal, 0);
    crumbs.add_css_class("breadcrumb");
    crumbs.set_hexpand(true);

    let filler = Button::new();
    filler.add_css_class("flat");
    filler.add_css_class("crumb-filler");
    filler.set_hexpand(true);
    filler.set_focusable(false);
    filler.set_tooltip_text(Some("Изменить путь (Ctrl+L)"));
    crumbs.append(&filler);

    let crumb_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .hexpand(true)
        .child(&crumbs)
        .build();

    let entry = Entry::builder()
        .hexpand(true)
        .width_chars(20)
        .has_frame(false)
        .placeholder_text("Введите путь")
        .build();
    entry.add_css_class("path-entry");
    entry.set_icon_from_icon_name(EntryIconPosition::Secondary, Some("edit-clear-symbolic"));
    entry.set_icon_activatable(EntryIconPosition::Secondary, true);
    entry.set_tooltip_text(Some("Введите путь и нажмите Enter; Esc отменяет ввод"));

    let stack = Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::None);
    stack.set_hhomogeneous(false);
    stack.set_vhomogeneous(false);
    stack.add_named(&crumb_scroller, Some("crumbs"));
    stack.add_named(&entry, Some("entry"));
    stack.set_visible_child_name("crumbs");
    stack.set_hexpand(true);

    let field = GtkBox::new(Orientation::Vertical, 0);
    field.add_css_class("path-field");
    field.set_hexpand(true);
    field.append(&stack);

    let edit_button = Button::from_icon_name("open-for-editing");
    edit_button.add_css_class("flat");
    edit_button.add_css_class("path-edit-button");
    edit_button.set_tooltip_text(Some("Редактировать путь"));

    let widget = GtkBox::new(Orientation::Horizontal, 4);
    widget.add_css_class("pathbar");
    widget.set_hexpand(true);
    widget.append(&places_button);
    widget.append(&field);
    widget.append(&edit_button);

    Rc::new(PathBar {
        widget,
        places_button,
        places_popover,
        places_list,
        places_targets: RefCell::new(Vec::new()),
        stack,
        crumb_scroller,
        crumbs,
        filler,
        entry,
        edit_button,
        editing: Cell::new(false),
    })
}

impl PathBar {
    fn set_place_icon(&self, icon_name: &str) {
        self.places_button
            .set_icon_name(&resolved_icon_name(icon_name));
    }

    fn set_place_tooltip(&self, label: &str) {
        self.places_button
            .set_tooltip_text(Some(&format!("Места — {label}")));
    }

    fn begin_edit(&self, path: &Path) {
        self.editing.set(true);
        self.entry.set_text(&path.to_string_lossy());
        self.entry.set_position(-1);
        self.stack.set_visible_child_name("entry");
        self.edit_button.set_icon_name("dialog-ok");
        self.edit_button
            .set_tooltip_text(Some("Показать панель перехода"));
        self.set_error(None);
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }

    fn end_edit(&self) {
        self.editing.set(false);
        self.stack.set_visible_child_name("crumbs");
        self.edit_button.set_icon_name("open-for-editing");
        self.edit_button
            .set_tooltip_text(Some("Редактировать путь"));
        self.set_error(None);
    }

    fn set_error(&self, message: Option<&str>) {
        if message.is_some() {
            self.entry.add_css_class("error");
        } else {
            self.entry.remove_css_class("error");
        }
        self.entry.set_tooltip_text(message);
    }

    fn is_editing(&self) -> bool {
        self.editing.get()
    }
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

struct Toolbar {
    widget: GtkBox,
    back: Button,
    forward: Button,
    up: Button,
    sidebar_toggle: Button,
    split: ToggleButton,
    menu_button: MenuButton,
    pathbar: Rc<PathBar>,
}

fn toolbar_button(icon_name: &str, tooltip: &str, action: &str) -> Button {
    let button = Button::from_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
    button.set_action_name(Some(action));
    button.add_css_class("flat");
    button
}

fn build_toolbar() -> Toolbar {
    let back = toolbar_button("go-previous", "Назад (Alt+Left)", "win.back");
    let forward = toolbar_button("go-next", "Вперёд (Alt+Right)", "win.forward");
    let up = toolbar_button("go-up", "На уровень выше (Alt+Up)", "win.up");

    let sidebar_toggle = Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some("Боковая панель (F9)"));
    sidebar_toggle.set_action_name(Some("win.toggle-sidebar"));
    sidebar_toggle.add_css_class("flat");

    let pathbar = build_path_bar();

    let split = ToggleButton::new();
    split.set_tooltip_text(Some("Две панели (скоро)"));
    split.add_css_class("flat");
    let split_content = GtkBox::new(Orientation::Horizontal, 6);
    let split_icon = Image::from_icon_name("view-split-left-right");
    split_icon.set_pixel_size(16);
    split_content.append(&split_icon);
    split_content.append(&Label::new(Some("Две панели")));
    split.set_child(Some(&split_content));

    let search = toolbar_button("system-search", "Поиск (Ctrl+F)", "win.search");

    let menu_button = MenuButton::new();
    menu_button.set_icon_name("application-menu");
    menu_button.set_tooltip_text(Some("Главное меню"));
    menu_button.set_always_show_arrow(false);
    menu_button.add_css_class("flat");

    let widget = GtkBox::new(Orientation::Horizontal, 4);
    widget.add_css_class("toolbar");
    widget.append(&sidebar_toggle);
    widget.append(&back);
    widget.append(&forward);
    widget.append(&up);
    widget.append(&pathbar.widget);
    widget.append(&split);
    widget.append(&search);
    widget.append(&menu_button);

    Toolbar {
        widget,
        back,
        forward,
        up,
        sidebar_toggle,
        split,
        menu_button,
        pathbar,
    }
}

// ---------------------------------------------------------------------------
// Browser window
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum RecentKind {
    Files,
    Locations,
}

struct Browser {
    window: ApplicationWindow,
    title_label: Label,
    toolbar: Toolbar,
    sidebar: Sidebar,
    paned: Paned,
    column_view: ColumnView,
    store: gio::ListStore,
    selection: SingleSelection,
    status_left: Label,
    status_right: Label,
    state: RefCell<Navigator>,
    settings: Rc<RefCell<Settings>>,
    devices: RefCell<Vec<DeviceSpec>>,
    popup: RefCell<Option<Popover>>,
    save_generation: Rc<Cell<u64>>,
    show_all_places: Cell<bool>,
    last_sidebar_mouse_press: Cell<Option<Instant>>,
}

impl Browser {
    fn new(app: &Application, settings: Rc<RefCell<Settings>>) -> Rc<Self> {
        let (start, window_width, window_height, sidebar_visible, sidebar_width, show_hidden) = {
            let settings = settings.borrow();
            let start = settings
                .last_location
                .clone()
                .filter(|path| path.is_dir())
                .unwrap_or_else(glib::home_dir);
            (
                start,
                settings.window_width,
                settings.window_height,
                settings.sidebar_visible,
                settings.sidebar_width,
                settings.show_hidden,
            )
        };

        let window = ApplicationWindow::builder()
            .application(app)
            .title(format!("{APP_NAME} — файловый менеджер"))
            .default_width(window_width)
            .default_height(window_height)
            .build();

        let header = HeaderBar::new();
        let title_label = Label::new(Some(APP_NAME));
        title_label.add_css_class("title");
        header.set_title_widget(Some(&title_label));
        window.set_titlebar(Some(&header));

        let toolbar = build_toolbar();
        let sidebar = build_sidebar();

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);

        let (column_view, view_scroller) = build_file_view(&selection);

        let main_area = GtkBox::new(Orientation::Vertical, 0);
        main_area.append(&view_scroller);

        let paned = Paned::new(Orientation::Horizontal);
        paned.set_start_child(Some(&sidebar.widget));
        paned.set_end_child(Some(&main_area));
        paned.set_position(sidebar_width);
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(true);
        paned.set_vexpand(true);
        sidebar.widget.set_size_request(MIN_SIDEBAR_WIDTH, -1);
        sidebar.widget.set_visible(sidebar_visible);

        let status_left = Label::new(Some("Ничего не выбрано"));
        status_left.set_xalign(0.0);
        status_left.set_hexpand(true);
        status_left.set_ellipsize(EllipsizeMode::End);
        status_left.add_css_class("dim-label");

        let status_right = Label::new(None);
        status_right.set_xalign(1.0);
        status_right.add_css_class("dim-label");

        let status_bar = GtkBox::new(Orientation::Horizontal, 12);
        status_bar.add_css_class("statusbar");
        status_bar.append(&status_left);
        status_bar.append(&status_right);

        let content = GtkBox::new(Orientation::Vertical, 0);
        content.append(&toolbar.widget);
        content.append(&Separator::new(Orientation::Horizontal));
        content.append(&paned);
        content.append(&Separator::new(Orientation::Horizontal));
        content.append(&status_bar);

        window.set_child(Some(&content));

        let browser = Rc::new(Self {
            window,
            title_label,
            toolbar,
            sidebar,
            paned,
            column_view,
            store,
            selection,
            status_left,
            status_right,
            state: RefCell::new(Navigator::new(start, show_hidden)),
            settings,
            devices: RefCell::new(Vec::new()),
            popup: RefCell::new(None),
            save_generation: Rc::new(Cell::new(0)),
            show_all_places: Cell::new(false),
            last_sidebar_mouse_press: Cell::new(None),
        });

        browser.connect_signals();
        browser.install_actions(app);
        browser.install_menu();
        browser.rebuild_sidebar();
        browser.refresh();
        browser.connect_width_persistence();
        browser.connect_close_persistence();
        browser.window.present();

        browser
    }

    fn connect_signals(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.sidebar.list.connect_row_activated(move |_, row| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            // A left click is handled by our own gesture, including the
            // "inert section header" rule. Suppress the activation that GTK
            // additionally emits for a double click or for a header row, but
            // keep keyboard activation working.
            if browser
                .last_sidebar_mouse_press
                .get()
                .is_some_and(|instant| instant.elapsed() < Duration::from_millis(600))
            {
                return;
            }
            browser.activate_sidebar_place(row.index());
        });

        let weak = Rc::downgrade(self);
        let context_gesture = gtk::GestureClick::new();
        context_gesture.set_button(3);
        context_gesture.connect_pressed(move |gesture, _, x, y| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let Some(widget) = gesture.widget() else {
                return;
            };
            let row_index = widget
                .downcast_ref::<ListBox>()
                .and_then(|list| list.row_at_y(y as i32))
                .map(|row| row.index());
            let over_header = widget
                .downcast_ref::<ListBox>()
                .and_then(|list| list.row_at_y(y as i32))
                .is_some_and(|row| {
                    widget
                        .downcast_ref::<ListBox>()
                        .is_some_and(|list| sidebar_click_over_header(&row, list, y))
                });
            browser.show_sidebar_context(row_index, over_header, x as i32, y as i32);
        });
        self.sidebar.list.add_controller(context_gesture);

        let status_left = self.status_left.clone();
        self.selection
            .connect_selection_changed(move |selection, _, _| {
                let text = selection
                    .selected_item()
                    .and_then(|item| item_data(&item))
                    .map(|data| describe(&data))
                    .unwrap_or_else(|| String::from("Ничего не выбрано"));
                status_left.set_text(&text);
            });

        let weak = Rc::downgrade(self);
        self.column_view.connect_activate(move |_, position| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let data = browser
                .store
                .item(position)
                .and_then(|item| item_data(&item));
            if let Some(data) = data {
                browser.open(&data.path);
            }
        });

        let weak = Rc::downgrade(self);
        self.toolbar.split.connect_toggled(move |button| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let text = if button.is_active() {
                "Две панели: режим появится в следующей версии"
            } else {
                "Однопанельный режим"
            };
            browser.status_left.set_text(text);
        });

        let weak = Rc::downgrade(self);
        let pathbar = Rc::clone(&self.toolbar.pathbar);
        self.toolbar.pathbar.filler.connect_clicked(move |_| {
            if let Some(browser) = weak.upgrade() {
                browser.begin_path_edit();
            }
        });

        let weak = Rc::downgrade(self);
        let places_popover = self.toolbar.pathbar.places_popover.clone();
        self.toolbar
            .pathbar
            .places_list
            .connect_row_activated(move |_, row| {
                let Some(browser) = weak.upgrade() else {
                    return;
                };
                let target = browser
                    .toolbar
                    .pathbar
                    .places_targets
                    .borrow()
                    .get(row.index() as usize)
                    .cloned()
                    .flatten();
                places_popover.popdown();
                match target {
                    Some(PlaceTarget::Directory(path)) => browser.navigate(path),
                    Some(PlaceTarget::Unavailable(message)) => {
                        browser.status_left.set_text(&message)
                    }
                    Some(PlaceTarget::RecentFiles) => {
                        browser.show_recents_popover(RecentKind::Files, None)
                    }
                    Some(PlaceTarget::RecentLocations) => {
                        browser.show_recents_popover(RecentKind::Locations, None)
                    }
                    None => {}
                }
            });

        let weak = Rc::downgrade(self);
        self.toolbar.pathbar.edit_button.connect_clicked(move |_| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            if browser.toolbar.pathbar.is_editing() {
                browser.submit_path_bar();
            } else {
                browser.begin_path_edit();
            }
        });

        let weak = Rc::downgrade(self);
        self.toolbar.pathbar.entry.connect_activate(move |_| {
            if let Some(browser) = weak.upgrade() {
                browser.submit_path_bar();
            }
        });

        let weak = Rc::downgrade(self);
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                if let Some(browser) = weak.upgrade() {
                    if browser.toolbar.pathbar.is_editing() {
                        browser.cancel_path_edit();
                        return glib::Propagation::Stop;
                    }
                }
            }
            glib::Propagation::Proceed
        });
        self.toolbar.pathbar.entry.add_controller(key_controller);

        let weak = Rc::downgrade(self);
        self.toolbar
            .pathbar
            .entry
            .connect_icon_release(move |entry, position| {
                if position != EntryIconPosition::Secondary {
                    return;
                }
                let Some(browser) = weak.upgrade() else {
                    return;
                };
                entry.set_text("");
                entry.grab_focus();
                browser.toolbar.pathbar.set_error(None);
            });

        self.toolbar.pathbar.entry.connect_changed(|entry| {
            entry.remove_css_class("error");
            entry.set_tooltip_text(Some("Введите путь и нажмите Enter; Esc отменяет ввод"));
        });

        let weak = Rc::downgrade(self);
        let gesture = gtk::GestureClick::new();
        let scroller_widget = pathbar.crumb_scroller.clone().upcast::<gtk::Widget>();
        gesture.connect_released(move |gesture, _, x, y| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            if browser.toolbar.pathbar.is_editing() {
                return;
            }
            let Some(widget) = gesture.widget() else {
                return;
            };
            let Some(mut picked) = widget.pick(x, y, PickFlags::DEFAULT) else {
                return;
            };
            loop {
                if picked == scroller_widget {
                    browser.begin_path_edit();
                    return;
                }
                if picked.downcast_ref::<Button>().is_some() {
                    return;
                }
                let Some(parent) = picked.parent() else {
                    return;
                };
                picked = parent;
            }
        });
        pathbar.crumb_scroller.add_controller(gesture);

        let weak = Rc::downgrade(self);
        let gesture = gtk::GestureClick::new();
        gesture.set_button(3);
        gesture.connect_pressed(move |gesture, _, x, y| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let Some(widget) = gesture.widget() else {
                return;
            };
            browser.show_pathbar_context(&widget, x as i32, y as i32);
        });
        self.toolbar.pathbar.widget.add_controller(gesture);
    }

    fn install_actions(self: &Rc<Self>, app: &Application) {
        self.add_action("back", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_back() {
                browser.refresh();
            }
        });
        self.add_action("forward", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_forward() {
                browser.refresh();
            }
        });
        self.add_action("up", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_up() {
                browser.refresh();
            }
        });
        self.add_action("refresh", |browser| {
            browser.rebuild_sidebar();
            browser.refresh();
        });
        self.add_action("search", |browser| {
            browser
                .status_left
                .set_text("Поиск будет добавлен в следующей версии")
        });
        self.add_action("edit-location", |browser| browser.begin_path_edit());
        self.add_action("ctx-open-tab", |browser| {
            browser.context_stub("Открыть в новой вкладке")
        });
        self.add_action("ctx-open-window", |browser| {
            browser.context_stub("Открыть в новом окне")
        });
        self.add_action("ctx-cut", |browser| browser.context_stub("Вырезать"));
        self.add_action("ctx-copy", |browser| browser.context_stub("Копировать"));
        self.add_action("ctx-trash", |browser| {
            browser.context_stub("Удалить в корзину")
        });
        self.add_action("about", |browser| browser.show_about_dialog());

        let show_hidden = gio::SimpleAction::new_stateful(
            "show-hidden",
            None,
            &self.settings.borrow().show_hidden.to_variant(),
        );
        let browser = Rc::clone(self);
        show_hidden.connect_activate(move |action, _| {
            let current = action
                .state()
                .and_then(|state| state.get::<bool>())
                .unwrap_or(false);
            let next = !current;
            action.set_state(&next.to_variant());
            browser.set_show_hidden(next);
        });
        self.window.add_action(&show_hidden);

        let toggle_sidebar = gio::SimpleAction::new_stateful(
            "toggle-sidebar",
            None,
            &self.settings.borrow().sidebar_visible.to_variant(),
        );
        let browser = Rc::clone(self);
        toggle_sidebar.connect_activate(move |action, _| {
            let current = action
                .state()
                .and_then(|state| state.get::<bool>())
                .unwrap_or(true);
            let next = !current;
            action.set_state(&next.to_variant());
            browser.set_sidebar_visible(next);
        });
        self.window.add_action(&toggle_sidebar);

        let new_window = gio::SimpleAction::new("new-window", None);
        let app_clone = app.clone();
        let settings = Rc::clone(&self.settings);
        new_window.connect_activate(move |_, _| {
            Browser::new(&app_clone, Rc::clone(&settings));
        });
        self.window.add_action(&new_window);

        app.set_accels_for_action("win.back", &["<alt>Left"]);
        app.set_accels_for_action("win.forward", &["<alt>Right"]);
        app.set_accels_for_action("win.up", &["<alt>Up"]);
        app.set_accels_for_action("win.refresh", &["F5"]);
        app.set_accels_for_action("win.show-hidden", &["<control>h"]);
        app.set_accels_for_action("win.edit-location", &["<control>l"]);
        app.set_accels_for_action("win.toggle-sidebar", &["F9"]);
        app.set_accels_for_action("win.search", &["<control>f"]);
        app.set_accels_for_action("win.new-window", &["<control>n"]);
    }

    fn install_menu(&self) {
        let menu = gio::Menu::new();

        let file_section = gio::Menu::new();
        file_section.append(Some("Новое окно"), Some("win.new-window"));
        file_section.append(Some("Обновить"), Some("win.refresh"));
        menu.append_section(None, &file_section);

        let view_section = gio::Menu::new();
        view_section.append(Some("Изменить путь"), Some("win.edit-location"));
        view_section.append(Some("Показывать скрытые файлы"), Some("win.show-hidden"));
        view_section.append(Some("Боковая панель"), Some("win.toggle-sidebar"));
        menu.append_section(None, &view_section);

        let help_section = gio::Menu::new();
        help_section.append(Some("Поиск"), Some("win.search"));
        help_section.append(Some("О программе"), Some("win.about"));
        menu.append_section(None, &help_section);

        self.toolbar.menu_button.set_menu_model(Some(&menu));
    }

    fn add_action<F>(self: &Rc<Self>, name: &str, callback: F)
    where
        F: Fn(&Rc<Self>) + 'static,
    {
        let action = gio::SimpleAction::new(name, None);
        let browser = Rc::clone(self);
        action.connect_activate(move |_, _| callback(&browser));
        self.window.add_action(&action);
    }

    fn refresh(self: &Rc<Self>) {
        let (path, show_hidden, can_back, can_forward, can_up) = {
            let state = self.state.borrow();
            (
                state.current().to_path_buf(),
                state.show_hidden,
                state.can_go_back(),
                state.can_go_forward(),
                state.can_go_up(),
            )
        };

        self.toolbar.back.set_sensitive(can_back);
        self.toolbar.forward.set_sensitive(can_forward);
        self.toolbar.up.set_sensitive(can_up);

        self.update_pathbar(&path);
        self.update_title(&path);

        self.store.remove_all();
        self.status_left.set_text("Ничего не выбрано");

        let entries = match read_directory(&path, show_hidden) {
            Ok(entries) => entries,
            Err(error) => {
                self.status_left
                    .set_text(&format!("Не удалось прочитать {}: {error}", path.display()));
                self.status_right.set_text("0 объектов");
                self.store_current_location(&path);
                self.sync_sidebar_selection();
                return;
            }
        };

        let total_size: u64 = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.size)
            .sum();

        for entry in &entries {
            let boxed = glib::BoxedAnyObject::new(Rc::clone(&entry.data));
            self.store.append(&boxed);
        }

        let count_text = plural_ru(entries.len(), "объект", "объекта", "объектов");
        let right_text = if total_size > 0 {
            format!("{count_text}, {}", human_size(total_size))
        } else {
            count_text
        };
        self.status_right.set_text(&right_text);

        self.store_current_location(&path);
        self.sync_sidebar_selection();
    }

    fn store_current_location(&self, path: &Path) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.last_location = Some(path.to_path_buf());
            remember_path(&mut settings.recent_locations, path, RECENT_LIMIT);
        }
        self.save_settings_now();
    }

    fn update_pathbar(self: &Rc<Self>, path: &Path) {
        let anchors = {
            let settings = self.settings.borrow();
            path_anchors(&settings, &self.devices.borrow())
        };
        let show_full_path = self.settings.borrow().show_full_path;
        let crumbs = build_breadcrumbs(path, &anchors, show_full_path);
        let icon = current_place_icon(path, &anchors);
        self.toolbar.pathbar.set_place_icon(&icon);
        let place_label = crumbs
            .first()
            .map(|(label, _)| label.clone())
            .unwrap_or_else(|| String::from("/"));
        self.toolbar.pathbar.set_place_tooltip(&place_label);

        if self.toolbar.pathbar.is_editing() {
            return;
        }

        detach_children(&self.toolbar.pathbar.crumbs);

        for (index, (label, target)) in crumbs.into_iter().enumerate() {
            if index > 0 {
                let separator = Label::new(Some("›"));
                separator.add_css_class("crumb-separator");
                self.toolbar.pathbar.crumbs.append(&separator);
            }

            let crumb_label = Label::new(Some(&label));
            crumb_label.set_ellipsize(EllipsizeMode::End);
            crumb_label.set_max_width_chars(32);

            let button = Button::new();
            button.set_child(Some(&crumb_label));
            button.add_css_class("flat");
            button.add_css_class("crumb");
            button.set_tooltip_text(Some(&target.display().to_string()));

            let weak = Rc::downgrade(self);
            button.connect_clicked(move |_| {
                if let Some(browser) = weak.upgrade() {
                    browser.navigate(target.clone());
                }
            });
            self.toolbar.pathbar.crumbs.append(&button);
        }

        self.toolbar
            .pathbar
            .crumbs
            .append(&self.toolbar.pathbar.filler);

        let scroller = self.toolbar.pathbar.crumb_scroller.clone();
        glib::idle_add_local_once(move || {
            let adjustment = scroller.hadjustment();
            let upper = adjustment.upper();
            let page = adjustment.page_size();
            adjustment.set_value((upper - page).max(0.0));
        });
    }

    fn update_title(&self, path: &Path) {
        let folder = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));
        let title = format!("{folder} — {APP_NAME}");
        self.window.set_title(Some(&title));
        self.title_label.set_text(&title);
    }

    fn begin_path_edit(self: &Rc<Self>) {
        let path = self.state.borrow().current().to_path_buf();
        if self.toolbar.pathbar.is_editing() {
            self.toolbar.pathbar.entry.grab_focus();
            self.toolbar.pathbar.entry.select_region(0, -1);
            return;
        }
        self.toolbar.pathbar.begin_edit(&path);
    }

    fn cancel_path_edit(self: &Rc<Self>) {
        self.toolbar.pathbar.end_edit();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    fn submit_path_bar(self: &Rc<Self>) {
        if !self.toolbar.pathbar.is_editing() {
            return;
        }
        let input = self.toolbar.pathbar.entry.text().to_string();
        let current = self.state.borrow().current().to_path_buf();
        let home = glib::home_dir();

        match resolve_path_text(&input, &current, &home) {
            Ok(path) if path.is_dir() => {
                self.navigate(path);
            }
            Ok(path) if path.is_file() => {
                let parent = path
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| current.clone());
                self.toolbar.pathbar.end_edit();
                self.navigate(parent);
                self.select_path(&path);
            }
            Ok(path) => {
                let message = format!("Это не папка и не файл: {}", path.display());
                self.toolbar.pathbar.set_error(Some(&message));
                self.status_left.set_text(&message);
            }
            Err(message) => {
                self.toolbar.pathbar.set_error(Some(&message));
                self.status_left.set_text(&message);
            }
        }
    }

    fn select_path(&self, path: &Path) {
        for position in 0..self.store.n_items() {
            let Some(item) = self.store.item(position) else {
                continue;
            };
            let Some(data) = item_data(&item) else {
                continue;
            };
            if data.path == path {
                self.selection.set_selected(position);
                self.column_view
                    .scroll_to(position, None, gtk::ListScrollFlags::NONE, None);
                break;
            }
        }
    }

    fn navigate(self: &Rc<Self>, path: PathBuf) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        self.toolbar.pathbar.end_edit();
        self.state.borrow_mut().navigate_to(path);
        self.refresh();
    }

    fn open(self: &Rc<Self>, path: &Path) {
        if path.is_dir() {
            self.navigate(path.to_path_buf());
            return;
        }

        {
            let mut settings = self.settings.borrow_mut();
            remember_path(&mut settings.recent_files, path, RECENT_LIMIT);
        }
        self.save_settings_now();

        let file = gio::File::for_path(path);
        let launcher = gtk::FileLauncher::new(Some(&file));
        let status_left = self.status_left.clone();
        launcher.launch(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(error) = result {
                    status_left.set_text(&format!("Не удалось открыть файл: {error}"));
                }
            },
        );
    }

    fn set_show_hidden(self: &Rc<Self>, show_hidden: bool) {
        if self.state.borrow().show_hidden == show_hidden {
            return;
        }
        self.state.borrow_mut().show_hidden = show_hidden;
        self.settings.borrow_mut().show_hidden = show_hidden;
        self.save_settings_now();
        self.refresh();
    }

    fn set_sidebar_visible(self: &Rc<Self>, visible: bool) {
        if visible {
            let width = self.settings.borrow().sidebar_width;
            self.sidebar.widget.set_visible(true);
            self.paned.set_position(width);
            self.toolbar
                .sidebar_toggle
                .set_icon_name("sidebar-show-symbolic");
        } else {
            // The width is persisted by connect_width_persistence only for
            // real user drags. Never overwrite it with a layout-induced
            // position when the panel is merely hidden.
            self.sidebar.widget.set_visible(false);
        }
        self.settings.borrow_mut().sidebar_visible = visible;
        self.save_settings_now();
    }

    fn rebuild_sidebar(self: &Rc<Self>) {
        let devices = collect_devices();
        let (rows, icon_size) = {
            let settings = self.settings.borrow();
            let current = self.state.borrow().current().to_path_buf();
            (
                compose_sidebar_rows(&settings, &devices, self.show_all_places.get(), &current),
                settings.sidebar_icon_size,
            )
        };
        *self.devices.borrow_mut() = devices;
        self.sidebar.render(&rows, icon_size);
        self.attach_sidebar_clicks();
        self.sync_sidebar_selection();
        self.update_places_popover();
    }

    fn attach_sidebar_clicks(self: &Rc<Self>) {
        for index in 0..self.sidebar.rows.borrow().len() {
            let Some(row) = self.sidebar.list.row_at_index(index as i32) else {
                continue;
            };
            let Some(sidebar_row) = self.sidebar.row(index as i32) else {
                continue;
            };
            let Some(content) = row
                .child()
                .and_then(|child| child.downcast::<GtkBox>().ok())
            else {
                continue;
            };

            // Attach to the whole row content, not to its last child: device
            // rows end with a capacity ProgressBar, and clicks on the icon or
            // label above it must still activate the place. The section header
            // painted inside the first row stays inert, as in KFilePlacesView.
            let weak = Rc::downgrade(self);
            let click_content = content.clone();
            let gesture = gtk::GestureClick::new();
            gesture.set_button(1);
            gesture.connect_released(move |_, _, _, y| {
                let Some(browser) = weak.upgrade() else {
                    return;
                };
                let Some(row) = browser.sidebar.list.row_at_index(index as i32) else {
                    return;
                };
                browser.last_sidebar_mouse_press.set(Some(Instant::now()));
                if sidebar_click_over_header(&row, &click_content, y) {
                    return;
                }
                browser.sidebar.list.select_row(Some(&row));
                browser.activate_sidebar_place(index as i32);
            });
            content.add_controller(gesture);

            if sidebar_row.section == SectionId::Places {
                let drag_source = gtk::DragSource::new();
                drag_source.set_actions(gtk::gdk::DragAction::MOVE);
                let dragged_id = sidebar_row.place.id.clone();
                drag_source.connect_prepare(move |_, _, _| {
                    Some(gtk::gdk::ContentProvider::for_value(&glib::Value::from(
                        dragged_id.clone(),
                    )))
                });
                content.add_controller(drag_source);

                let target_id = sidebar_row.place.id.clone();
                let drop_target =
                    gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
                let target_row = row.clone();
                drop_target.connect_enter(move |_, _, _| {
                    target_row.add_css_class("drop-target");
                    gtk::gdk::DragAction::MOVE
                });
                let target_row = row.clone();
                drop_target.connect_leave(move |_| {
                    target_row.remove_css_class("drop-target");
                });
                let weak = Rc::downgrade(self);
                let target_row = row.clone();
                let drop_content = content.clone();
                drop_target.connect_drop(move |_, value, _, y| {
                    target_row.remove_css_class("drop-target");
                    let Some(browser) = weak.upgrade() else {
                        return false;
                    };
                    let Ok(dragged_id) = value.get::<String>() else {
                        return false;
                    };
                    let after = y >= drop_content.height() as f64 / 2.0;
                    browser.reorder_place(&dragged_id, &target_id, after)
                });
                content.add_controller(drop_target);
            }
        }
    }

    fn reorder_place(self: &Rc<Self>, dragged_id: &str, target_id: &str, after: bool) -> bool {
        let changed = reorder_place_ids(
            &mut self.settings.borrow_mut().places_order,
            dragged_id,
            target_id,
            after,
        );
        if changed {
            self.save_settings_now();
            self.rebuild_sidebar();
        }
        changed
    }

    fn activate_sidebar_place(self: &Rc<Self>, index: i32) {
        let Some(row) = self.sidebar.row(index) else {
            return;
        };
        match row.place.target {
            PlaceTarget::Directory(path) => self.navigate(path),
            PlaceTarget::RecentFiles => self.show_recents_popover(RecentKind::Files, Some(index)),
            PlaceTarget::RecentLocations => {
                self.show_recents_popover(RecentKind::Locations, Some(index))
            }
            PlaceTarget::Unavailable(message) => self.status_left.set_text(&message),
        }
    }

    fn sync_sidebar_selection(&self) {
        let current = self.state.borrow().current().to_path_buf();
        let mut best: Option<(usize, usize)> = None;
        {
            let rows = self.sidebar.rows.borrow();
            for (index, row) in rows.iter().enumerate() {
                let PlaceTarget::Directory(path) = &row.place.target else {
                    continue;
                };
                if !current.starts_with(path) {
                    continue;
                }
                let depth = path.components().count();
                if best.is_none_or(|(_, best_depth)| depth >= best_depth) {
                    best = Some((index, depth));
                }
            }
        }

        match best.and_then(|(index, _)| self.sidebar.list.row_at_index(index as i32)) {
            Some(row) => self.sidebar.list.select_row(Some(&row)),
            None => self.sidebar.list.unselect_all(),
        }
    }

    fn update_places_popover(&self) {
        let list = &self.toolbar.pathbar.places_list;
        detach_children(list);
        self.toolbar.pathbar.places_targets.borrow_mut().clear();

        let devices = self.devices.borrow();
        let settings = self.settings.borrow();
        let mut targets = self.toolbar.pathbar.places_targets.borrow_mut();
        let mut row_count = 0;

        for section in SectionId::ALL {
            if settings.hidden_sections.contains(&section) {
                continue;
            }
            let places = places_for_section(&settings, &devices, section)
                .into_iter()
                .filter(|place| !settings.hidden_places.contains(&place.id))
                .filter(|place| matches!(place.target, PlaceTarget::Directory(_)))
                .collect::<Vec<_>>();
            if places.is_empty() {
                continue;
            }

            let header = Label::new(Some(section.title()));
            header.set_xalign(0.0);
            header.add_css_class("popover-section");
            let header_row = ListBoxRow::new();
            header_row.set_child(Some(&header));
            header_row.set_selectable(false);
            header_row.set_activatable(false);
            list.append(&header_row);
            targets.push(None);

            for place in places {
                let content = GtkBox::new(Orientation::Horizontal, 8);
                let icon = Image::from_icon_name(&resolved_icon_name(&place.icon));
                icon.set_pixel_size(16);
                let label = Label::new(Some(&place.label));
                label.set_xalign(0.0);
                label.set_hexpand(true);
                label.set_ellipsize(EllipsizeMode::End);
                content.append(&icon);
                content.append(&label);

                let row = ListBoxRow::new();
                row.set_child(Some(&content));
                if let PlaceTarget::Directory(path) = &place.target {
                    row.set_tooltip_text(Some(&path.display().to_string()));
                }
                list.append(&row);
                targets.push(Some(place.target));
                row_count += 1;
            }
        }

        if row_count == 0 {
            let label = Label::new(Some("Нет доступных мест"));
            label.add_css_class("menu-empty");
            let row = ListBoxRow::new();
            row.set_child(Some(&label));
            row.set_selectable(false);
            row.set_activatable(false);
            list.append(&row);
            targets.push(None);
        }
    }

    fn save_settings_now(&self) {
        if let Err(error) = self.settings.borrow().save() {
            glib::g_warning!("Dolphy", "не удалось сохранить настройки: {}", error);
        }
    }

    fn schedule_save(self: &Rc<Self>) {
        let generation = self.save_generation.get().wrapping_add(1);
        self.save_generation.set(generation);
        let settings = Rc::clone(&self.settings);
        let generation_cell = Rc::clone(&self.save_generation);
        glib::timeout_add_local_once(Duration::from_millis(250), move || {
            if generation_cell.get() == generation {
                if let Err(error) = settings.borrow().save() {
                    glib::g_warning!("Dolphy", "не удалось сохранить настройки: {}", error);
                }
            }
        });
    }

    fn connect_width_persistence(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.paned.connect_position_notify(move |paned| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            // Ignore every position change that is not caused by the user
            // dragging the splitter (initial layout, window clamping, hiding
            // the panel, ...). Persisting those would silently replace the
            // width chosen by the user.
            if !sidebar_resize_is_user_drag() || !browser.settings.borrow().sidebar_visible {
                return;
            }
            let Some(width) = sidebar_width_from_paned_position(paned.position()) else {
                return;
            };
            let changed = {
                let mut settings = browser.settings.borrow_mut();
                if settings.sidebar_width != width {
                    settings.sidebar_width = width;
                    true
                } else {
                    false
                }
            };
            if changed {
                browser.schedule_save();
            }
        });
    }

    fn connect_close_persistence(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |window| {
            if let Some(browser) = weak.upgrade() {
                if !window.is_maximized() && !window.is_fullscreen() {
                    let width = window.width().max(1);
                    let height = window.height().max(1);
                    let mut settings = browser.settings.borrow_mut();
                    settings.window_width = width;
                    settings.window_height = height;
                }
                browser.save_settings_now();
            }
            glib::Propagation::Proceed
        });
    }

    fn selected_data(&self) -> Option<Rc<FileData>> {
        self.selection
            .selected_item()
            .and_then(|item| item_data(&item))
    }

    fn context_stub(&self, action: &str) {
        let text = self
            .selected_data()
            .map(|data| format!("{action}: «{}» — пока не реализовано", data.name))
            .unwrap_or_else(|| format!("{action}: нет выбранного элемента"));
        self.status_left.set_text(&text);
    }

    fn show_about_dialog(&self) {
        let about = gtk::AboutDialog::builder()
            .program_name(APP_NAME)
            .version(VERSION)
            .comments("Файловый менеджер для GTK4, вдохновлённый KDE Dolphin")
            .website("https://github.com/ArmanIOL/Dolphy")
            .license_type(gtk::License::Gpl30)
            .authors(["ArmanIOL"])
            .build();

        about.set_transient_for(Some(&self.window));
        about.set_modal(true);
        about.present();
    }

    fn show_pathbar_context(self: &Rc<Self>, widget: &impl IsA<gtk::Widget>, x: i32, y: i32) {
        let popover = Popover::new();
        popover.add_css_class("places-popover");
        popover.set_has_arrow(false);

        let content = GtkBox::new(Orientation::Vertical, 1);
        content.set_margin_top(4);
        content.set_margin_bottom(4);
        content.set_margin_start(4);
        content.set_margin_end(4);
        content.set_size_request(250, -1);

        let copy = menu_row("edit-copy", "Копировать расположение", true, false, false);
        let weak = Rc::downgrade(self);
        let clipboard = widget.clipboard();
        let popover_clone = popover.clone();
        copy.connect_clicked(move |_| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let path = browser.state.borrow().current().to_path_buf();
            clipboard.set_text(&path.to_string_lossy());
            browser
                .status_left
                .set_text(&format!("Скопировано: {}", path.display()));
            popover_clone.popdown();
        });
        content.append(&copy);

        let paste = menu_row("edit-paste", "Вставить", true, false, false);
        let weak = Rc::downgrade(self);
        let popover_clone = popover.clone();
        let clipboard = widget.clipboard();
        paste.connect_clicked(move |_| {
            popover_clone.popdown();
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let weak_browser = Rc::downgrade(&browser);
            clipboard.read_text_async(None::<&gio::Cancellable>, move |result| {
                let Some(browser) = weak_browser.upgrade() else {
                    return;
                };
                if let Ok(Some(text)) = result {
                    let text = text.to_string();
                    if text.trim().is_empty() {
                        return;
                    }
                    browser.begin_path_edit();
                    browser.toolbar.pathbar.entry.set_text(&text);
                    browser.submit_path_bar();
                }
            });
        });
        content.append(&paste);

        content.append(&menu_separator());

        let editing = self.toolbar.pathbar.is_editing();
        let edit = menu_row("edit-entry", "Изменить", true, false, editing);
        self.connect_menu_action(&edit, &popover, |browser| browser.begin_path_edit());
        content.append(&edit);

        let navigate = menu_row("go-home", "Перейти", true, false, !editing);
        self.connect_menu_action(&navigate, &popover, |browser| browser.cancel_path_edit());
        content.append(&navigate);

        content.append(&menu_separator());

        let show_full_path = self.settings.borrow().show_full_path;
        let full_path = menu_row(
            "view-list-details",
            "Показывать полный путь",
            true,
            false,
            show_full_path,
        );
        self.connect_menu_action(&full_path, &popover, move |browser| {
            browser.set_show_full_path(!show_full_path)
        });
        content.append(&full_path);

        popover.set_child(Some(&content));
        let rectangle = gtk::gdk::Rectangle::new(x, y, 1, 1);
        self.present_popup(&popover, widget, &rectangle);
    }

    fn set_show_full_path(self: &Rc<Self>, show_full_path: bool) {
        {
            let mut settings = self.settings.borrow_mut();
            if settings.show_full_path == show_full_path {
                return;
            }
            settings.show_full_path = show_full_path;
        }
        self.save_settings_now();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    fn show_sidebar_context(
        self: &Rc<Self>,
        row_index: Option<i32>,
        over_header: bool,
        x: i32,
        y: i32,
    ) {
        let popover = Popover::new();
        popover.add_css_class("places-popover");
        popover.set_has_arrow(false);

        let content = GtkBox::new(Orientation::Vertical, 1);
        content.set_margin_top(4);
        content.set_margin_bottom(4);
        content.set_margin_start(4);
        content.set_margin_end(4);
        content.set_size_request(280, -1);

        let row =
            row_index.and_then(|index| self.sidebar.rows.borrow().get(index as usize).cloned());

        // KFilePlacesView only edits the clicked entry when it is also the
        // current selection. Otherwise the click is treated as empty space.
        let clicked_selected = row_index.is_some()
            && self
                .sidebar
                .list
                .selected_row()
                .map(|selected| selected.index())
                == row_index;
        let item = if over_header || !clicked_selected {
            None
        } else {
            row.as_ref()
        };

        if item.is_none() {
            let add = menu_row("document-new", "Добавить…", true, false, false);
            self.connect_menu_action(&add, &popover, |browser| browser.show_add_place_dialog());
            content.append(&add);
        }

        if let Some(row) = item {
            let place = &row.place;
            if let PlaceTarget::Directory(path) = &place.target {
                let open = menu_row("window-new", "Открыть в новом окне", true, false, false);
                let target = path.clone();
                self.connect_menu_action(&open, &popover, move |browser| {
                    browser.open_place_in_new_window(target.clone())
                });
                content.append(&open);

                let properties = menu_row("document-properties", "Свойства", true, false, false);
                let target = path.clone();
                self.connect_menu_action(&properties, &popover, move |browser| {
                    browser.show_place_properties(&target)
                });
                content.append(&properties);
                content.append(&menu_separator());
            }

            // KFilePlacesView offers Edit for every non-device bookmark,
            // including the standard places. In our model only directory-backed
            // entries can be represented by the edit dialog.
            let editable = !place.id.starts_with("device:")
                && place.id != "trash"
                && matches!(&place.target, PlaceTarget::Directory(_));
            if editable {
                let edit = menu_row("edit-entry", "Изменить…", true, false, false);
                let place_id = place.id.clone();
                self.connect_menu_action(&edit, &popover, move |browser| {
                    browser.show_edit_place_dialog(&place_id)
                });
                content.append(&edit);
            }

            if place.custom {
                let remove = menu_row(
                    "bookmark-remove-symbolic",
                    "Удалить из панели «Точки входа»",
                    true,
                    true,
                    false,
                );
                let place_id = place.id.clone();
                self.connect_menu_action(&remove, &popover, move |browser| {
                    browser.remove_custom_place(&place_id)
                });
                content.append(&remove);
            }

            let hidden = row.hidden;
            let hide = menu_row("hint", "Скрыть", !row.section_hidden, false, hidden);
            let place_id = place.id.clone();
            self.connect_menu_action(&hide, &popover, move |browser| {
                browser.set_place_hidden(&place_id, !hidden)
            });
            content.append(&hide);

            let section = row.section;
            let section_hidden = row.section_hidden;
            let hide_section = menu_row(
                "hint",
                &format!("Скрыть раздел «{}»", section.title()),
                true,
                false,
                section_hidden,
            );
            self.connect_menu_action(&hide_section, &popover, move |browser| {
                browser.set_section_hidden(section, !section_hidden)
            });
            content.append(&hide_section);
        } else if over_header {
            if let Some(row) = &row {
                let section = row.section;
                let section_hidden = row.section_hidden;
                let hide_section = menu_row(
                    "hint",
                    &format!("Скрыть раздел «{}»", section.title()),
                    true,
                    false,
                    section_hidden,
                );
                self.connect_menu_action(&hide_section, &popover, move |browser| {
                    browser.set_section_hidden(section, !section_hidden)
                });
                content.append(&hide_section);
            }
        }

        if item.is_none() {
            if self.hidden_place_count() > 0 {
                let show_all = menu_row(
                    "visibility",
                    "Показать все",
                    true,
                    false,
                    self.show_all_places.get(),
                );
                self.connect_menu_action(&show_all, &popover, |browser| {
                    browser.toggle_show_all_places()
                });
                content.append(&show_all);
            }

            content.append(&menu_separator());
            let heading = Label::new(Some("Размер значков"));
            heading.set_xalign(0.0);
            heading.add_css_class("popover-section");
            content.append(&heading);

            let icon_size = self.settings.borrow().sidebar_icon_size;
            for (size, label) in [
                (16, "Маленькие (16×16)"),
                (22, "Средние (22×22)"),
                (32, "Большие (32×32)"),
                (48, "Огромные (48×48)"),
            ] {
                let button = menu_row("view-list-details", label, true, false, icon_size == size);
                self.connect_menu_action(&button, &popover, move |browser| {
                    browser.set_sidebar_icon_size(size)
                });
                content.append(&button);
            }
        }

        popover.set_child(Some(&content));
        let rectangle = gtk::gdk::Rectangle::new(x, y, 1, 1);
        self.present_popup(&popover, &self.sidebar.list, &rectangle);
    }

    fn hidden_place_count(&self) -> usize {
        let settings = self.settings.borrow();
        let devices = self.devices.borrow();
        let mut count = 0;
        for section in SectionId::ALL {
            let section_hidden = settings.hidden_sections.contains(&section);
            for place in places_for_section(&settings, &devices, section) {
                if section_hidden || settings.hidden_places.contains(&place.id) {
                    count += 1;
                }
            }
        }
        count
    }

    fn set_place_hidden(self: &Rc<Self>, place_id: &str, hidden: bool) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.hidden_places.retain(|id| id != place_id);
            if hidden {
                settings.hidden_places.push(place_id.to_string());
            }
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    fn set_section_hidden(self: &Rc<Self>, section: SectionId, hidden: bool) {
        {
            let mut settings = self.settings.borrow_mut();
            settings
                .hidden_sections
                .retain(|candidate| *candidate != section);
            if hidden {
                settings.hidden_sections.push(section);
            }
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    fn toggle_show_all_places(self: &Rc<Self>) {
        let show_all = !self.show_all_places.get();
        self.show_all_places.set(show_all);
        self.rebuild_sidebar();
        self.status_left.set_text(if show_all {
            "Показаны все скрытые записи"
        } else {
            "Скрытые записи снова скрыты"
        });
    }

    fn set_sidebar_icon_size(self: &Rc<Self>, size: i32) {
        if !matches!(size, 16 | 22 | 32 | 48) {
            return;
        }
        {
            let mut settings = self.settings.borrow_mut();
            if settings.sidebar_icon_size == size {
                return;
            }
            settings.sidebar_icon_size = size;
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        self.status_left
            .set_text(&format!("Размер значков боковой панели: {size} px"));
    }

    fn open_place_in_new_window(self: &Rc<Self>, path: PathBuf) {
        let Some(app) = self.window.application() else {
            return;
        };
        let browser = Browser::new(&app, Rc::clone(&self.settings));
        browser.navigate(path);
    }

    fn show_place_properties(self: &Rc<Self>, path: &Path) {
        let dialog = gtk::Window::builder()
            .title("Свойства")
            .transient_for(&self.window)
            .modal(true)
            .resizable(false)
            .default_width(460)
            .build();

        let content = GtkBox::new(Orientation::Vertical, 8);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(16);
        content.set_margin_end(16);

        let heading = Label::new(Some(
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| path.display().to_string())
                .as_str(),
        ));
        heading.set_xalign(0.0);
        heading.add_css_class("title");
        content.append(&heading);

        let info = gio::File::for_path(path)
            .query_info(
                "standard::display-name,standard::content-type,standard::size,time::modified",
                gio::FileQueryInfoFlags::NONE,
                gio::Cancellable::NONE,
            )
            .ok();
        let mut rows = vec![("Расположение", path.display().to_string())];
        if let Some(info) = info {
            if let Some(content_type) = info.content_type() {
                rows.push(("Тип", content_type.to_string()));
            }
            rows.push(("Размер", human_size(info.size().max(0) as u64)));
            if let Some(modified) = info.modification_date_time() {
                rows.push((
                    "Изменён",
                    modified
                        .format("%d.%m.%Y %H:%M")
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                ));
            }
        }

        for (label, value) in rows {
            let row = GtkBox::new(Orientation::Horizontal, 12);
            let name = Label::new(Some(label));
            name.set_xalign(0.0);
            name.set_size_request(110, -1);
            name.add_css_class("dim-label");
            let text = Label::new(Some(&value));
            text.set_xalign(0.0);
            text.set_hexpand(true);
            text.set_selectable(true);
            text.set_wrap(true);
            row.append(&name);
            row.append(&text);
            content.append(&row);
        }

        let close = Button::with_label("Закрыть");
        close.set_halign(gtk::Align::End);
        let dialog_clone = dialog.clone();
        close.connect_clicked(move |_| dialog_clone.close());
        content.append(&close);
        dialog.set_child(Some(&content));
        dialog.present();
    }

    fn remove_custom_place(self: &Rc<Self>, place_id: &str) {
        let name = {
            let mut settings = self.settings.borrow_mut();
            let Some(place) = settings
                .custom_places
                .iter()
                .find(|place| place.id == place_id)
            else {
                return;
            };
            let name = place.name.clone();
            settings.custom_places.retain(|place| place.id != place_id);
            settings.places_order.retain(|id| id != place_id);
            settings.hidden_places.retain(|id| id != place_id);
            name
        };
        self.save_settings_now();
        self.rebuild_sidebar();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
        self.status_left
            .set_text(&format!("Запись «{name}» удалена"));
    }

    fn add_custom_place(self: &Rc<Self>, name: String, path: PathBuf, icon: String) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        let name = if name.trim().is_empty() {
            path.file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        } else {
            name.trim().to_string()
        };

        {
            let mut settings = self.settings.borrow_mut();
            let id = format!("custom:{}", settings.next_custom_id.max(1));
            settings.next_custom_id = settings.next_custom_id.saturating_add(1).max(1);
            settings.custom_places.push(CustomPlace {
                id: id.clone(),
                name: name.clone(),
                path: path.clone(),
                icon,
            });
            settings.places_order.push(id);
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        let current = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&current);
        self.status_left
            .set_text(&format!("Добавлена запись «{name}»"));
    }

    fn update_custom_place(
        self: &Rc<Self>,
        place_id: &str,
        name: String,
        path: PathBuf,
        icon: String,
    ) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        let name = if name.trim().is_empty() {
            path.file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        } else {
            name.trim().to_string()
        };

        {
            let mut settings = self.settings.borrow_mut();
            let Some(place) = settings
                .custom_places
                .iter_mut()
                .find(|place| place.id == place_id)
            else {
                return;
            };
            place.name = name.clone();
            place.path = path;
            place.icon = icon;
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        let current = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&current);
        self.status_left
            .set_text(&format!("Запись «{name}» изменена"));
    }

    fn update_system_place(
        self: &Rc<Self>,
        place_id: &str,
        name: String,
        path: PathBuf,
        icon: String,
    ) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        let name = if name.trim().is_empty() {
            path.file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned())
        } else {
            name.trim().to_string()
        };

        {
            let mut settings = self.settings.borrow_mut();
            settings.edited_places.retain(|place| place.id != place_id);
            settings.edited_places.push(CustomPlace {
                id: place_id.to_string(),
                name: name.clone(),
                path,
                icon,
            });
            settings.normalize();
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        let current = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&current);
        self.status_left
            .set_text(&format!("Запись «{name}» изменена"));
    }

    fn update_place(self: &Rc<Self>, place_id: &str, name: String, path: PathBuf, icon: String) {
        let is_custom = self
            .settings
            .borrow()
            .custom_places
            .iter()
            .any(|place| place.id == place_id);
        if is_custom {
            self.update_custom_place(place_id, name, path, icon);
        } else {
            self.update_system_place(place_id, name, path, icon);
        }
    }

    fn show_add_place_dialog(self: &Rc<Self>) {
        self.show_place_dialog(None);
    }

    fn show_edit_place_dialog(self: &Rc<Self>, place_id: &str) {
        self.show_place_dialog(Some(place_id.to_string()));
    }

    fn show_place_dialog(self: &Rc<Self>, edit_id: Option<String>) {
        let edit_place = edit_id.as_ref().and_then(|id| {
            let settings = self.settings.borrow();
            if let Some(place) = settings.custom_places.iter().find(|place| &place.id == id) {
                return Some(place.clone());
            }
            default_places(&settings)
                .into_iter()
                .find(|place| &place.id == id)
                .and_then(|place| match place.target {
                    PlaceTarget::Directory(path) => Some(CustomPlace {
                        id: place.id,
                        name: place.label,
                        path,
                        icon: place.icon,
                    }),
                    _ => None,
                })
        });
        if edit_id.is_some() && edit_place.is_none() {
            return;
        }

        let editing = edit_place.is_some();
        let title = if editing {
            "Изменить пункт на панели быстрого доступа"
        } else {
            "Добавить пункт на панель быстрого доступа"
        };
        let dialog = gtk::Window::builder()
            .title(title)
            .transient_for(&self.window)
            .modal(true)
            .resizable(false)
            .default_width(460)
            .build();

        let content = GtkBox::new(Orientation::Vertical, 10);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(16);
        content.set_margin_end(16);

        let heading = Label::new(Some(title));
        heading.set_xalign(0.0);
        heading.add_css_class("title");

        let name_label = Label::new(Some("Название"));
        name_label.set_xalign(0.0);
        let name_entry = Entry::new();
        name_entry.set_placeholder_text(Some("Введите название точки входа"));
        name_entry.set_activates_default(true);
        if let Some(place) = &edit_place {
            name_entry.set_text(&place.name);
        }

        let path_label = Label::new(Some("Расположение"));
        path_label.set_xalign(0.0);
        let path_entry = Entry::new();
        path_entry.set_hexpand(true);
        path_entry.set_activates_default(true);
        if let Some(place) = &edit_place {
            path_entry.set_text(&place.path.to_string_lossy());
        } else {
            let current = self.state.borrow().current().to_path_buf();
            path_entry.set_text(&current.to_string_lossy());
        }

        let icon_label = Label::new(Some("Значок"));
        icon_label.set_xalign(0.0);
        let icon_entry = Entry::new();
        icon_entry.set_placeholder_text(Some("folder"));
        icon_entry.set_activates_default(true);
        icon_entry.set_text(
            edit_place
                .as_ref()
                .map(|place| place.icon.as_str())
                .unwrap_or("folder"),
        );

        let error = Label::new(None);
        error.set_xalign(0.0);
        error.set_wrap(true);
        error.add_css_class("add-place-error");
        error.set_visible(false);

        let buttons = GtkBox::new(Orientation::Horizontal, 8);
        buttons.set_halign(gtk::Align::End);
        let cancel = Button::with_label("Отмена");
        let add = Button::with_label("OK");
        add.add_css_class("suggested-action");
        buttons.append(&cancel);
        buttons.append(&add);

        content.append(&heading);
        content.append(&name_label);
        content.append(&name_entry);
        content.append(&path_label);
        content.append(&path_entry);
        content.append(&icon_label);
        content.append(&icon_entry);
        content.append(&error);
        content.append(&buttons);
        dialog.set_child(Some(&content));
        dialog.set_default_widget(Some(&add));

        let dialog_clone = dialog.clone();
        cancel.connect_clicked(move |_| dialog_clone.close());

        let weak = Rc::downgrade(self);
        let dialog_clone = dialog.clone();
        let name_entry_clone = name_entry.clone();
        let path_entry_clone = path_entry.clone();
        let icon_entry_clone = icon_entry.clone();
        let error_clone = error.clone();
        let edit_id_clone = edit_id.clone();
        add.connect_clicked(move |_| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let input = path_entry_clone.text().to_string();
            let current = browser.state.borrow().current().to_path_buf();
            let home = glib::home_dir();
            let icon = icon_entry_clone.text().trim().to_string();
            let icon = if icon.is_empty() {
                String::from("folder")
            } else {
                icon
            };
            match resolve_path_text(&input, &current, &home) {
                Ok(path) if path.is_dir() => {
                    let name = name_entry_clone.text().to_string();
                    dialog_clone.close();
                    if let Some(id) = &edit_id_clone {
                        browser.update_place(id, name, path, icon);
                    } else {
                        browser.add_custom_place(name, path, icon);
                    }
                }
                Ok(path) => {
                    error_clone.set_text(&format!(
                        "Нужно указать папку, а не файл: {}",
                        path.display()
                    ));
                    error_clone.set_visible(true);
                }
                Err(message) => {
                    error_clone.set_text(&message);
                    error_clone.set_visible(true);
                }
            }
        });

        let dialog_clone = dialog.clone();
        let key_controller = EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                dialog_clone.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        dialog.add_controller(key_controller);

        dialog.present();
        name_entry.grab_focus();
    }

    fn show_recents_popover(self: &Rc<Self>, kind: RecentKind, row_index: Option<i32>) {
        let (title, items) = {
            let settings = self.settings.borrow();
            match kind {
                RecentKind::Files => ("Последние файлы", settings.recent_files.clone()),
                RecentKind::Locations => {
                    ("Последние расположения", settings.recent_locations.clone())
                }
            }
        };

        let popover = Popover::new();
        popover.add_css_class("places-popover");
        popover.add_css_class("recent-popover");

        let content = GtkBox::new(Orientation::Vertical, 2);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(4);
        content.set_margin_end(4);
        content.set_size_request(360, -1);

        let heading = Label::new(Some(title));
        heading.set_xalign(0.0);
        heading.add_css_class("popover-section");
        content.append(&heading);

        if items.is_empty() {
            let empty = Label::new(Some(match kind {
                RecentKind::Files => "Пока нет недавно открытых файлов",
                RecentKind::Locations => "Пока нет недавних расположений",
            }));
            empty.add_css_class("menu-empty");
            empty.set_xalign(0.0);
            content.append(&empty);
        }

        for path in items {
            let name = path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let subtitle = path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .unwrap_or_default();
            let icon = match kind {
                RecentKind::Files => "document-open-recent",
                RecentKind::Locations => "folder-open-recent",
            };
            let button = recent_row(icon, &name, &subtitle, &path.display().to_string());

            let weak = Rc::downgrade(self);
            let popover_weak = popover.downgrade();
            let path_clone = path.clone();
            button.connect_clicked(move |_| {
                if let Some(popover) = popover_weak.upgrade() {
                    popover.popdown();
                }
                let Some(browser) = weak.upgrade() else {
                    return;
                };
                match kind {
                    RecentKind::Files => browser.open_recent_file(&path_clone),
                    RecentKind::Locations => browser.open_recent_location(&path_clone),
                }
            });
            content.append(&button);
        }

        popover.set_child(Some(&content));

        let row = row_index.and_then(|index| self.sidebar.list.row_at_index(index));
        let rectangle = row
            .and_then(|row| self.sidebar.list.compute_bounds(&row))
            .map(|bounds| {
                gtk::gdk::Rectangle::new(
                    bounds.x() as i32,
                    bounds.y() as i32,
                    bounds.width() as i32,
                    bounds.height() as i32,
                )
            })
            .unwrap_or_else(|| gtk::gdk::Rectangle::new(8, 8, 1, 1));
        self.present_popup(&popover, &self.sidebar.list, &rectangle);
    }

    fn open_recent_file(self: &Rc<Self>, path: &Path) {
        if path.is_file() {
            self.open(path);
            return;
        }
        {
            let mut settings = self.settings.borrow_mut();
            settings.recent_files.retain(|entry| entry != path);
        }
        self.save_settings_now();
        self.status_left
            .set_text(&format!("Файл больше не существует: {}", path.display()));
    }

    fn open_recent_location(self: &Rc<Self>, path: &Path) {
        if path.is_dir() {
            self.navigate(path.to_path_buf());
            return;
        }
        {
            let mut settings = self.settings.borrow_mut();
            settings.recent_locations.retain(|entry| entry != path);
        }
        self.save_settings_now();
        self.status_left
            .set_text(&format!("Папка больше не существует: {}", path.display()));
    }

    fn present_popup(
        &self,
        popover: &Popover,
        parent: &impl IsA<gtk::Widget>,
        rectangle: &gtk::gdk::Rectangle,
    ) {
        if let Some(previous) = self.popup.borrow_mut().take() {
            previous.popdown();
            previous.unparent();
        }
        if popover.parent().is_some() {
            popover.unparent();
        }
        popover.set_parent(parent);
        popover.set_pointing_to(Some(rectangle));
        popover.popup();
        *self.popup.borrow_mut() = Some(popover.clone());
    }

    fn connect_menu_action<F>(self: &Rc<Self>, button: &Button, popover: &Popover, action: F)
    where
        F: Fn(&Rc<Self>) + 'static,
    {
        let weak = Rc::downgrade(self);
        let popover_weak = popover.downgrade();
        button.connect_clicked(move |_| {
            if let Some(popover) = popover_weak.upgrade() {
                popover.popdown();
            }
            if let Some(browser) = weak.upgrade() {
                action(&browser);
            }
        });
    }
}

fn menu_row(icon_name: &str, label: &str, sensitive: bool, danger: bool, checked: bool) -> Button {
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);

    let text = Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    text.set_ellipsize(EllipsizeMode::End);

    content.append(&icon);
    content.append(&text);
    if checked {
        let check = Label::new(Some("✓"));
        check.add_css_class("menu-check");
        content.append(&check);
    }

    let button = Button::new();
    button.set_child(Some(&content));
    button.add_css_class("flat");
    button.add_css_class("menu-row");
    if danger {
        button.add_css_class("danger");
    }
    button.set_sensitive(sensitive);
    button
}

fn recent_row(icon_name: &str, name: &str, subtitle: &str, tooltip: &str) -> Button {
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    icon.set_valign(gtk::Align::Center);

    let text = GtkBox::new(Orientation::Vertical, 0);
    text.set_hexpand(true);
    let name_label = Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.set_ellipsize(EllipsizeMode::End);
    name_label.add_css_class("recent-name");
    let path_label = Label::new(Some(subtitle));
    path_label.set_xalign(0.0);
    path_label.set_ellipsize(EllipsizeMode::Middle);
    path_label.add_css_class("recent-path");
    text.append(&name_label);
    text.append(&path_label);

    content.append(&icon);
    content.append(&text);

    let button = Button::new();
    button.set_child(Some(&content));
    button.add_css_class("flat");
    button.add_css_class("menu-row");
    button.set_tooltip_text(Some(tooltip));
    button
}

fn menu_separator() -> Separator {
    let separator = Separator::new(Orientation::Horizontal);
    separator.add_css_class("menu-separator");
    separator
}

// ---------------------------------------------------------------------------
// File view
// ---------------------------------------------------------------------------

fn build_file_view(selection: &SingleSelection) -> (ColumnView, ScrolledWindow) {
    let context_menu = gio::Menu::new();
    context_menu.append(Some("Открыть в новой вкладке"), Some("win.ctx-open-tab"));
    context_menu.append(Some("Открыть в новом окне"), Some("win.ctx-open-window"));
    context_menu.append(Some("Вырезать"), Some("win.ctx-cut"));
    context_menu.append(Some("Копировать"), Some("win.ctx-copy"));
    context_menu.append(Some("Удалить в корзину"), Some("win.ctx-trash"));

    let popover = PopoverMenu::from_model(Some(&context_menu));
    popover.set_has_arrow(false);
    popover.add_css_class("context-menu");

    let name_factory = build_name_factory(selection, &popover);
    let size_factory = build_text_factory(true, selection, &popover);
    let date_factory = build_text_factory(false, selection, &popover);

    let name_column = ColumnViewColumn::new(Some("Имя"), Some(name_factory));
    name_column.set_expand(true);
    name_column.set_resizable(true);

    let size_column = ColumnViewColumn::new(Some("Размер"), Some(size_factory));
    size_column.set_fixed_width(110);
    size_column.set_resizable(true);

    let date_column = ColumnViewColumn::new(Some("Дата изменения"), Some(date_factory));
    date_column.set_fixed_width(180);
    date_column.set_resizable(true);

    let column_view = ColumnView::new(Some(selection.clone()));
    column_view.append_column(&name_column);
    column_view.append_column(&size_column);
    column_view.append_column(&date_column);
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_hexpand(true);
    column_view.set_vexpand(true);

    let scroller = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&column_view)
        .build();

    (column_view, scroller)
}

fn build_name_factory(
    selection: &SingleSelection,
    popover: &PopoverMenu,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let selection_setup = selection.clone();
    let popover_setup = popover.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        hbox.set_margin_top(2);
        hbox.set_margin_bottom(2);
        hbox.set_margin_start(4);
        hbox.set_margin_end(4);

        let icon = Image::new();
        icon.set_pixel_size(16);

        let label = Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(EllipsizeMode::Middle);

        hbox.append(&icon);
        hbox.append(&label);
        list_item.set_child(Some(&hbox));

        attach_context_menu(&hbox, list_item, &selection_setup, &popover_setup);
    });

    factory.connect_bind(|_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(data) = list_item.item().as_ref().and_then(item_data) else {
            return;
        };
        let Some(child) = list_item.child() else {
            return;
        };
        let Some(hbox) = child.downcast_ref::<GtkBox>() else {
            return;
        };
        let Some(icon) = hbox.first_child().and_then(|w| w.downcast::<Image>().ok()) else {
            return;
        };
        let Some(label) = hbox.last_child().and_then(|w| w.downcast::<Label>().ok()) else {
            return;
        };

        icon.set_from_gicon(&data.icon);
        label.set_text(&data.name);
    });

    factory
}

fn build_text_factory(
    right_aligned: bool,
    selection: &SingleSelection,
    popover: &PopoverMenu,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let selection_setup = selection.clone();
    let popover_setup = popover.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let label = Label::new(None);
        label.set_xalign(if right_aligned { 1.0 } else { 0.0 });
        label.set_hexpand(true);
        if right_aligned {
            label.set_halign(gtk::Align::End);
            label.set_margin_end(6);
        } else {
            label.set_halign(gtk::Align::Start);
        }

        list_item.set_child(Some(&label));
        attach_context_menu(&label, list_item, &selection_setup, &popover_setup);
    });

    factory.connect_bind(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(data) = list_item.item().as_ref().and_then(item_data) else {
            return;
        };
        let Some(child) = list_item.child() else {
            return;
        };
        let Some(label) = child.downcast_ref::<Label>() else {
            return;
        };

        if right_aligned {
            label.set_text(&data.size_text);
        } else {
            label.set_text(&data.modified_text);
        }
    });

    factory
}

fn attach_context_menu<W>(
    widget: &W,
    list_item: &gtk::ListItem,
    selection: &SingleSelection,
    popover: &PopoverMenu,
) where
    W: IsA<gtk::Widget> + 'static,
{
    let gesture = gtk::GestureClick::new();
    gesture.set_button(3);

    let list_item = list_item.clone();
    let selection = selection.clone();
    let popover = popover.clone();

    gesture.connect_pressed(move |gesture, _, x, y| {
        let position = list_item.position();
        if position != u32::MAX {
            selection.set_selected(position);
        }

        let Some(widget) = gesture.widget() else {
            return;
        };

        popover.popdown();
        if popover.parent().is_some() {
            popover.unparent();
        }
        popover.set_parent(&widget);
        popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });

    widget.add_controller(gesture);
}

// ---------------------------------------------------------------------------
// Directory reading and formatting
// ---------------------------------------------------------------------------

struct DirEntryData {
    is_dir: bool,
    size: u64,
    data: Rc<FileData>,
}

fn read_directory(path: &Path, show_hidden: bool) -> std::io::Result<Vec<DirEntryData>> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let name = entry.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let entry_path = entry.path();
        let metadata = entry.metadata().ok();
        let is_dir = entry
            .file_type()
            .map(|kind| kind.is_dir())
            .unwrap_or_else(|_| metadata.as_ref().map(|meta| meta.is_dir()).unwrap_or(false));

        let size = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
        let modified_text = metadata
            .as_ref()
            .map(format_modified)
            .unwrap_or_else(|| String::from("—"));

        let (mime, _) = gio::content_type_guess(Some(&entry_path), None::<&[u8]>);
        let kind_text = if is_dir {
            String::from("Папка")
        } else {
            let description = gio::content_type_get_description(&mime).to_string();
            if description.is_empty() || description == "Unknown" {
                String::from("Файл")
            } else {
                description
            }
        };

        let icon: gio::Icon = if is_dir {
            gio::ThemedIcon::new("folder").upcast()
        } else {
            gio::content_type_get_icon(&mime)
        };

        let size_text = if is_dir {
            plural_ru(count_children(&entry_path), "объект", "объекта", "объектов")
        } else {
            human_size(size)
        };

        entries.push(DirEntryData {
            is_dir,
            size,
            data: Rc::new(FileData {
                path: entry_path,
                name,
                kind_text,
                size_text,
                modified_text,
                icon,
            }),
        });
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.data.name.to_lowercase().cmp(&b.data.name.to_lowercase()))
    });

    Ok(entries)
}

fn count_children(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| entries.count())
        .unwrap_or(0)
}

fn format_modified(metadata: &std::fs::Metadata) -> String {
    let Ok(modified) = metadata.modified() else {
        return String::from("—");
    };
    let Ok(duration) = modified.duration_since(UNIX_EPOCH) else {
        return String::from("—");
    };
    let Ok(datetime) = glib::DateTime::from_unix_local(duration.as_secs() as i64) else {
        return String::from("—");
    };

    datetime
        .format("%d.%m.%Y в %H:%M")
        .map(|text| text.to_string())
        .unwrap_or_else(|_| String::from("—"))
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} Б")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn plural_ru(count: usize, one: &str, few: &str, many: &str) -> String {
    let mod100 = count % 100;
    let mod10 = count % 10;
    let word = if (11..=14).contains(&mod100) {
        many
    } else if mod10 == 1 {
        one
    } else if (2..=4).contains(&mod10) {
        few
    } else {
        many
    };
    format!("{count} {word}")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(label: &str) -> Self {
            let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "dolphy-test-{}-{unique}-{label}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create test dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn sample_settings() -> Settings {
        let mut settings = Settings::defaults();
        settings.sidebar_visible = false;
        settings.sidebar_width = 321;
        settings.sidebar_icon_size = 22;
        settings.hidden_sections = vec![SectionId::Remote];
        settings.custom_places.push(CustomPlace {
            id: String::from("custom:7"),
            name: String::from("Проекты"),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder-projects"),
        });
        settings.next_custom_id = 8;
        settings.places_order.push(String::from("custom:7"));
        settings.edited_places.push(CustomPlace {
            id: String::from("home"),
            name: String::from("Личное"),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder-home"),
        });
        settings.hidden_places = vec![
            String::from("music"),
            String::from("custom:7"),
            String::from("device:/mnt/windows"),
        ];
        settings.show_hidden = true;
        settings.show_full_path = true;
        settings.window_width = 1440;
        settings.window_height = 900;
        settings.last_location = Some(PathBuf::from("/tmp"));
        settings.recent_locations = vec![PathBuf::from("/tmp"), PathBuf::from("/")];
        settings.recent_files = vec![PathBuf::from("/tmp/report.txt")];
        settings.normalize();
        settings
    }

    #[test]
    fn settings_round_trip_preserves_all_state() {
        let dir = TempDir::new("round-trip");
        let path = dir.path.join("dolphy.conf");
        let settings = sample_settings();

        settings.save_to_path(&path).expect("save settings");
        let loaded = Settings::load_from_path(&path);

        assert_eq!(loaded, settings);
        let data = std::fs::read_to_string(&path).expect("read settings");
        assert!(data.contains("version=1"));
        assert!(!data.contains("sidebar_width_placeholder"));
        assert!(data.contains("width=321"));
    }

    #[test]
    fn missing_config_uses_defaults_without_backup() {
        let dir = TempDir::new("missing");
        let path = dir.path.join("dolphy.conf");
        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded, Settings::defaults());
        assert!(!path.with_file_name("dolphy.conf.bak").exists());
    }

    #[test]
    fn corrupt_config_is_backed_up_and_defaults_are_used() {
        let dir = TempDir::new("corrupt");
        let path = dir.path.join("dolphy.conf");
        std::fs::write(&path, "это не key file\n[[[bad").expect("write corrupt config");

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded, Settings::defaults());
        assert!(!path.exists());
        assert!(path.with_file_name("dolphy.conf.bak").is_file());
    }

    #[test]
    fn incompatible_config_version_falls_back_to_defaults() {
        let dir = TempDir::new("version");
        let path = dir.path.join("dolphy.conf");
        std::fs::write(&path, "[General]\nversion=99\n").expect("write versioned config");

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded, Settings::defaults());
        assert!(path.with_file_name("dolphy.conf.bak").is_file());
    }

    #[test]
    fn resolve_path_text_handles_all_input_forms() {
        let dir = TempDir::new("resolve");
        let folder = dir.path.join("папка с пробелом");
        let file = dir.path.join("файл.txt");
        std::fs::create_dir_all(&folder).expect("create folder");
        std::fs::write(&file, "data").expect("create file");
        let canonical_dir = dir.path.canonicalize().expect("canonical dir");
        let canonical_folder = folder.canonicalize().expect("canonical folder");
        let canonical_file = file.canonicalize().expect("canonical file");

        assert!(resolve_path_text("", &canonical_dir, &canonical_dir).is_err());
        assert!(resolve_path_text("   ", &canonical_dir, &canonical_dir).is_err());
        assert_eq!(
            resolve_path_text("~", &canonical_dir, &canonical_dir).unwrap(),
            canonical_dir
        );
        assert_eq!(
            resolve_path_text("папка с пробелом", &canonical_dir, &canonical_dir).unwrap(),
            canonical_folder
        );
        assert_eq!(
            resolve_path_text(
                &canonical_file.to_string_lossy(),
                &canonical_dir,
                &canonical_dir
            )
            .unwrap(),
            canonical_file
        );
        let uri = gio::File::for_path(&folder).uri();
        assert_eq!(
            resolve_path_text(&uri, &canonical_dir, &canonical_dir).unwrap(),
            canonical_folder
        );

        let missing =
            resolve_path_text("нет-такой-папки", &canonical_dir, &canonical_dir).unwrap_err();
        assert!(missing.contains("не найдена"), "{missing}");

        let remote =
            resolve_path_text("smb://server/share", &canonical_dir, &canonical_dir).unwrap_err();
        assert!(remote.contains("локальные"), "{remote}");
    }

    #[test]
    fn breadcrumbs_start_at_deepest_place_and_keep_unicode_names() {
        let home = PathBuf::from("/home/user");
        let documents = home.join("Документы");
        let windows = PathBuf::from("/mnt/windows");
        let anchors = vec![
            BreadcrumbAnchor {
                label: String::from("Домашняя папка"),
                path: home.clone(),
                icon: String::from("user-home"),
            },
            BreadcrumbAnchor {
                label: String::from("Документы"),
                path: documents.clone(),
                icon: String::from("folder-documents"),
            },
            BreadcrumbAnchor {
                label: String::from("Basic data partition"),
                path: windows.clone(),
                icon: String::from("drive-harddisk"),
            },
        ];

        assert_eq!(
            build_breadcrumbs(&documents, &anchors, false),
            vec![(String::from("Документы"), documents.clone())]
        );

        let nested = documents.join("Отчёты").join("2026");
        assert_eq!(
            build_breadcrumbs(&nested, &anchors, false),
            vec![
                (String::from("Документы"), documents.clone()),
                (String::from("Отчёты"), documents.join("Отчёты")),
                (String::from("2026"), nested.clone()),
            ]
        );

        assert_eq!(
            build_breadcrumbs(&home.join("Загрузки"), &anchors, false),
            vec![
                (String::from("Домашняя папка"), home.clone()),
                (String::from("Загрузки"), home.join("Загрузки")),
            ]
        );

        assert_eq!(
            build_breadcrumbs(&windows.join("Users"), &anchors, false),
            vec![
                (String::from("Basic data partition"), windows.clone()),
                (String::from("Users"), windows.join("Users")),
            ]
        );

        assert_eq!(
            build_breadcrumbs(Path::new("/var/log"), &anchors, false),
            vec![
                (String::from("/"), PathBuf::from("/")),
                (String::from("var"), PathBuf::from("/var")),
                (String::from("log"), PathBuf::from("/var/log")),
            ]
        );
    }

    #[test]
    fn current_place_icon_uses_closest_anchor() {
        let anchors = vec![
            BreadcrumbAnchor {
                label: String::from("Домашняя папка"),
                path: PathBuf::from("/home/user"),
                icon: String::from("user-home"),
            },
            BreadcrumbAnchor {
                label: String::from("Документы"),
                path: PathBuf::from("/home/user/Документы"),
                icon: String::from("folder-documents"),
            },
        ];

        assert_eq!(
            current_place_icon(Path::new("/home/user/Документы/2026"), &anchors),
            "folder-documents"
        );
        assert_eq!(
            current_place_icon(Path::new("/home/user/Музыка"), &anchors),
            "user-home"
        );
        assert_eq!(current_place_icon(Path::new("/var"), &anchors), "folder");
    }

    #[test]
    fn mounts_are_parsed_with_octal_escapes() {
        let mounts = "\
/dev/nvme0n1p3 /mnt/windows fuseblk rw,relatime 0 0
/dev/sda1 /media/My\\040Disk ext4 rw 0 0
proc /proc proc rw 0 0
";
        let parsed = parse_mounts(mounts);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].source, "/dev/nvme0n1p3");
        assert_eq!(parsed[0].target, PathBuf::from("/mnt/windows"));
        assert_eq!(parsed[1].target, PathBuf::from("/media/My Disk"));
        assert_eq!(parsed[2].fstype, "proc");
        assert_eq!(unescape_mount_field("/a\\040b\\134c"), "/a b\\c");
        assert_eq!(decode_by_label("My\\x20Disk"), "My Disk");
        assert!(is_pseudo_filesystem("proc"));
        assert!(!is_pseudo_filesystem("ext4"));
        assert!(is_system_mount(Path::new("/boot/efi")));
    }

    #[test]
    fn recent_paths_are_deduplicated_and_limited() {
        let paths = vec![
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/a"),
            PathBuf::new(),
            PathBuf::from("/c"),
        ];
        assert_eq!(
            deduplicate_paths(&paths, 2),
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
    }

    #[test]
    fn normalization_repairs_corrupt_values() {
        let mut settings = Settings::defaults();
        settings.sidebar_width = 10;
        settings.sidebar_icon_size = 99;
        settings.window_width = 10;
        settings.window_height = 10;
        settings.hidden_sections = vec![SectionId::Devices, SectionId::Devices];
        settings.hidden_places = vec![
            String::from("missing"),
            String::from("home"),
            String::from("home"),
            String::from("network"),
            String::from("recent-files"),
            String::from("device:/mnt/windows"),
            String::from("device:/mnt/windows"),
        ];
        settings.places_order = vec![
            String::from("home"),
            String::from("home"),
            String::from("missing"),
            String::from("network"),
        ];
        settings.custom_places = vec![
            CustomPlace {
                id: String::from("home"),
                name: String::new(),
                path: PathBuf::from("/tmp/Мои документы"),
                icon: String::new(),
            },
            CustomPlace {
                id: String::from("home"),
                name: String::from("  Второе место  "),
                path: PathBuf::from("/tmp"),
                icon: String::from("folder-temp"),
            },
        ];
        settings.next_custom_id = 3;
        settings.recent_locations = vec![
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            PathBuf::from("/"),
            PathBuf::from("/"),
        ];

        settings.normalize();

        assert_eq!(settings.sidebar_width, MIN_SIDEBAR_WIDTH);
        assert_eq!(settings.sidebar_icon_size, DEFAULT_ICON_SIZE);
        assert_eq!(settings.window_width, DEFAULT_WINDOW_WIDTH);
        assert_eq!(settings.window_height, DEFAULT_WINDOW_HEIGHT);
        assert_eq!(settings.hidden_sections, vec![SectionId::Devices]);
        assert_eq!(
            settings.hidden_places,
            vec![
                String::from("home"),
                String::from("network"),
                String::from("recent-files"),
                String::from("device:/mnt/windows"),
            ]
        );
        assert!(settings.places_order.contains(&String::from("custom:3")));
        assert!(settings.places_order.contains(&String::from("custom:4")));
        assert!(!settings.places_order.contains(&String::from("network")));
        assert_eq!(settings.places_order.len(), DEFAULT_PLACE_IDS.len() + 2);
        assert_eq!(settings.custom_places[0].id, "custom:3");
        assert_eq!(settings.custom_places[0].name, "Мои документы");
        assert_eq!(settings.custom_places[0].icon, "folder");
        assert_eq!(settings.custom_places[1].id, "custom:4");
        assert_eq!(settings.custom_places[1].name, "Второе место");
        assert_eq!(settings.next_custom_id, 5);
        assert_eq!(
            settings.recent_locations,
            vec![PathBuf::from("/tmp"), PathBuf::from("/")]
        );
    }

    #[test]
    fn sidebar_uses_fixed_sections_with_headers_on_first_row() {
        let mut settings = Settings::defaults();
        settings.hidden_places.clear();
        settings.hidden_sections.clear();
        let devices = vec![DeviceSpec {
            mount: PathBuf::from("/mnt/windows"),
            label: String::from("Windows"),
            used_fraction: Some(0.5),
            tooltip: String::from("Windows"),
        }];

        let rows = compose_sidebar_rows(
            &settings,
            &devices,
            false,
            Path::new("/definitely/not/a/place"),
        );
        let sections: Vec<SectionId> =
            rows.iter()
                .map(|row| row.section)
                .fold(Vec::new(), |mut result, section| {
                    if result.last() != Some(&section) {
                        result.push(section);
                    }
                    result
                });
        assert_eq!(sections, SectionId::ALL.to_vec());

        let mut seen = Vec::new();
        for row in &rows {
            if seen.contains(&row.section) {
                assert!(
                    row.header.is_none(),
                    "duplicate header for {:?}",
                    row.section
                );
            } else {
                assert_eq!(row.header, Some(row.section));
                seen.push(row.section);
            }
        }
    }

    #[test]
    fn places_order_can_move_entries_before_and_after_a_target() {
        let mut order = ["home", "desktop", "documents", "downloads"]
            .iter()
            .map(|id| (*id).to_string())
            .collect::<Vec<_>>();

        assert!(reorder_place_ids(&mut order, "home", "documents", true));
        assert_eq!(order, ["desktop", "documents", "home", "downloads"]);

        assert!(reorder_place_ids(&mut order, "downloads", "desktop", false));
        assert_eq!(order, ["downloads", "desktop", "documents", "home"]);

        assert!(!reorder_place_ids(&mut order, "home", "home", true));
        assert!(!reorder_place_ids(&mut order, "missing", "home", true));
        assert!(!reorder_place_ids(&mut order, "home", "network", false));
    }

    #[test]
    fn hidden_entries_and_sections_stay_out_unless_show_all_or_current() {
        let mut settings = Settings::defaults();
        settings.hidden_places = vec![String::from("home"), String::from("documents")];
        settings.hidden_sections = vec![SectionId::Remote];

        let hidden_rows =
            compose_sidebar_rows(&settings, &[], false, Path::new("/definitely/not/a/place"));
        assert!(!hidden_rows.iter().any(|row| row.place.id == "home"));
        assert!(!hidden_rows.iter().any(|row| row.place.id == "network"));
        assert!(!hidden_rows
            .iter()
            .any(|row| row.section == SectionId::Remote));

        // The current place remains reachable even when it is hidden.
        let home = glib::home_dir();
        let current_rows = compose_sidebar_rows(&settings, &[], false, &home);
        let home_row = current_rows
            .iter()
            .find(|row| row.place.id == "home")
            .expect("current hidden home place");
        assert!(home_row.hidden);
        assert_eq!(home_row.header, Some(SectionId::Places));

        let all_rows = compose_sidebar_rows(&settings, &[], true, &home);
        let home_row = all_rows
            .iter()
            .find(|row| row.place.id == "home")
            .expect("home in show-all mode");
        assert!(home_row.hidden);
        let network_row = all_rows
            .iter()
            .find(|row| row.place.id == "network")
            .expect("network in show-all mode");
        assert!(network_row.section_hidden);
        assert_eq!(
            section_header_text(SectionId::Remote, true),
            "В сети (скрыт)"
        );
    }

    #[test]
    fn full_path_breadcrumbs_and_anchors_ignore_visibility() {
        let mut settings = Settings::defaults();
        settings.sidebar_visible = false;
        settings.hidden_sections = SectionId::ALL.to_vec();
        settings.custom_places.push(CustomPlace {
            id: String::from("custom:5"),
            name: String::from("Временное"),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder-temp"),
        });
        settings.places_order.push(String::from("custom:5"));
        settings.normalize();

        let anchors = path_anchors(&settings, &[]);
        assert!(anchors
            .iter()
            .any(|anchor| anchor.path == Path::new("/tmp") && anchor.label == "Временное"));

        let path = PathBuf::from("/tmp/Отчёты/2026");
        assert_eq!(
            build_breadcrumbs(&path, &anchors, true),
            vec![
                (String::from("/"), PathBuf::from("/")),
                (String::from("tmp"), PathBuf::from("/tmp")),
                (String::from("Отчёты"), PathBuf::from("/tmp/Отчёты")),
                (String::from("2026"), path.clone()),
            ]
        );
    }

    #[test]
    fn edited_system_place_overrides_defaults_and_persists() {
        let mut settings = Settings::defaults();
        settings.edited_places = vec![
            CustomPlace {
                id: String::from("home"),
                name: String::from("  Личное  "),
                path: PathBuf::from("/tmp"),
                icon: String::from("  folder-home  "),
            },
            CustomPlace {
                id: String::from("home"),
                name: String::from("Дубликат"),
                path: PathBuf::from("/"),
                icon: String::from("folder"),
            },
            CustomPlace {
                id: String::from("bogus"),
                name: String::from("Неизвестно"),
                path: PathBuf::from("/tmp"),
                icon: String::from("folder"),
            },
            CustomPlace {
                id: String::from("custom:1"),
                name: String::from("Своя запись"),
                path: PathBuf::from("/tmp"),
                icon: String::from("folder"),
            },
        ];
        settings.normalize();

        assert_eq!(settings.edited_places.len(), 1);
        assert_eq!(settings.edited_places[0].name, "Личное");
        assert_eq!(settings.edited_places[0].icon, "folder-home");

        let places = default_places(&settings);
        let home = places
            .iter()
            .find(|place| place.id == "home")
            .expect("home place");
        assert_eq!(home.label, "Личное");
        assert_eq!(home.icon, "folder-home");
        match &home.target {
            PlaceTarget::Directory(path) => assert_eq!(path, Path::new("/tmp")),
            _ => panic!("edited place lost its directory target"),
        }

        let dir = TempDir::new("edited-places");
        let path = dir.path.join("dolphy.conf");
        settings.save_to_path(&path).expect("save settings");
        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded.edited_places, settings.edited_places);
    }

    #[test]
    fn icon_size_accepts_supported_values_including_48() {
        let mut settings = Settings::defaults();
        settings.sidebar_icon_size = 48;
        settings.normalize();
        assert_eq!(settings.sidebar_icon_size, 48);
        settings.sidebar_icon_size = 17;
        settings.normalize();
        assert_eq!(settings.sidebar_icon_size, DEFAULT_ICON_SIZE);
    }

    #[test]
    fn sidebar_width_ignores_degenerate_and_extreme_paned_positions() {
        assert_eq!(sidebar_width_from_paned_position(0), None);
        assert_eq!(
            sidebar_width_from_paned_position(MIN_SIDEBAR_WIDTH - 1),
            None
        );
        assert_eq!(
            sidebar_width_from_paned_position(MIN_SIDEBAR_WIDTH),
            Some(MIN_SIDEBAR_WIDTH)
        );
        assert_eq!(sidebar_width_from_paned_position(300), Some(300));
        assert_eq!(sidebar_width_from_paned_position(9999), Some(4000));
    }

    #[test]
    fn navigator_history_truncates_forward_entries() {
        let mut navigator = Navigator::new(PathBuf::from("/a"), false);
        assert!(!navigator.can_go_back());
        assert!(!navigator.can_go_forward());

        navigator.navigate_to(PathBuf::from("/a/b"));
        navigator.navigate_to(PathBuf::from("/a/b/c"));
        assert!(navigator.can_go_back());
        assert_eq!(navigator.current(), Path::new("/a/b/c"));

        assert!(navigator.go_back());
        assert_eq!(navigator.current(), Path::new("/a/b"));
        assert!(navigator.can_go_forward());

        navigator.navigate_to(PathBuf::from("/other"));
        assert_eq!(navigator.current(), Path::new("/other"));
        assert!(!navigator.can_go_forward());
        assert!(navigator.can_go_up());
        assert!(navigator.go_up());
        assert_eq!(navigator.current(), Path::new("/"));
        assert!(!navigator.can_go_up());
    }

    #[test]
    fn formatting_helpers_keep_russian_plural_rules() {
        assert_eq!(plural_ru(1, "объект", "объекта", "объектов"), "1 объект");
        assert_eq!(plural_ru(2, "объект", "объекта", "объектов"), "2 объекта");
        assert_eq!(plural_ru(5, "объект", "объекта", "объектов"), "5 объектов");
        assert_eq!(
            plural_ru(11, "объект", "объекта", "объектов"),
            "11 объектов"
        );
        assert_eq!(plural_ru(21, "объект", "объекта", "объектов"), "21 объект");
        assert_eq!(human_size(0), "0 Б");
        assert_eq!(human_size(1024), "1.0 КиБ");
        assert_eq!(human_size(1536), "1.5 КиБ");
    }

    #[test]
    fn config_with_wrong_value_types_uses_defaults_for_those_keys() {
        let dir = TempDir::new("types");
        let path = dir.path.join("dolphy.conf");
        std::fs::write(
            &path,
            "[General]\nversion=1\nshow_hidden=maybe\n[Sidebar]\nwidth=wide\nicon_size=17\nvisible=perhaps\n",
        )
        .expect("write config");

        let loaded = Settings::load_from_path(&path);
        assert_eq!(loaded.sidebar_width, DEFAULT_SIDEBAR_WIDTH);
        assert_eq!(loaded.sidebar_icon_size, DEFAULT_ICON_SIZE);
        assert!(!loaded.show_hidden);
        assert!(loaded.sidebar_visible);
    }
}
