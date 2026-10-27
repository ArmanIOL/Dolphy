use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{
    Box as GtkBox, Image, Label, ListBox, ListBoxRow, Orientation, Popover, ProgressBar,
    ScrolledWindow, SelectionMode,
};
use pango::EllipsizeMode;

use crate::app::MIN_SIDEBAR_WIDTH;
use crate::filesystem::collect_devices;
use crate::icons::resolved_icon_name;
use crate::places::{
    compose_sidebar_rows, places_for_section, reorder_place_ids, PlaceTarget, SidebarRow,
};
use crate::settings::SectionId;
use crate::ui::menu::{menu_row, menu_separator};
use crate::ui::window::Browser;
use crate::ui::RecentKind;
use crate::util::detach_children;

pub(crate) struct Sidebar {
    pub(crate) widget: ScrolledWindow,
    pub(crate) list: ListBox,
    pub(crate) rows: RefCell<Vec<SidebarRow>>,
}

pub(crate) fn build_sidebar() -> Sidebar {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .activate_on_single_click(false)
        .build();
    list.add_css_class("sidebar");

    let widget = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();
    widget.add_css_class("sidebar-scroller");

    Sidebar {
        widget,
        list,
        rows: RefCell::new(Vec::new()),
    }
}

impl Sidebar {
    pub(crate) fn render(&self, rows: &[SidebarRow], icon_size: i32) {
        detach_children(&self.list);

        for row in rows {
            let widget = build_sidebar_place_row(row, icon_size);
            self.list.append(&widget);
        }

        *self.rows.borrow_mut() = rows.to_vec();
    }

    pub(crate) fn row(&self, index: i32) -> Option<SidebarRow> {
        self.rows.borrow().get(index as usize).cloned()
    }
}

pub(crate) fn sidebar_click_over_header(
    row: &ListBoxRow,
    reference: &impl IsA<gtk::Widget>,
    y: f64,
) -> bool {
    let Some(content) = row.child() else {
        return false;
    };
    let Some(first) = content.first_child() else {
        return false;
    };
    if !first.has_css_class("group-label") {
        return false;
    }
    // The header is the first child of the row. Include the area above it
    // (row padding and the header's top margin) in the inert header zone.
    first
        .compute_bounds(reference)
        .is_some_and(|bounds| y < (bounds.y() + bounds.height() + 2.0) as f64)
}

pub(crate) fn section_header_text(section: SectionId, hidden: bool) -> String {
    if hidden {
        format!("{} (скрыт)", section.title())
    } else {
        section.title().to_string()
    }
}

pub(crate) fn build_sidebar_header_label(section: SectionId, hidden: bool) -> Label {
    let text = section_header_text(section, hidden);
    let label = Label::new(Some(&text));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);
    label.add_css_class("group-label");
    label.set_tooltip_text(Some(section.title()));
    label
}

pub(crate) fn build_sidebar_place_row(row: &SidebarRow, icon_size: i32) -> ListBoxRow {
    let place = &row.place;
    let content = GtkBox::new(Orientation::Vertical, 0);
    content.set_hexpand(true);

    if let Some(section) = row.header {
        content.append(&build_sidebar_header_label(section, row.section_hidden));
    }

    let line = GtkBox::new(Orientation::Horizontal, 4);
    line.set_hexpand(true);
    line.add_css_class("place-line");

    let icon = Image::from_icon_name(&resolved_icon_name(&place.icon));
    icon.set_pixel_size(icon_size);
    icon.set_valign(gtk::Align::Center);

    let label = Label::new(Some(&place.label));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);

    line.append(&icon);
    line.append(&label);
    content.append(&line);

    if let Some(fraction) = place.capacity {
        let bar = ProgressBar::new();
        bar.set_show_text(false);
        bar.set_fraction(fraction);
        bar.set_valign(gtk::Align::Center);
        bar.add_css_class("capacity-bar");
        bar.set_margin_start(icon_size + 4);
        bar.set_margin_top(1);
        bar.set_margin_end(4);
        content.append(&bar);
    }

    let widget = ListBoxRow::new();
    widget.set_child(Some(&content));
    if row.hidden || row.section_hidden {
        widget.set_opacity(0.6);
    }
    let tooltip = place.tooltip.clone().unwrap_or_else(|| place.label.clone());
    widget.set_tooltip_text(Some(&tooltip));
    widget
}

