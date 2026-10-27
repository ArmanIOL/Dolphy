use std::cell::RefCell;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use gio::prelude::*;
use glib::variant::ToVariant;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, ColumnView, ColumnViewColumn,
    CssProvider, HeaderBar, Image, Label, ListBox, ListBoxRow, MenuButton, Orientation, Paned,
    PopoverMenu, ScrolledWindow, SelectionMode, Separator, SingleSelection, ToggleButton,
};
use pango::EllipsizeMode;

const APP_ID: &str = "io.github.ArmanIOL.Dolphy";
const APP_NAME: &str = "Dolphy";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Dark Breeze-like palette, close to the Dolphin reference screenshots.
const APP_CSS: &str = r#"
window {
    background-color: #1e1f22;
    color: #d7d8da;
}

headerbar {
    background-color: #2a2b2f;
    min-height: 36px;
}

headerbar .title {
    font-weight: 600;
}

.toolbar {
    background-color: #25262a;
    padding: 4px 6px;
}

.toolbar button {
    min-width: 26px;
    min-height: 26px;
    padding: 2px 6px;
}

.breadcrumb button {
    padding: 2px 6px;
    border-radius: 4px;
}

.breadcrumb button:hover {
    background-color: rgba(255, 255, 255, 0.08);
}

.crumb-separator {
    color: #7f8287;
    margin: 0 1px;
}

.sidebar {
    background-color: #25262a;
}

.sidebar row {
    min-height: 24px;
    padding: 2px 8px;
    margin: 1px 6px;
    border-radius: 4px;
}

.sidebar .group-label {
    color: #8d9095;
    font-size: 0.82em;
    font-weight: 700;
    margin: 10px 10px 3px 10px;
}

columnview {
    background-color: #1e1f22;
}

columnview > header {
    background-color: #2a2b2f;
}

columnview > header button {
    border-radius: 0;
    background-color: transparent;
}

columnview row {
    min-height: 24px;
}

columnview cell {
    padding: 0 4px;
}

.statusbar {
    background-color: #25262a;
    padding: 4px 10px;
}

.dim-label {
    color: #9a9da2;
}
"#;

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_interface_color_scheme(gtk::InterfaceColorScheme::Dark);
    }

    static CSS_INIT: std::sync::Once = std::sync::Once::new();
    CSS_INIT.call_once(install_css);

    Browser::new(app);
}

