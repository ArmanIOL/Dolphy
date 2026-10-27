use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Box as GtkBox, Button, Image, Label, MenuButton, Orientation, ToggleButton};

use crate::ui::pathbar::{build_path_bar, PathBar};

pub(crate) struct Toolbar {
    pub(crate) widget: GtkBox,
    pub(crate) back: Button,
    pub(crate) forward: Button,
    pub(crate) up: Button,
    pub(crate) sidebar_toggle: Button,
    pub(crate) split: ToggleButton,
    pub(crate) menu_button: MenuButton,
    pub(crate) pathbar: Rc<PathBar>,
}

fn toolbar_button(icon_name: &str, tooltip: &str, action: &str) -> Button {
    let button = Button::from_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
    button.set_action_name(Some(action));
    button.add_css_class("flat");
    button
}

pub(crate) fn build_toolbar() -> Toolbar {
    let back = toolbar_button("go-previous", "Назад (Alt+Left)", "win.back");
    let forward = toolbar_button("go-next", "Вперёд (Alt+Right)", "win.forward");
    let up = toolbar_button("go-up", "На уровень выше (Alt+Up)", "win.up");

    let sidebar_toggle = Button::from_icon_name("sidebar-show-symbolic");
    sidebar_toggle.set_tooltip_text(Some("Боковая панель (F9)"));
    sidebar_toggle.set_action_name(Some("win.toggle-sidebar"));
    sidebar_toggle.add_css_class("flat");

    let pathbar = build_path_bar();

    let split = ToggleButton::new();
    split.set_tooltip_text(Some("Две панели (скоро)"));
    split.add_css_class("flat");
    let split_content = GtkBox::new(Orientation::Horizontal, 6);
    let split_icon = Image::from_icon_name("view-split-left-right");
    split_icon.set_pixel_size(16);
    split_content.append(&split_icon);
    split_content.append(&Label::new(Some("Две панели")));
    split.set_child(Some(&split_content));

    let search = toolbar_button("system-search", "Поиск (Ctrl+F)", "win.search");

    let menu_button = MenuButton::new();
    menu_button.set_icon_name("application-menu");
    menu_button.set_tooltip_text(Some("Главное меню"));
    menu_button.set_always_show_arrow(false);
    menu_button.add_css_class("flat");

    let widget = GtkBox::new(Orientation::Horizontal, 4);
    widget.add_css_class("toolbar");
    widget.append(&sidebar_toggle);
    widget.append(&back);
    widget.append(&forward);
    widget.append(&up);
    widget.append(&pathbar.widget);
    widget.append(&split);
    widget.append(&search);
    widget.append(&menu_button);

    Toolbar {
        widget,
        back,
        forward,
        up,
        sidebar_toggle,
        split,
        menu_button,
        pathbar,
    }
}
