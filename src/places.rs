use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::filesystem::DeviceSpec;
use crate::settings::{SectionId, Settings};

#[derive(Clone)]
pub(crate) enum PlaceTarget {
    Directory(PathBuf),
    RecentFiles,
    RecentLocations,
    Unavailable(String),
}

#[derive(Clone)]
pub(crate) struct PlaceSpec {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) icon: String,
    pub(crate) target: PlaceTarget,
    pub(crate) custom: bool,
    pub(crate) capacity: Option<f64>,
    pub(crate) tooltip: Option<String>,
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

pub(crate) fn network_place() -> PlaceSpec {
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

pub(crate) fn default_places(settings: &Settings) -> Vec<PlaceSpec> {
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

pub(crate) fn ordered_places(settings: &Settings) -> Vec<PlaceSpec> {
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

pub(crate) fn places_for_section(
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
pub(crate) struct SidebarRow {
    pub(crate) place: PlaceSpec,
    pub(crate) section: SectionId,
    pub(crate) header: Option<SectionId>,
    pub(crate) hidden: bool,
    pub(crate) section_hidden: bool,
}

pub(crate) fn compose_sidebar_rows(
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

/// Moves `dragged` before or after `target` in the persisted order used by
/// the Places section. IDs from the fixed sections (network, recents, devices)
/// are never part of this list.
pub(crate) fn reorder_place_ids(
    order: &mut Vec<String>,
    dragged: &str,
    target: &str,
    after: bool,
) -> bool {
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
