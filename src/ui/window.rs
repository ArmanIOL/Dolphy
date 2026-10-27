use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gio::prelude::*;
use glib::variant::ToVariant;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, ColumnView, HeaderBar, Label, Orientation,
    Paned, Popover, Separator, SingleSelection,
};
use pango::EllipsizeMode;

use crate::app::{APP_NAME, MIN_SIDEBAR_WIDTH, RECENT_LIMIT};
use crate::filesystem::{describe, item_data, read_directory, DeviceSpec, FileData};
use crate::navigation::{remember_path, Navigator};
use crate::settings::Settings;
use crate::ui::file_view::build_file_view;
use crate::ui::sidebar::{
    build_sidebar, sidebar_resize_is_user_drag, sidebar_width_from_paned_position, Sidebar,
};
use crate::ui::toolbar::{build_toolbar, Toolbar};
use crate::util::{human_size, plural_ru};

pub(crate) struct Browser {
    pub(crate) window: ApplicationWindow,
    pub(crate) title_label: Label,
    pub(crate) toolbar: Toolbar,
    pub(crate) sidebar: Sidebar,
    pub(crate) paned: Paned,
    pub(crate) column_view: ColumnView,
    pub(crate) store: gio::ListStore,
    pub(crate) selection: SingleSelection,
    pub(crate) status_left: Label,
    pub(crate) status_right: Label,
    pub(crate) state: RefCell<Navigator>,
    pub(crate) settings: Rc<RefCell<Settings>>,
    pub(crate) devices: RefCell<Vec<DeviceSpec>>,
    pub(crate) popup: RefCell<Option<Popover>>,
    pub(crate) save_generation: Rc<Cell<u64>>,
    pub(crate) show_all_places: Cell<bool>,
    pub(crate) last_sidebar_mouse_press: Cell<Option<Instant>>,
}