fn install_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = CssProvider::new();
    provider.load_from_string(APP_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// ---------------------------------------------------------------------------
// Navigation state
// ---------------------------------------------------------------------------

struct Navigator {
    history: Vec<PathBuf>,
    position: usize,
    show_hidden: bool,
}

impl Navigator {
    fn new(start: PathBuf) -> Self {
        Self {
            history: vec![start],
            position: 0,
            show_hidden: false,
        }
    }

    fn current(&self) -> &Path {
        &self.history[self.position]
    }

    fn navigate_to(&mut self, path: PathBuf) {
        if self.current() == path {
            return;
        }
        self.history.truncate(self.position + 1);
        self.history.push(path);
        self.position = self.history.len() - 1;
    }

    fn go_back(&mut self) -> bool {
        if self.position == 0 {
            return false;
        }
        self.position -= 1;
        true
    }

    fn go_forward(&mut self) -> bool {
        if self.position + 1 >= self.history.len() {
            return false;
        }
        self.position += 1;
        true
    }

    fn go_up(&mut self) -> bool {
        let Some(parent) = self.current().parent().map(Path::to_path_buf) else {
            return false;
        };
        self.navigate_to(parent);
        true
    }

    fn can_go_back(&self) -> bool {
        self.position > 0
    }

    fn can_go_forward(&self) -> bool {
        self.position + 1 < self.history.len()
    }

    fn can_go_up(&self) -> bool {
        self.current().parent().is_some()
    }
}

// ---------------------------------------------------------------------------
// File data
// ---------------------------------------------------------------------------

struct FileData {
    path: PathBuf,
    name: String,
    kind_text: String,
    size_text: String,
    modified_text: String,
    icon: gio::Icon,
}

fn item_data(object: &glib::Object) -> Option<Rc<FileData>> {
    let boxed = object.downcast_ref::<glib::BoxedAnyObject>()?;
    let borrowed = boxed.borrow::<Rc<FileData>>();
    Some((*borrowed).clone())
}

fn describe(data: &FileData) -> String {
    format!("{} ({}, {})", data.name, data.kind_text, data.size_text)
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum SidebarTarget {
    Path(PathBuf),
    Stub(&'static str),
}

struct Sidebar {
    widget: ScrolledWindow,
    list: ListBox,
    targets: Vec<Option<SidebarTarget>>,
}

fn build_sidebar() -> Sidebar {
    let list = ListBox::builder()
        .selection_mode(SelectionMode::Single)
        .activate_on_single_click(true)
        .build();
    list.add_css_class("sidebar");

    let mut targets = Vec::new();
    let home = glib::home_dir();

    add_group(&list, &mut targets, "Точки входа");

    add_place(
        &list,
        &mut targets,
        "go-home-symbolic",
        "Домашняя папка",
        SidebarTarget::Path(home.clone()),
    );
    add_place(
        &list,
        &mut targets,
        "user-desktop-symbolic",
        "Рабочий стол",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Desktop, "Desktop")),
    );
    add_place(
        &list,
        &mut targets,
        "folder-documents-symbolic",
        "Документы",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Documents, "Documents")),
    );
    add_place(
        &list,
        &mut targets,
        "folder-download-symbolic",
        "Загрузки",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Downloads, "Downloads")),
    );
    add_place(
        &list,
        &mut targets,
        "folder-music-symbolic",
        "Музыка",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Music, "Music")),
    );
    add_place(
        &list,
        &mut targets,
        "folder-pictures-symbolic",
        "Изображения",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Pictures, "Pictures")),
    );
    add_place(
        &list,
        &mut targets,
        "folder-videos-symbolic",
        "Видеофайлы",
        SidebarTarget::Path(special_dir(glib::UserDirectory::Videos, "Videos")),
    );

    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"));
    let trash = data_home.join("Trash/files");
    let trash_target = if trash.is_dir() {
        SidebarTarget::Path(trash)
    } else {
        SidebarTarget::Stub("Корзина пуста или ещё не создана")
    };
    add_place(
        &list,
        &mut targets,
        "user-trash-symbolic",
        "Корзина",
        trash_target,
    );

    let steam = [home.join(".steam/steam"), home.join(".local/share/Steam")]
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| home.clone());
    add_place(
        &list,
        &mut targets,
        "applications-games-symbolic",
        "Steam",
        SidebarTarget::Path(steam),
    );

    add_group(&list, &mut targets, "В сети");

    let network = std::env::var_os("XDG_RUNTIME_DIR")
        .map(|dir| PathBuf::from(dir).join("gvfs"))
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| home.clone());
    add_place(
        &list,
        &mut targets,
        "network-workgroup-symbolic",
        "Сеть",
        SidebarTarget::Path(network),
    );

    add_group(&list, &mut targets, "Недавно изменённые");

    add_place(
        &list,
        &mut targets,
        "document-open-recent-symbolic",
        "Последние файлы",
        SidebarTarget::Stub("Раздел «Последние файлы» появится в следующей версии"),
    );
    add_place(
        &list,
        &mut targets,
        "document-open-recent-symbolic",
        "Последние расположения",
        SidebarTarget::Stub("Раздел «Последние расположения» появится в следующей версии"),
    );

    add_group(&list, &mut targets, "Устройства");

    add_place(
        &list,
        &mut targets,
        "drive-harddisk-symbolic",
        "Basic data partition",
        SidebarTarget::Path(first_data_mount()),
    );
    add_place(
        &list,
        &mut targets,
        "drive-harddisk-symbolic",
        "endeavouros",
        SidebarTarget::Path(PathBuf::from("/")),
    );

    let widget = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&list)
        .build();

    Sidebar {
        widget,
        list,
        targets,
    }
}

