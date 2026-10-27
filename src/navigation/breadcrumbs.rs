use std::path::{Component, Path, PathBuf};

use crate::filesystem::DeviceSpec;
use crate::places::{network_place, ordered_places, PlaceTarget};
use crate::settings::Settings;

#[derive(Clone)]
pub(crate) struct BreadcrumbAnchor {
    pub(crate) label: String,
    pub(crate) path: PathBuf,
    pub(crate) icon: String,
}

pub(crate) fn path_anchors(settings: &Settings, devices: &[DeviceSpec]) -> Vec<BreadcrumbAnchor> {
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

pub(crate) fn build_breadcrumbs(
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

pub(crate) fn current_place_icon(path: &Path, anchors: &[BreadcrumbAnchor]) -> String {
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
