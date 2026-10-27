use std::path::{Path, PathBuf};

use gio::prelude::*;

pub(crate) fn resolve_path_text(
    input: &str,
    current: &Path,
    home: &Path,
) -> Result<PathBuf, String> {
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

pub(crate) fn remember_path(list: &mut Vec<PathBuf>, path: &Path, limit: usize) {
    list.retain(|entry| entry != path);
    list.insert(0, path.to_path_buf());
    list.truncate(limit);
}