impl Browser {
    pub(crate) fn new(app: &Application, settings: Rc<RefCell<Settings>>) -> Rc<Self> {
        let (start, window_width, window_height, sidebar_visible, sidebar_width, show_hidden) = {
            let settings = settings.borrow();
            let start = settings
                .last_location
                .clone()
                .filter(|path| path.is_dir())
                .unwrap_or_else(glib::home_dir);
            (
                start,
                settings.window_width,
                settings.window_height,
                settings.sidebar_visible,
                settings.sidebar_width,
                settings.show_hidden,
            )
        };

        let window = ApplicationWindow::builder()
            .application(app)
            .title(format!("{APP_NAME} — файловый менеджер"))
            .default_width(window_width)
            .default_height(window_height)
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
        paned.set_position(sidebar_width);
        paned.set_resize_start_child(false);
        paned.set_shrink_start_child(true);
        paned.set_vexpand(true);
        sidebar.widget.set_size_request(MIN_SIDEBAR_WIDTH, -1);
        sidebar.widget.set_visible(sidebar_visible);

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
            paned,
            column_view,
            store,
            selection,
            status_left,
            status_right,
            state: RefCell::new(Navigator::new(start, show_hidden)),
            settings,
            devices: RefCell::new(Vec::new()),
            popup: RefCell::new(None),
            save_generation: Rc::new(Cell::new(0)),
            show_all_places: Cell::new(false),
            last_sidebar_mouse_press: Cell::new(None),
        });

        browser.connect_signals();
        browser.install_actions(app);
        browser.install_menu();
        browser.rebuild_sidebar();
        browser.refresh();
        browser.connect_width_persistence();
        browser.connect_close_persistence();
        browser.window.present();

        browser
    }

    pub(crate) fn connect_signals(self: &Rc<Self>) {
        self.connect_sidebar_signals();
        self.connect_file_view_signals();
        self.connect_toolbar_signals();
        self.connect_pathbar_signals();
    }

    pub(crate) fn connect_file_view_signals(self: &Rc<Self>) {
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

        let weak = Rc::downgrade(self);
        self.column_view.connect_activate(move |_, position| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let data = browser
                .store
                .item(position)
                .and_then(|item| item_data(&item));
            if let Some(data) = data {
                browser.open(&data.path);
            }
        });
    }

    pub(crate) fn connect_toolbar_signals(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.toolbar.split.connect_toggled(move |button| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            let text = if button.is_active() {
                "Две панели: режим появится в следующей версии"
            } else {
                "Однопанельный режим"
            };
            browser.status_left.set_text(text);
        });
    }

    pub(crate) fn install_actions(self: &Rc<Self>, app: &Application) {
        self.add_action("back", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_back() {
                browser.refresh();
            }
        });
        self.add_action("forward", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_forward() {
                browser.refresh();
            }
        });
        self.add_action("up", |browser| {
            browser.toolbar.pathbar.end_edit();
            if browser.state.borrow_mut().go_up() {
                browser.refresh();
            }
        });
        self.add_action("refresh", |browser| {
            browser.rebuild_sidebar();
            browser.refresh();
        });
        self.add_action("search", |browser| {
            browser
                .status_left
                .set_text("Поиск будет добавлен в следующей версии")
        });
        self.add_action("edit-location", |browser| browser.begin_path_edit());
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

        let show_hidden = gio::SimpleAction::new_stateful(
            "show-hidden",
            None,
            &self.settings.borrow().show_hidden.to_variant(),
        );
        let browser = Rc::clone(self);
        show_hidden.connect_activate(move |action, _| {
            let current = action
                .state()
                .and_then(|state| state.get::<bool>())
                .unwrap_or(false);
            let next = !current;
            action.set_state(&next.to_variant());
            browser.set_show_hidden(next);
        });
        self.window.add_action(&show_hidden);

        let toggle_sidebar = gio::SimpleAction::new_stateful(
            "toggle-sidebar",
            None,
            &self.settings.borrow().sidebar_visible.to_variant(),
        );
        let browser = Rc::clone(self);
        toggle_sidebar.connect_activate(move |action, _| {
            let current = action
                .state()
                .and_then(|state| state.get::<bool>())
                .unwrap_or(true);
            let next = !current;
            action.set_state(&next.to_variant());
            browser.set_sidebar_visible(next);
        });
        self.window.add_action(&toggle_sidebar);

        let new_window = gio::SimpleAction::new("new-window", None);
        let app_clone = app.clone();
        let settings = Rc::clone(&self.settings);
        new_window.connect_activate(move |_, _| {
            Browser::new(&app_clone, Rc::clone(&settings));
        });
        self.window.add_action(&new_window);

        app.set_accels_for_action("win.back", &["<alt>Left"]);
        app.set_accels_for_action("win.forward", &["<alt>Right"]);
        app.set_accels_for_action("win.up", &["<alt>Up"]);
        app.set_accels_for_action("win.refresh", &["F5"]);
        app.set_accels_for_action("win.show-hidden", &["<control>h"]);
        app.set_accels_for_action("win.edit-location", &["<control>l"]);
        app.set_accels_for_action("win.toggle-sidebar", &["F9"]);
        app.set_accels_for_action("win.search", &["<control>f"]);
        app.set_accels_for_action("win.new-window", &["<control>n"]);
    }

    pub(crate) fn add_action<F>(self: &Rc<Self>, name: &str, callback: F)
    where
        F: Fn(&Rc<Self>) + 'static,
    {
        let action = gio::SimpleAction::new(name, None);
        let browser = Rc::clone(self);
        action.connect_activate(move |_, _| callback(&browser));
        self.window.add_action(&action);
    }

    pub(crate) fn refresh(self: &Rc<Self>) {
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

        self.update_pathbar(&path);
        self.update_title(&path);

        self.store.remove_all();
        self.status_left.set_text("Ничего не выбрано");

        let entries = match read_directory(&path, show_hidden) {
            Ok(entries) => entries,
            Err(error) => {
                self.status_left
                    .set_text(&format!("Не удалось прочитать {}: {error}", path.display()));
                self.status_right.set_text("0 объектов");
                self.store_current_location(&path);
                self.sync_sidebar_selection();
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

        self.store_current_location(&path);
        self.sync_sidebar_selection();
    }

    pub(crate) fn store_current_location(&self, path: &Path) {
        {
            let mut settings = self.settings.borrow_mut();
            settings.last_location = Some(path.to_path_buf());
            remember_path(&mut settings.recent_locations, path, RECENT_LIMIT);
        }
        self.save_settings_now();
    }

    pub(crate) fn update_title(&self, path: &Path) {
        let folder = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| String::from("/"));
        let title = format!("{folder} — {APP_NAME}");
        self.window.set_title(Some(&title));
        self.title_label.set_text(&title);
    }

    pub(crate) fn select_path(&self, path: &Path) {
        for position in 0..self.store.n_items() {
            let Some(item) = self.store.item(position) else {
                continue;
            };
            let Some(data) = item_data(&item) else {
                continue;
            };
            if data.path == path {
                self.selection.set_selected(position);
                self.column_view
                    .scroll_to(position, None, gtk::ListScrollFlags::NONE, None);
                break;
            }
        }
    }

    pub(crate) fn navigate(self: &Rc<Self>, path: PathBuf) {
        if !path.is_dir() {
            self.status_left
                .set_text(&format!("Не папка: {}", path.display()));
            return;
        }
        self.toolbar.pathbar.end_edit();
        self.state.borrow_mut().navigate_to(path);
        self.refresh();
    }

    pub(crate) fn open(self: &Rc<Self>, path: &Path) {
        if path.is_dir() {
            self.navigate(path.to_path_buf());
            return;
        }

        {
            let mut settings = self.settings.borrow_mut();
            remember_path(&mut settings.recent_files, path, RECENT_LIMIT);
        }
        self.save_settings_now();

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

    pub(crate) fn set_show_hidden(self: &Rc<Self>, show_hidden: bool) {
        if self.state.borrow().show_hidden == show_hidden {
            return;
        }
        self.state.borrow_mut().show_hidden = show_hidden;
        self.settings.borrow_mut().show_hidden = show_hidden;
        self.save_settings_now();
        self.refresh();
    }

    pub(crate) fn set_sidebar_visible(self: &Rc<Self>, visible: bool) {
        if visible {
            let width = self.settings.borrow().sidebar_width;
            self.sidebar.widget.set_visible(true);
            self.paned.set_position(width);
            self.toolbar
                .sidebar_toggle
                .set_icon_name("sidebar-show-symbolic");
        } else {
            // The width is persisted by connect_width_persistence only for
            // real user drags. Never overwrite it with a layout-induced
            // position when the panel is merely hidden.
            self.sidebar.widget.set_visible(false);
        }
        self.settings.borrow_mut().sidebar_visible = visible;
        self.save_settings_now();
    }

    pub(crate) fn save_settings_now(&self) {
        if let Err(error) = self.settings.borrow().save() {
            glib::g_warning!("Dolphy", "не удалось сохранить настройки: {}", error);
        }
    }

    pub(crate) fn schedule_save(self: &Rc<Self>) {
        let generation = self.save_generation.get().wrapping_add(1);
        self.save_generation.set(generation);
        let settings = Rc::clone(&self.settings);
        let generation_cell = Rc::clone(&self.save_generation);
        glib::timeout_add_local_once(Duration::from_millis(250), move || {
            if generation_cell.get() == generation {
                if let Err(error) = settings.borrow().save() {
                    glib::g_warning!("Dolphy", "не удалось сохранить настройки: {}", error);
                }
            }
        });
    }

    pub(crate) fn connect_width_persistence(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.paned.connect_position_notify(move |paned| {
            let Some(browser) = weak.upgrade() else {
                return;
            };
            // Ignore every position change that is not caused by the user
            // dragging the splitter (initial layout, window clamping, hiding
            // the panel, ...). Persisting those would silently replace the
            // width chosen by the user.
            if !sidebar_resize_is_user_drag() || !browser.settings.borrow().sidebar_visible {
                return;
            }
            let Some(width) = sidebar_width_from_paned_position(paned.position()) else {
                return;
            };
            let changed = {
                let mut settings = browser.settings.borrow_mut();
                if settings.sidebar_width != width {
                    settings.sidebar_width = width;
                    true
                } else {
                    false
                }
            };
            if changed {
                browser.schedule_save();
            }
        });
    }

    pub(crate) fn connect_close_persistence(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.window.connect_close_request(move |window| {
            if let Some(browser) = weak.upgrade() {
                if !window.is_maximized() && !window.is_fullscreen() {
                    let width = window.width().max(1);
                    let height = window.height().max(1);
                    let mut settings = browser.settings.borrow_mut();
                    settings.window_width = width;
                    settings.window_height = height;
                }
                browser.save_settings_now();
            }
            glib::Propagation::Proceed
        });
    }

    pub(crate) fn selected_data(&self) -> Option<Rc<FileData>> {
        self.selection
            .selected_item()
            .and_then(|item| item_data(&item))
    }

    pub(crate) fn context_stub(&self, action: &str) {
        let text = self
            .selected_data()
            .map(|data| format!("{action}: «{}» — пока не реализовано", data.name))
            .unwrap_or_else(|| format!("{action}: нет выбранного элемента"));
        self.status_left.set_text(&text);
    }
}