/// GTK also changes `GtkPaned::position` when the window is laid out or
/// clamped, and that must not be mistaken for a user resize. During a real
/// handle drag the primary mouse button is held down, which is visible on the
/// seat's pointer device independently of where the pointer currently is.
pub(crate) fn sidebar_resize_is_user_drag() -> bool {
    gtk::gdk::Display::default()
        .and_then(|display| display.default_seat())
        .and_then(|seat| seat.pointer())
        .is_some_and(|pointer| {
            pointer
                .modifier_state()
                .contains(gtk::gdk::ModifierType::BUTTON1_MASK)
        })
}

pub(crate) fn sidebar_width_from_paned_position(position: i32) -> Option<i32> {
    if position < MIN_SIDEBAR_WIDTH {
        None
    } else {
        Some(position.min(4000))
    }
}

impl Browser {
    pub(crate) fn rebuild_sidebar(self: &Rc<Self>) {
        let devices = collect_devices();
        let (rows, icon_size) = {
            let settings = self.settings.borrow();
            let current = self.state.borrow().current().to_path_buf();
            (
                compose_sidebar_rows(&settings, &devices, self.show_all_places.get(), &current),
                settings.sidebar_icon_size,
            )
        };
        *self.devices.borrow_mut() = devices;
        self.sidebar.render(&rows, icon_size);
        self.attach_sidebar_clicks();
        self.sync_sidebar_selection();
        self.update_places_popover();
    }

    pub(crate) fn attach_sidebar_clicks(self: &Rc<Self>) {
        for index in 0..self.sidebar.rows.borrow().len() {
            let Some(row) = self.sidebar.list.row_at_index(index as i32) else {
                continue;
            };
            let Some(sidebar_row) = self.sidebar.row(index as i32) else {
                continue;
            };
            let Some(content) = row
                .child()
                .and_then(|child| child.downcast::<GtkBox>().ok())
            else {
                continue;
            };

            // Attach to the whole row content, not to its last child: device
            // rows end with a capacity ProgressBar, and clicks on the icon or
            // label above it must still activate the place. The section header
            // painted inside the first row stays inert, as in KFilePlacesView.
            let weak = Rc::downgrade(self);
            let click_content = content.clone();
            let gesture = gtk::GestureClick::new();
            gesture.set_button(1);
            gesture.connect_released(move |_, _, _, y| {
                let Some(browser) = weak.upgrade() else {
                    return;
                };
                let Some(row) = browser.sidebar.list.row_at_index(index as i32) else {
                    return;
                };
                browser.last_sidebar_mouse_press.set(Some(Instant::now()));
                if sidebar_click_over_header(&row, &click_content, y) {
                    return;
                }
                browser.sidebar.list.select_row(Some(&row));
                browser.activate_sidebar_place(index as i32);
            });
            content.add_controller(gesture);

            if sidebar_row.section == SectionId::Places {
                let drag_source = gtk::DragSource::new();
                drag_source.set_actions(gtk::gdk::DragAction::MOVE);
                let dragged_id = sidebar_row.place.id.clone();
                drag_source.connect_prepare(move |_, _, _| {
                    Some(gtk::gdk::ContentProvider::for_value(&glib::Value::from(
                        dragged_id.clone(),
                    )))
                });
                content.add_controller(drag_source);

                let target_id = sidebar_row.place.id.clone();
                let drop_target =
                    gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
                let target_row = row.clone();
                drop_target.connect_enter(move |_, _, _| {
                    target_row.add_css_class("drop-target");
                    gtk::gdk::DragAction::MOVE
                });
                let target_row = row.clone();
                drop_target.connect_leave(move |_| {
                    target_row.remove_css_class("drop-target");
                });
                let weak = Rc::downgrade(self);
                let target_row = row.clone();
                let drop_content = content.clone();
                drop_target.connect_drop(move |_, value, _, y| {
                    target_row.remove_css_class("drop-target");
                    let Some(browser) = weak.upgrade() else {
                        return false;
                    };
                    let Ok(dragged_id) = value.get::<String>() else {
                        return false;
                    };
                    let after = y >= drop_content.height() as f64 / 2.0;
                    browser.reorder_place(&dragged_id, &target_id, after)
                });
                content.add_controller(drop_target);
            }
        }
    }

