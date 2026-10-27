use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use gtk::prelude::*;

use crate::util::{human_size, plural_ru};

pub(crate) struct FileData {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) kind_text: String,
    pub(crate) size_text: String,
    pub(crate) modified_text: String,
    pub(crate) icon: gio::Icon,
}

pub(crate) fn item_data(object: &glib::Object) -> Option<Rc<FileData>> {
    let boxed = object.downcast_ref::<glib::BoxedAnyObject>()?;
    let borrowed = boxed.borrow::<Rc<FileData>>();
    Some((*borrowed).clone())
}

pub(crate) fn describe(data: &FileData) -> String {
    format!("{} ({}, {})", data.name, data.kind_text, data.size_text)
}

pub(crate) struct DirEntryData {
    pub(crate) is_dir: bool,
    pub(crate) size: u64,
    pub(crate) data: Rc<FileData>,
}

pub(crate) fn read_directory(path: &Path, show_hidden: bool) -> std::io::Result<Vec<DirEntryData>> {
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
