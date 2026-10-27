use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Button, Entry, EntryIconPosition, EventControllerKey, Image, Label, ListBox,
    ListBoxRow, MenuButton, Orientation, PickFlags, Popover, ScrolledWindow, SelectionMode, Stack,
};
use pango::EllipsizeMode;

use crate::icons::resolved_icon_name;
use crate::navigation::{build_breadcrumbs, current_place_icon, path_anchors, resolve_path_text};
use crate::places::{places_for_section, PlaceTarget};
use crate::settings::SectionId;
use crate::ui::menu::{menu_row, menu_separator};
use crate::ui::window::Browser;
use crate::util::detach_children;

pub(crate) struct PathBar {
    pub(crate) widget: GtkBox,
    pub(crate) places_button: MenuButton,
    pub(crate) places_popover: Popover,
    pub(crate) places_list: ListBox,
    pub(crate) places_targets: RefCell<Vec<Option<PlaceTarget>>>,
    pub(crate) stack: Stack,
    pub(crate) crumb_scroller: ScrolledWindow,
    pub(crate) crumbs: GtkBox,
    pub(crate) filler: Button,
    pub(crate) entry: Entry,
    pub(crate) edit_button: Button,
    pub(crate) editing: Cell<bool>,
}

pub(crate) fn build_path_bar() -> Rc<PathBar> {
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
    pub(crate) fn set_place_icon(&self, icon_name: &str) {
        self.places_button
            .set_icon_name(&resolved_icon_name(icon_name));
    }

    pub(crate) fn set_place_tooltip(&self, label: &str) {
        self.places_button
            .set_tooltip_text(Some(&format!("Места — {label}")));
    }

    pub(crate) fn begin_edit(&self, path: &Path) {
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

    pub(crate) fn end_edit(&self) {
        self.editing.set(false);
        self.stack.set_visible_child_name("crumbs");
        self.edit_button.set_icon_name("open-for-editing");
        self.edit_button
            .set_tooltip_text(Some("Редактировать путь"));
        self.set_error(None);
    }

    pub(crate) fn set_error(&self, message: Option<&str>) {
        if message.is_some() {
            self.entry.add_css_class("error");
        } else {
            self.entry.remove_css_class("error");
        }
        self.entry.set_tooltip_text(message);
    }

    pub(crate) fn is_editing(&self) -> bool {
        self.editing.get()
    }
}

impl Browser {
    pub(crate) fn update_pathbar(self: &Rc<Self>, path: &Path) {
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

    pub(crate) fn begin_path_edit(self: &Rc<Self>) {
        let path = self.state.borrow().current().to_path_buf();
        if self.toolbar.pathbar.is_editing() {
            self.toolbar.pathbar.entry.grab_focus();
            self.toolbar.pathbar.entry.select_region(0, -1);
            return;
        }
        self.toolbar.pathbar.begin_edit(&path);
    }

    pub(crate) fn cancel_path_edit(self: &Rc<Self>) {
        self.toolbar.pathbar.end_edit();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    pub(crate) fn submit_path_bar(self: &Rc<Self>) {
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

    pub(crate) fn show_pathbar_context(
        self: &Rc<Self>,
        widget: &impl IsA<gtk::Widget>,
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

    pub(crate) fn set_show_full_path(self: &Rc<Self>, show_full_path: bool) {
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

    pub(crate) fn update_places_popover(&self) {
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
}

impl Browser {
    pub(crate) fn connect_pathbar_signals(self: &Rc<Self>) {
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
                if let Some(target) = target {
                    browser.activate_place_target(target, None);
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
}