    pub(crate) fn reorder_place(
        self: &Rc<Self>,
        dragged_id: &str,
        target_id: &str,
        after: bool,
    ) -> bool {
        let changed = reorder_place_ids(
            &mut self.settings.borrow_mut().places_order,
            dragged_id,
            target_id,
            after,
        );
        if changed {
            self.save_settings_now();
            self.rebuild_sidebar();
        }
        changed
    }

    pub(crate) fn activate_sidebar_place(self: &Rc<Self>, index: i32) {
        let Some(row) = self.sidebar.row(index) else {
            return;
        };
        self.activate_place_target(row.place.target, Some(index));
    }

    /// Shared entry point for both the Places panel and the path bar's
    /// places popover. `row_index` anchors the recents popover to a sidebar
    /// row and is `None` when the place was chosen from the path bar.
    pub(crate) fn activate_place_target(
        self: &Rc<Self>,
        target: PlaceTarget,
        row_index: Option<i32>,
    ) {
        match target {
            PlaceTarget::Directory(path) => self.navigate(path),
            PlaceTarget::RecentFiles => self.show_recents_popover(RecentKind::Files, row_index),
            PlaceTarget::RecentLocations => {
                self.show_recents_popover(RecentKind::Locations, row_index)
            }
            PlaceTarget::Unavailable(message) => self.status_left.set_text(&message),
        }
    }

    /// Persists a place edit and refreshes every view that derives from the
    /// place list. Both the sidebar and the breadcrumb anchors depend on it.
    pub(crate) fn refresh_places(self: &Rc<Self>) {
        self.save_settings_now();
        self.rebuild_sidebar();
        let path = self.state.borrow().current().to_path_buf();
        self.update_pathbar(&path);
    }

    pub(crate) fn after_places_changed(self: &Rc<Self>, message: &str) {
        self.refresh_places();
        self.status_left.set_text(message);
    }

    pub(crate) fn sync_sidebar_selection(&self) {
        let current = self.state.borrow().current().to_path_buf();
        let mut best: Option<(usize, usize)> = None;
        {
            let rows = self.sidebar.rows.borrow();
            for (index, row) in rows.iter().enumerate() {
                let PlaceTarget::Directory(path) = &row.place.target else {
                    continue;
                };
                if !current.starts_with(path) {
                    continue;
                }
                let depth = path.components().count();
                if best.is_none_or(|(_, best_depth)| depth >= best_depth) {
                    best = Some((index, depth));
                }
            }
        }

        match best.and_then(|(index, _)| self.sidebar.list.row_at_index(index as i32)) {
            Some(row) => self.sidebar.list.select_row(Some(&row)),
            None => self.sidebar.list.unselect_all(),
        }
    }

