use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use gio::prelude::*;

use crate::util::human_size;

#[derive(Clone)]
pub(crate) struct DeviceSpec {
    pub(crate) mount: PathBuf,
    pub(crate) label: String,
    pub(crate) used_fraction: Option<f64>,
    pub(crate) tooltip: String,
}

#[derive(Clone)]
pub(crate) struct RawMount {
    pub(crate) source: String,
    pub(crate) target: PathBuf,
    pub(crate) fstype: String,
}

pub(crate) fn parse_mounts(contents: &str) -> Vec<RawMount> {
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

pub(crate) fn unescape_mount_field(value: &str) -> String {
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

pub(crate) fn decode_by_label(value: &str) -> String {
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

pub(crate) fn is_pseudo_filesystem(fstype: &str) -> bool {
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

pub(crate) fn is_system_mount(path: &Path) -> bool {
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

pub(crate) fn collect_devices() -> Vec<DeviceSpec> {
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
