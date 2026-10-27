use gtk::prelude::*;
use gtk::{
    Box as GtkBox, ColumnView, ColumnViewColumn, Image, Label, Orientation, PopoverMenu,
    ScrolledWindow, SingleSelection,
};
use pango::EllipsizeMode;

use crate::filesystem::item_data;

pub(crate) fn build_file_view(selection: &SingleSelection) -> (ColumnView, ScrolledWindow) {
    let context_menu = gio::Menu::new();
    context_menu.append(Some("Открыть в новой вкладке"), Some("win.ctx-open-tab"));
    context_menu.append(Some("Открыть в новом окне"), Some("win.ctx-open-window"));
    context_menu.append(Some("Вырезать"), Some("win.ctx-cut"));
    context_menu.append(Some("Копировать"), Some("win.ctx-copy"));
    context_menu.append(Some("Удалить в корзину"), Some("win.ctx-trash"));

    let popover = PopoverMenu::from_model(Some(&context_menu));
    popover.set_has_arrow(false);
    popover.add_css_class("context-menu");

    let name_factory = build_name_factory(selection, &popover);
    let size_factory = build_text_factory(true, selection, &popover);
    let date_factory = build_text_factory(false, selection, &popover);

    let name_column = ColumnViewColumn::new(Some("Имя"), Some(name_factory));
    name_column.set_expand(true);
    name_column.set_resizable(true);

    let size_column = ColumnViewColumn::new(Some("Размер"), Some(size_factory));
    size_column.set_fixed_width(110);
    size_column.set_resizable(true);

    let date_column = ColumnViewColumn::new(Some("Дата изменения"), Some(date_factory));
    date_column.set_fixed_width(180);
    date_column.set_resizable(true);

    let column_view = ColumnView::new(Some(selection.clone()));
    column_view.append_column(&name_column);
    column_view.append_column(&size_column);
    column_view.append_column(&date_column);
    column_view.set_show_row_separators(true);
    column_view.set_show_column_separators(true);
    column_view.set_hexpand(true);
    column_view.set_vexpand(true);

    let scroller = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&column_view)
        .build();

    (column_view, scroller)
}

fn build_name_factory(
    selection: &SingleSelection,
    popover: &PopoverMenu,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let selection_setup = selection.clone();
    let popover_setup = popover.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        hbox.set_margin_top(2);
        hbox.set_margin_bottom(2);
        hbox.set_margin_start(4);
        hbox.set_margin_end(4);

        let icon = Image::new();
        icon.set_pixel_size(16);

        let label = Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(EllipsizeMode::Middle);

        hbox.append(&icon);
        hbox.append(&label);
        list_item.set_child(Some(&hbox));

        attach_context_menu(&hbox, list_item, &selection_setup, &popover_setup);
    });

    factory.connect_bind(|_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(data) = list_item.item().as_ref().and_then(item_data) else {
            return;
        };
        let Some(child) = list_item.child() else {
            return;
        };
        let Some(hbox) = child.downcast_ref::<GtkBox>() else {
            return;
        };
        let Some(icon) = hbox.first_child().and_then(|w| w.downcast::<Image>().ok()) else {
            return;
        };
        let Some(label) = hbox.last_child().and_then(|w| w.downcast::<Label>().ok()) else {
            return;
        };

        icon.set_from_gicon(&data.icon);
        label.set_text(&data.name);
    });

    factory
}

fn build_text_factory(
    right_aligned: bool,
    selection: &SingleSelection,
    popover: &PopoverMenu,
) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();

    let selection_setup = selection.clone();
    let popover_setup = popover.clone();
    factory.connect_setup(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };

        let label = Label::new(None);
        label.set_xalign(if right_aligned { 1.0 } else { 0.0 });
        label.set_hexpand(true);
        if right_aligned {
            label.set_halign(gtk::Align::End);
            label.set_margin_end(6);
        } else {
            label.set_halign(gtk::Align::Start);
        }

        list_item.set_child(Some(&label));
        attach_context_menu(&label, list_item, &selection_setup, &popover_setup);
    });

    factory.connect_bind(move |_, object| {
        let Some(list_item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(data) = list_item.item().as_ref().and_then(item_data) else {
            return;
        };
        let Some(child) = list_item.child() else {
            return;
        };
        let Some(label) = child.downcast_ref::<Label>() else {
            return;
        };

        if right_aligned {
            label.set_text(&data.size_text);
        } else {
            label.set_text(&data.modified_text);
        }
    });

    factory
}

fn attach_context_menu<W>(
    widget: &W,
    list_item: &gtk::ListItem,
    selection: &SingleSelection,
    popover: &PopoverMenu,
) where
    W: IsA<gtk::Widget> + 'static,
{
    let gesture = gtk::GestureClick::new();
    gesture.set_button(3);

    let list_item = list_item.clone();
    let selection = selection.clone();
    let popover = popover.clone();

    gesture.connect_pressed(move |gesture, _, x, y| {
        let position = list_item.position();
        if position != u32::MAX {
            selection.set_selected(position);
        }

        let Some(widget) = gesture.widget() else {
            return;
        };

        popover.popdown();
        if popover.parent().is_some() {
            popover.unparent();
        }
        popover.set_parent(&widget);
        popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.popup();
    });

    widget.add_controller(gesture);
}
