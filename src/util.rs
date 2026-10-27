use std::path::PathBuf;

use gtk::prelude::*;

/// Detaches every child widget. Unparenting is used instead of container-specific
/// removal so that it stays correct even if a widget has already been reparented.
pub(crate) fn detach_children(container: &impl IsA<gtk::Widget>) {
    let mut child = container.first_child();
    while let Some(current) = child {
        child = current.next_sibling();
        current.unparent();
    }
}

pub(crate) fn deduplicate_paths(paths: &[PathBuf], limit: usize) -> Vec<PathBuf> {
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

pub(crate) fn human_size(bytes: u64) -> String {
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

pub(crate) fn plural_ru(count: usize, one: &str, few: &str, many: &str) -> String {
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