    pub(crate) fn show_sidebar_context(
        self: &Rc<Self>,
        row_index: Option<i32>,
        over_header: bool,
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
        content.set_size_request(280, -1);

        let row =
            row_index.and_then(|index| self.sidebar.rows.borrow().get(index as usize).cloned());

        // KFilePlacesView only edits the clicked entry when it is also the
        // current selection. Otherwise the click is treated as empty space.
        let clicked_selected = row_index.is_some()
            && self
                .sidebar
                .list
                .selected_row()
                .map(|selected| selected.index())
                == row_index;
        let item = if over_header || !clicked_selected {
            None
        } else {
            row.as_ref()
        };

        if item.is_none() {
            let add = menu_row("document-new", "Добавить…", true, false, false);
            self.connect_menu_action(&add, &popover, |browser| browser.show_add_place_dialog());
            content.append(&add);
        }

        if let Some(row) = item {
            let place = &row.place;
            if let PlaceTarget::Directory(path) = &place.target {
                let open = menu_row("window-new", "Открыть в новом окне", true, false, false);
                let target = path.clone();
                self.connect_menu_action(&open, &popover, move |browser| {
                    browser.open_place_in_new_window(target.clone())
                });
                content.append(&open);

                let properties = menu_row("document-properties", "Свойства", true, false, false);
                let target = path.clone();
                self.connect_menu_action(&properties, &popover, move |browser| {
                    browser.show_place_properties(&target)
                });
                content.append(&properties);
                content.append(&menu_separator());
            }

            // KFilePlacesView offers Edit for every non-device bookmark,
            // including the standard places. In our model only directory-backed
            // entries can be represented by the edit dialog.
            let editable = !place.id.starts_with("device:")
                && place.id != "trash"
                && matches!(&place.target, PlaceTarget::Directory(_));
            if editable {
                let edit = menu_row("edit-entry", "Изменить…", true, false, false);
                let place_id = place.id.clone();
                self.connect_menu_action(&edit, &popover, move |browser| {
                    browser.show_edit_place_dialog(&place_id)
                });
                content.append(&edit);
            }

            if place.custom {
                let remove = menu_row(
                    "bookmark-remove-symbolic",
                    "Удалить из панели «Точки входа»",
                    true,
                    true,
                    false,
                );
                let place_id = place.id.clone();
                self.connect_menu_action(&remove, &popover, move |browser| {
                    browser.remove_custom_place(&place_id)
                });
                content.append(&remove);
            }

            let hidden = row.hidden;
            let hide = menu_row("hint", "Скрыть", !row.section_hidden, false, hidden);
            let place_id = place.id.clone();
            self.connect_menu_action(&hide, &popover, move |browser| {
                browser.set_place_hidden(&place_id, !hidden)
            });
            content.append(&hide);

            let section = row.section;
            let section_hidden = row.section_hidden;
            let hide_section = menu_row(
                "hint",
                &format!("Скрыть раздел «{}»", section.title()),
                true,
                false,
                section_hidden,
            );
            self.connect_menu_action(&hide_section, &popover, move |browser| {
                browser.set_section_hidden(section, !section_hidden)
            });
            content.append(&hide_section);
        } else if over_header {
            if let Some(row) = &row {
                let section = row.section;
                let section_hidden = row.section_hidden;
                let hide_section = menu_row(
                    "hint",
                    &format!("Скрыть раздел «{}»", section.title()),
                    true,
                    false,
                    section_hidden,
                );
                self.connect_menu_action(&hide_section, &popover, move |browser| {
                    browser.set_section_hidden(section, !section_hidden)
                });
                content.append(&hide_section);
            }
        }

        if item.is_none() {
            if self.hidden_place_count() > 0 {
                let show_all = menu_row(
                    "visibility",
                    "Показать все",
                    true,
                    false,
                    self.show_all_places.get(),
                );
                self.connect_menu_action(&show_all, &popover, |browser| {
                    browser.toggle_show_all_places()
                });
                content.append(&show_all);
            }

            content.append(&menu_separator());
            let heading = Label::new(Some("Размер значков"));
            heading.set_xalign(0.0);
            heading.add_css_class("popover-section");
            content.append(&heading);

            let icon_size = self.settings.borrow().sidebar_icon_size;
            for (size, label) in [
                (16, "Маленькие (16×16)"),
                (22, "Средние (22×22)"),
                (32, "Большие (32×32)"),
                (48, "Огромные (48×48)"),
            ] {
                let button = menu_row("view-list-details", label, true, false, icon_size == size);
                self.connect_menu_action(&button, &popover, move |browser| {
                    browser.set_sidebar_icon_size(size)
                });
                content.append(&button);
            }
        }

        popover.set_child(Some(&content));
        let rectangle = gtk::gdk::Rectangle::new(x, y, 1, 1);
        self.present_popup(&popover, &self.sidebar.list, &rectangle);
    }

