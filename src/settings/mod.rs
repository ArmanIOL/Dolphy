use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::app::{
    DEFAULT_ICON_SIZE, DEFAULT_PLACE_IDS, DEFAULT_SIDEBAR_WIDTH, DEFAULT_WINDOW_HEIGHT,
    DEFAULT_WINDOW_WIDTH, MIN_SIDEBAR_WIDTH, RECENT_LIMIT,
};
use crate::util::deduplicate_paths;

mod persistence;

use persistence::{
    backup_config, key_file_bool, key_file_i32, key_file_string, read_indexed, write_indexed,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SectionId {
    Places,
    Remote,
    Recent,
    Devices,
}

impl SectionId {
    pub(crate) const ALL: [Self; 4] = [Self::Places, Self::Remote, Self::Recent, Self::Devices];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Places => "places",
            Self::Remote => "remote",
            Self::Recent => "recent",
            Self::Devices => "devices",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            // Exact KIO Russian translations of KFilePlacesModel's group
            // names ("Places", "Remote", "Recent", "Devices").
            Self::Places => "Точки входа",
            Self::Remote => "В сети",
            Self::Recent => "Недавно изменённые",
            Self::Devices => "Устройства",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
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
pub(crate) struct CustomPlace {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) icon: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Settings {
    pub(crate) sidebar_visible: bool,
    pub(crate) sidebar_width: i32,
    pub(crate) sidebar_icon_size: i32,
    pub(crate) hidden_sections: Vec<SectionId>,
    pub(crate) places_order: Vec<String>,
    pub(crate) hidden_places: Vec<String>,
    pub(crate) custom_places: Vec<CustomPlace>,
    pub(crate) edited_places: Vec<CustomPlace>,
    pub(crate) next_custom_id: u32,
    pub(crate) show_hidden: bool,
    pub(crate) show_full_path: bool,
    pub(crate) window_width: i32,
    pub(crate) window_height: i32,
    pub(crate) last_location: Option<PathBuf>,
    pub(crate) recent_locations: Vec<PathBuf>,
    pub(crate) recent_files: Vec<PathBuf>,
}

impl Settings {
    pub(crate) fn defaults() -> Self {
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

    pub(crate) fn config_path() -> PathBuf {
        if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
            PathBuf::from(dir).join("dolphy").join("dolphy.conf")
        } else {
            glib::home_dir()
                .join(".config")
                .join("dolphy")
                .join("dolphy.conf")
        }
    }

    pub(crate) fn load() -> Self {
        Self::load_from_path(&Self::config_path())
    }

    pub(crate) fn load_from_path(path: &Path) -> Self {
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

    pub(crate) fn save(&self) -> std::io::Result<()> {
        self.save_to_path(&Self::config_path())
    }

    pub(crate) fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
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

    pub(crate) fn normalize(&mut self) {
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
