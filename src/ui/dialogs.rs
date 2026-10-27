use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Entry, EventControllerKey, Label, Orientation, Popover};

use crate::app::{APP_NAME, VERSION};
use crate::navigation::resolve_path_text;
use crate::places::{default_places, PlaceTarget};
use crate::settings::CustomPlace;
use crate::ui::menu::recent_row;
use crate::ui::window::Browser;
use crate::ui::RecentKind;
use crate::util::human_size;

fn normalized_place_name(name: String, path: &Path) -> String {
    if name.trim().is_empty() {
        path.file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    } else {
        name.trim().to_string()
    }
}

impl Browser {
    pub(crate) fn show_about_dialog(&self) {
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

    pub(crate) fn open_place_in_new_window(self: &Rc<Self>, path: PathBuf) {
        let Some(app) = self.window.application() else {
            return;
        };
        let browser = Browser::new(&app, Rc::clone(&self.settings));
        browser.navigate(path);
    }

    pub(crate) fn show_place_properties(self: &Rc<Self>, path: &Path) {
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

    fn directory_or_report(&self, path: &Path) -> bool {
        if path.is_dir() {
            return true;
        }
        self.status_left
            .set_text(&format!("Не папка: {}", path.display()));
        false
    }

    pub(crate) fn remove_custom_place(self: &Rc<Self>, place_id: &str) {
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
        self.after_places_changed(&format!("Запись «{name}» удалена"));
    }

    pub(crate) fn add_custom_place(self: &Rc<Self>, name: String, path: PathBuf, icon: String) {
        if !self.directory_or_report(&path) {
            return;
        }
        let name = normalized_place_name(name, &path);

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
        self.after_places_changed(&format!("Добавлена запись «{name}»"));
    }

    pub(crate) fn update_custom_place(
        self: &Rc<Self>,
        place_id: &str,
        name: String,
        path: PathBuf,
        icon: String,
    ) {
        if !self.directory_or_report(&path) {
            return;
        }
        let name = normalized_place_name(name, &path);

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
        self.after_places_changed(&format!("Запись «{name}» изменена"));
    }

    pub(crate) fn update_system_place(
        self: &Rc<Self>,
        place_id: &str,
        name: String,
        path: PathBuf,
        icon: String,
    ) {
        if !self.directory_or_report(&path) {
            return;
        }
        let name = normalized_place_name(name, &path);

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
        self.after_places_changed(&format!("Запись «{name}» изменена"));
    }

    pub(crate) fn update_place(
        self: &Rc<Self>,
        place_id: &str,
        name: String,
        path: PathBuf,
        icon: String,
    ) {
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

    pub(crate) fn show_add_place_dialog(self: &Rc<Self>) {
        self.show_place_dialog(None);
    }

    pub(crate) fn show_edit_place_dialog(self: &Rc<Self>, place_id: &str) {
        self.show_place_dialog(Some(place_id.to_string()));
    }

    pub(crate) fn show_place_dialog(self: &Rc<Self>, edit_id: Option<String>) {
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

    pub(crate) fn show_recents_popover(self: &Rc<Self>, kind: RecentKind, row_index: Option<i32>) {
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

    pub(crate) fn open_recent_file(self: &Rc<Self>, path: &Path) {
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

    pub(crate) fn open_recent_location(self: &Rc<Self>, path: &Path) {
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
}