    pub(crate) fn hidden_place_count(&self) -> usize {
        let settings = self.settings.borrow();
        let devices = self.devices.borrow();
        let mut count = 0;
        for section in SectionId::ALL {
            let section_hidden = settings.hidden_sections.contains(&section);
            for place in places_for_section(&settings, &devices, section) {
                if section_hidden || settings.hidden_places.contains(&place.id) {
                    count += 1;
                }
            }
        }
        count
    }

    pub(crate) fn set_place_hidden(self: &Rc<Self>, place_id: &str, hidden: bool) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.hidden_places.retain(|id| id != place_id);
            if hidden {
                settings.hidden_places.push(place_id.to_string());
            }
        }
        self.refresh_places();
    }

    pub(crate) fn set_section_hidden(self: &Rc<Self>, section: SectionId, hidden: bool) {
        {
            let mut settings = self.settings.borrow_mut();
            settings
                .hidden_sections
                .retain(|candidate| *candidate != section);
            if hidden {
                settings.hidden_sections.push(section);
            }
        }
        self.refresh_places();
    }

    pub(crate) fn toggle_show_all_places(self: &Rc<Self>) {
        let show_all = !self.show_all_places.get();
        self.show_all_places.set(show_all);
        self.rebuild_sidebar();
        self.status_left.set_text(if show_all {
            "Показаны все скрытые записи"
        } else {
            "Скрытые записи снова скрыты"
        });
    }

    pub(crate) fn set_sidebar_icon_size(self: &Rc<Self>, size: i32) {
        if !matches!(size, 16 | 22 | 32 | 48) {
            return;
        }
        {
            let mut settings = self.settings.borrow_mut();
            if settings.sidebar_icon_size == size {
                return;
            }
            settings.sidebar_icon_size = size;
        }
        self.save_settings_now();
        self.rebuild_sidebar();
        self.status_left
            .set_text(&format!("Размер значков боковой панели: {size} px"));
    }
}

impl Browser {
    pub(crate) fn connect_sidebar_signals(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.sidebar.list.connect_row_activated(move |_, row| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            // A left click is handled by our own gesture, including the
            // "inert section header" rule. Suppress the activation that GTK
            // additionally emits for a double click or for a header row, but
            // keep keyboard activation working.
            if browser
                .last_sidebar_mouse_press
                .get()
                .is_some_and(|instant| instant.elapsed() < Duration::from_millis(600))
            {
                return;
            }
            browser.activate_sidebar_place(row.index());
        });

        let weak = Rc::downgrade(self);
        let context_gesture = gtk::GestureClick::new();
        context_gesture.set_button(3);
        context_gesture.connect_pressed(move |gesture, _, x, y| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let Some(widget) = gesture.widget() else {
                return;
            };
            let row_index = widget
                .downcast_ref::<ListBox>()
                .and_then(|list| list.row_at_y(y as i32))
                .map(|row| row.index());
            let over_header = widget
                .downcast_ref::<ListBox>()
                .and_then(|list| list.row_at_y(y as i32))
                .is_some_and(|row| {
                    widget
                        .downcast_ref::<ListBox>()
                        .is_some_and(|list| sidebar_click_over_header(&row, list, y))
                });
            browser.show_sidebar_context(row_index, over_header, x as i32, y as i32);
        });
        self.sidebar.list.add_controller(context_gesture);
    }
}
