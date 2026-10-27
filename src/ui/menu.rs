use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Image, Label, Orientation, Popover, Separator};
use pango::EllipsizeMode;

use crate::ui::window::Browser;

pub(crate) fn menu_row(
    icon_name: &str,
    label: &str,
    sensitive: bool,
    danger: bool,
    checked: bool,
) -> Button {
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);

    let text = Label::new(Some(label));
    text.set_xalign(0.0);
    text.set_hexpand(true);
    text.set_ellipsize(EllipsizeMode::End);

    content.append(&icon);
    content.append(&text);
    if checked {
        let check = Label::new(Some("✓"));
        check.add_css_class("menu-check");
        content.append(&check);
    }

    let button = Button::new();
    button.set_child(Some(&content));
    button.add_css_class("flat");
    button.add_css_class("menu-row");
    if danger {
        button.add_css_class("danger");
    }
    button.set_sensitive(sensitive);
    button
}

pub(crate) fn recent_row(icon_name: &str, name: &str, subtitle: &str, tooltip: &str) -> Button {
    let content = GtkBox::new(Orientation::Horizontal, 8);
    content.set_hexpand(true);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);
    icon.set_valign(gtk::Align::Center);

    let text = GtkBox::new(Orientation::Vertical, 0);
    text.set_hexpand(true);
    let name_label = Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.set_ellipsize(EllipsizeMode::End);
    name_label.add_css_class("recent-name");
    let path_label = Label::new(Some(subtitle));
    path_label.set_xalign(0.0);
    path_label.set_ellipsize(EllipsizeMode::Middle);
    path_label.add_css_class("recent-path");
    text.append(&name_label);
    text.append(&path_label);

    content.append(&icon);
    content.append(&text);

    let button = Button::new();
    button.set_child(Some(&content));
    button.add_css_class("flat");
    button.add_css_class("menu-row");
    button.set_tooltip_text(Some(tooltip));
    button
}

pub(crate) fn menu_separator() -> Separator {
    let separator = Separator::new(Orientation::Horizontal);
    separator.add_css_class("menu-separator");
    separator
}

impl Browser {
    pub(crate) fn present_popup(
        &self,
        popover: &Popover,
        parent: &impl IsA<gtk::Widget>,
        rectangle: &gtk::gdk::Rectangle,
    ) {
        if let Some(previous) = self.popup.borrow_mut().take() {
            previous.popdown();
            previous.unparent();
        }
        if popover.parent().is_some() {
            popover.unparent();
        }
        popover.set_parent(parent);
        popover.set_pointing_to(Some(rectangle));
        popover.popup();
        *self.popup.borrow_mut() = Some(popover.clone());
    }

    pub(crate) fn connect_menu_action<F>(
        self: &Rc<Self>,
        button: &Button,
        popover: &Popover,
        action: F,
    ) where
        F: Fn(&Rc<Self>) + 'static,
    {
        let weak = Rc::downgrade(self);
        let popover_weak = popover.downgrade();
        button.connect_clicked(move |_| {
            if let Some(popover) = popover_weak.upgrade() {
                popover.popdown();
            }
            if let Some(browser) = weak.upgrade() {
                action(&browser);
            }
        });
    }
}

impl Browser {
    pub(crate) fn install_menu(&self) {
        let menu = gio::Menu::new();

        let file_section = gio::Menu::new();
        file_section.append(Some("Новое окно"), Some("win.new-window"));
        file_section.append(Some("Обновить"), Some("win.refresh"));
        menu.append_section(None, &file_section);

        let view_section = gio::Menu::new();
        view_section.append(Some("Изменить путь"), Some("win.edit-location"));
        view_section.append(Some("Показывать скрытые файлы"), Some("win.show-hidden"));
        view_section.append(Some("Боковая панель"), Some("win.toggle-sidebar"));
        menu.append_section(None, &view_section);

        let help_section = gio::Menu::new();
        help_section.append(Some("Поиск"), Some("win.search"));
        help_section.append(Some("О программе"), Some("win.about"));
        menu.append_section(None, &help_section);

        self.toolbar.menu_button.set_menu_model(Some(&menu));
    }
}
