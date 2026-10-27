use std::path::Path;

pub(super) fn key_file_bool(
    key_file: &glib::KeyFile,
    group: &str,
    key: &str,
    default: bool,
) -> bool {
    key_file.boolean(group, key).unwrap_or(default)
}

pub(super) fn backup_config(path: &Path) {
    let backup = path.with_file_name(format!(
        "{}.bak",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dolphy.conf")
    ));
    let _ = std::fs::rename(path, &backup);
}

pub(super) fn key_file_i32(key_file: &glib::KeyFile, group: &str, key: &str, default: i32) -> i32 {
    key_file.integer(group, key).unwrap_or(default)
}

pub(super) fn key_file_string(key_file: &glib::KeyFile, group: &str, key: &str) -> String {
    key_file
        .string(group, key)
        .map(|value| value.to_string())
        .unwrap_or_default()
}

pub(super) fn read_indexed(key_file: &glib::KeyFile, group: &str, prefix: &str) -> Vec<String> {
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

pub(super) fn write_indexed(
    key_file: &glib::KeyFile,
    group: &str,
    prefix: &str,
    values: &[String],
) {
    key_file.set_integer(group, &format!("{prefix}_count"), values.len() as i32);
    for (index, value) in values.iter().enumerate() {
        key_file.set_string(group, &format!("{prefix}_{index}"), value);
    }
}