fn add_group(list: &ListBox, targets: &mut Vec<Option<SidebarTarget>>, title: &str) {
    let label = Label::new(Some(title));
    label.set_xalign(0.0);
    label.add_css_class("group-label");

    let row = ListBoxRow::new();
    row.set_child(Some(&label));
    row.set_selectable(false);
    row.set_activatable(false);
    row.set_focusable(false);

    list.append(&row);
    targets.push(None);
}

fn add_place(
    list: &ListBox,
    targets: &mut Vec<Option<SidebarTarget>>,
    icon_name: &str,
    title: &str,
    target: SidebarTarget,
) {
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_top(2);
    hbox.set_margin_bottom(2);

    let icon = Image::from_icon_name(icon_name);
    icon.set_pixel_size(16);

    let label = Label::new(Some(title));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(EllipsizeMode::End);

    hbox.append(&icon);
    hbox.append(&label);

    let row = ListBoxRow::new();
    row.set_child(Some(&hbox));

    list.append(&row);
    targets.push(Some(target));
}

fn special_dir(directory: glib::UserDirectory, fallback: &str) -> PathBuf {
    glib::user_special_dir(directory).unwrap_or_else(|| glib::home_dir().join(fallback))
}

fn first_data_mount() -> PathBuf {
    for mount in gio::VolumeMonitor::get().mounts() {
        if let Some(path) = mount.root().path() {
            if path != Path::new("/") && path.is_dir() {
                return path;
            }
        }
    }
    PathBuf::from("/")
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

struct Toolbar {
    widget: GtkBox,
    back: Button,
    forward: Button,
    up: Button,
    crumbs: GtkBox,
    split: ToggleButton,
}

fn build_toolbar() -> Toolbar {
    let back = Button::from_icon_name("go-previous-symbolic");
    back.set_tooltip_text(Some("Назад (Alt+Left)"));
    back.set_action_name(Some("win.back"));
    back.add_css_class("flat");

    let forward = Button::from_icon_name("go-next-symbolic");
    forward.set_tooltip_text(Some("Вперёд (Alt+Right)"));
    forward.set_action_name(Some("win.forward"));
    forward.add_css_class("flat");

    let up = Button::from_icon_name("go-up-symbolic");
    up.set_tooltip_text(Some("На уровень выше (Alt+Up)"));
    up.set_action_name(Some("win.up"));
    up.add_css_class("flat");

    let crumbs = GtkBox::new(Orientation::Horizontal, 0);
    crumbs.add_css_class("breadcrumb");

    let crumb_scroller = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .hexpand(true)
        .child(&crumbs)
        .build();

    let split = ToggleButton::new();
    split.set_tooltip_text(Some("Две панели (скоро)"));
    split.add_css_class("flat");
    let split_content = GtkBox::new(Orientation::Horizontal, 6);
    let split_icon = Image::from_icon_name("view-dual-symbolic");
    split_icon.set_pixel_size(16);
    split_content.append(&split_icon);
    split_content.append(&Label::new(Some("Две панели")));
    split.set_child(Some(&split_content));

    let search = Button::from_icon_name("system-search-symbolic");
    search.set_tooltip_text(Some("Поиск (Ctrl+F)"));
    search.set_action_name(Some("win.search"));
    search.add_css_class("flat");

    let menu = gio::Menu::new();
    menu.append(Some("Новое окно"), Some("win.new-window"));
    menu.append(Some("Обновить"), Some("win.refresh"));
    menu.append(Some("Показывать скрытые файлы"), Some("win.show-hidden"));
    menu.append(Some("Поиск"), Some("win.search"));
    menu.append(Some("О программе"), Some("win.about"));

    let menu_button = MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_menu_model(Some(&menu));
    menu_button.set_tooltip_text(Some("Главное меню"));
    menu_button.add_css_class("flat");

    let widget = GtkBox::new(Orientation::Horizontal, 4);
    widget.add_css_class("toolbar");
    widget.append(&back);
    widget.append(&forward);
    widget.append(&up);
    widget.append(&crumb_scroller);
    widget.append(&split);
    widget.append(&search);
    widget.append(&menu_button);

    Toolbar {
        widget,
        back,
        forward,
        up,
        crumbs,
        split,
    }
}

// ---------------------------------------------------------------------------
// Main view
// ---------------------------------------------------------------------------

fn build_file_view(selection: &SingleSelection) -> (ColumnView, ScrolledWindow) {
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

// ---------------------------------------------------------------------------
// Browser window
// ---------------------------------------------------------------------------

struct Browser {
    window: ApplicationWindow,
    title_label: Label,
    toolbar: Toolbar,
    sidebar: Sidebar,
    column_view: ColumnView,
    store: gio::ListStore,
    selection: SingleSelection,
    status_left: Label,
    status_right: Label,
    state: RefCell<Navigator>,
}

impl Browser {
    fn new(app: &Application) -> Rc<Self> {
        let start = std::env::current_dir()
            .ok()
            .filter(|path| path.is_dir())
            .unwrap_or_else(glib::home_dir);

        let window = ApplicationWindow::builder()
            .application(app)
            .title(format!("{APP_NAME} — файловый менеджер"))
            .default_width(1100)
            .default_height(750)
            .build();

        let header = HeaderBar::new();
        let title_label = Label::new(Some(APP_NAME));
        title_label.add_css_class("title");
        header.set_title_widget(Some(&title_label));
        window.set_titlebar(Some(&header));

        let toolbar = build_toolbar();
        let sidebar = build_sidebar();

        let store = gio::ListStore::new::<glib::BoxedAnyObject>();
        let selection = SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);

        let (column_view, view_scroller) = build_file_view(&selection);

        let main_area = GtkBox::new(Orientation::Vertical, 0);
        main_area.append(&view_scroller);

        let paned = Paned::new(Orientation::Horizontal);
        paned.set_start_child(Some(&sidebar.widget));
        paned.set_end_child(Some(&main_area));
        paned.set_position(235);
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(false);
        paned.set_vexpand(true);
        sidebar.widget.set_size_request(200, -1);

        let status_left = Label::new(Some("Ничего не выбрано"));
        status_left.set_xalign(0.0);
        status_left.set_hexpand(true);
        status_left.set_ellipsize(EllipsizeMode::End);
        status_left.add_css_class("dim-label");

        let status_right = Label::new(None);
        status_right.set_xalign(1.0);
        status_right.add_css_class("dim-label");

        let status_bar = GtkBox::new(Orientation::Horizontal, 12);
        status_bar.add_css_class("statusbar");
        status_bar.append(&status_left);
        status_bar.append(&status_right);

        let content = GtkBox::new(Orientation::Vertical, 0);
        content.append(&toolbar.widget);
        content.append(&Separator::new(Orientation::Horizontal));
        content.append(&paned);
        content.append(&Separator::new(Orientation::Horizontal));
        content.append(&status_bar);

        window.set_child(Some(&content));

        let browser = Rc::new(Self {
            window,
            title_label,
            toolbar,
            sidebar,
            column_view,
            store,
            selection,
            status_left,
            status_right,
            state: RefCell::new(Navigator::new(start)),
        });

        browser.connect_signals();
        browser.install_actions(app);
        browser.refresh();
        browser.window.present();

        browser
    }

    fn connect_signals(self: &Rc<Self>) {
        let browser = Rc::clone(self);
        self.sidebar.list.connect_row_activated(move |_, row| {
            let target = browser
                .sidebar
                .targets
                .get(row.index() as usize)
                .cloned()
                .flatten();
            match target {
                Some(SidebarTarget::Path(path)) => browser.navigate(path),
                Some(SidebarTarget::Stub(message)) => browser.status_left.set_text(message),
                None => {}
            }
        });

        let status_left = self.status_left.clone();
        self.selection
            .connect_selection_changed(move |selection, _, _| {
                let text = selection
                    .selected_item()
                    .and_then(|item| item_data(&item))
                    .map(|data| describe(&data))
                    .unwrap_or_else(|| String::from("Ничего не выбрано"));
                status_left.set_text(&text);
            });

        let browser = Rc::clone(self);
        self.column_view.connect_activate(move |_, position| {
            let data = browser
                .store
                .item(position)
                .and_then(|item| item_data(&item));
            if let Some(data) = data {
                browser.open(&data.path);
            }
        });

        let browser = Rc::clone(self);
        self.toolbar.split.connect_toggled(move |button| {
            let text = if button.is_active() {
                "Две панели: режим появится в следующей версии"
            } else {
                "Однопанельный режим"
            };
            browser.status_left.set_text(text);
        });
    }

    fn install_actions(self: &Rc<Self>, app: &Application) {
        self.add_action("back", |browser| {
            let moved = browser.state.borrow_mut().go_back();
            if moved {
                browser.refresh();
            }
        });

        self.add_action("forward", |browser| {
            let moved = browser.state.borrow_mut().go_forward();
            if moved {
                browser.refresh();
            }
        });

        self.add_action("up", |browser| {
            let moved = browser.state.borrow_mut().go_up();
            if moved {
                browser.refresh();
            }
        });

        self.add_action("refresh", |browser| browser.refresh());
        self.add_action("search", |browser| {
            browser
                .status_left
                .set_text("Поиск будет добавлен в следующей версии")
        });
        self.add_action("ctx-open-tab", |browser| {
            browser.context_stub("Открыть в новой вкладке")
        });
        self.add_action("ctx-open-window", |browser| {
            browser.context_stub("Открыть в новом окне")
        });
        self.add_action("ctx-cut", |browser| browser.context_stub("Вырезать"));
        self.add_action("ctx-copy", |browser| browser.context_stub("Копировать"));
        self.add_action("ctx-trash", |browser| {
            browser.context_stub("Удалить в корзину")
        });

        self.add_action("about", |browser| browser.show_about_dialog());

        let new_window = gio::SimpleAction::new("new-window", None);
        let app_clone = app.clone();
        new_window.connect_activate(move |_, _| {
            Browser::new(&app_clone);
        });
        self.window.add_action(&new_window);

        let show_hidden = gio::SimpleAction::new_stateful("show-hidden", None, &false.to_variant());
        let browser = Rc::clone(self);
        show_hidden.connect_activate(move |action, _| {
            let current = action
                .state()
                .and_then(|state| state.get::<bool>())
                .unwrap_or(false);
            action.set_state(&(!current).to_variant());
            browser.state.borrow_mut().show_hidden = !current;
            browser.refresh();
        });
        self.window.add_action(&show_hidden);

        app.set_accels_for_action("win.back", &["<alt>Left"]);
        app.set_accels_for_action("win.forward", &["<alt>Right"]);
        app.set_accels_for_action("win.up", &["<alt>Up"]);
        app.set_accels_for_action("win.refresh", &["F5"]);
        app.set_accels_for_action("win.show-hidden", &["<control>h"]);
        app.set_accels_for_action("win.search", &["<control>f"]);
        app.set_accels_for_action("win.new-window", &["<control>n"]);
    }

    fn add_action<F>(self: &Rc<Self>, name: &str, callback: F)
    where
        F: Fn(&Rc<Self>) + 'static,
    {
        let action = gio::SimpleAction::new(name, None);
        let browser = Rc::clone(self);
        action.connect_activate(move |_, _| callback(&browser));
        self.window.add_action(&action);
    }

    fn context_stub(&self, action: &str) {
        let text = self
            .selected_data()
            .map(|data| format!("{action}: «{}» — пока не реализовано", data.name))
            .unwrap_or_else(|| format!("{action}: нет выбранного элемента"));
        self.status_left.set_text(&text);
    }

    fn selected_data(&self) -> Option<Rc<FileData>> {
        self.selection
            .selected_item()
            .and_then(|item| item_data(&item))
    }

    fn show_about_dialog(&self) {
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

    fn navigate(self: &Rc<Self>, path: PathBuf) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        self.state.borrow_mut().navigate_to(path);
        self.refresh();
    }

    fn open(self: &Rc<Self>, path: &Path) {
        if path.is_dir() {
            self.navigate(path.to_path_buf());
            return;
        }

        let file = gio::File::for_path(path);
        let launcher = gtk::FileLauncher::new(Some(&file));
        let status_left = self.status_left.clone();
        launcher.launch(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(error) = result {
                    status_left.set_text(&format!("Не удалось открыть файл: {error}"));
                }
            },
        );
    }

    fn refresh(self: &Rc<Self>) {
        let (path, show_hidden, can_back, can_forward, can_up) = {
            let state = self.state.borrow();
            (
                state.current().to_path_buf(),
                state.show_hidden,
                state.can_go_back(),
                state.can_go_forward(),
                state.can_go_up(),
            )
        };

        self.toolbar.back.set_sensitive(can_back);
        self.toolbar.forward.set_sensitive(can_forward);
        self.toolbar.up.set_sensitive(can_up);

        self.update_breadcrumbs(&path);
        self.update_title(&path);

        self.store.remove_all();
        self.status_left.set_text("Ничего не выбрано");

        let entries = match read_directory(&path, show_hidden) {
            Ok(entries) => entries,
            Err(error) => {
                self.status_left
                    .set_text(&format!("Не удалось прочитать {}: {error}", path.display()));
                self.status_right.set_text("0 объектов");
                return;
            }
        };

        let total_size: u64 = entries
            .iter()
            .filter(|entry| !entry.is_dir)
            .map(|entry| entry.size)
            .sum();

        for entry in &entries {
            let boxed = glib::BoxedAnyObject::new(Rc::clone(&entry.data));
            self.store.append(&boxed);
        }

        let count_text = plural_ru(entries.len(), "объект", "объекта", "объектов");
        let right_text = if total_size > 0 {
            format!("{count_text}, {}", human_size(total_size))
        } else {
            count_text
        };
        self.status_right.set_text(&right_text);
    }

    fn update_title(&self, path: &Path) {
        let folder = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));
        let title = format!("{folder} — {APP_NAME}");
        self.window.set_title(Some(&title));
        self.title_label.set_text(&title);
    }

    fn update_breadcrumbs(self: &Rc<Self>, path: &Path) {
        while let Some(child) = self.toolbar.crumbs.first_child() {
            self.toolbar.crumbs.remove(&child);
        }

        for (index, (label, target)) in breadcrumb_targets(path).into_iter().enumerate() {
            if index > 0 {
                let separator = Label::new(Some("▸"));
                separator.add_css_class("crumb-separator");
                self.toolbar.crumbs.append(&separator);
            }

            let button = Button::with_label(&label);
            button.add_css_class("flat");
            button.add_css_class("crumb");
            button.set_tooltip_text(Some(&target.display().to_string()));

            let browser = Rc::clone(self);
            button.connect_clicked(move |_| browser.navigate(target.clone()));
            self.toolbar.crumbs.append(&button);
        }
    }
}

// ---------------------------------------------------------------------------
// Directory reading and formatting
// ---------------------------------------------------------------------------

struct DirEntryData {
    is_dir: bool,
    size: u64,
    data: Rc<FileData>,
}

fn read_directory(path: &Path, show_hidden: bool) -> std::io::Result<Vec<DirEntryData>> {
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

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} Б")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn plural_ru(count: usize, one: &str, few: &str, many: &str) -> String {
    let mod100 = count % 100;
    let mod10 = count % 10;
    let word = if (11..=14).contains(&mod100) {
        many
    } else if mod10 == 1 {
        one
    } else if (2..=4).contains(&mod10) {
        few
    } else {
        many
    };
    format!("{count} {word}")
}

fn breadcrumb_targets(path: &Path) -> Vec<(String, PathBuf)> {
    let mut targets = vec![(root_label(), PathBuf::from("/"))];
    let mut accumulated = PathBuf::from("/");

    for component in path.components() {
        if let Component::Normal(name) = component {
            accumulated.push(name);
            targets.push((name.to_string_lossy().into_owned(), accumulated.clone()));
        }
    }

    targets
}

fn root_label() -> String {
    for mount in gio::VolumeMonitor::get().mounts() {
        if mount.root().path().as_deref() == Some(Path::new("/")) {
            let name = mount.name();
            if !name.is_empty() {
                return name.to_string();
            }
        }
    }
    String::from("/")
}
