use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Button, Entry, HeaderBar, Image, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, SelectionMode, ToggleButton,
};
use pango::EllipsizeMode;

const APP_ID: &str = "io.github.ArmanIOL.Dolphy";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    Browser::new(app);
}

/// Navigation history and view settings for one window.
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

struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    size: u64,
}

struct Browser {
    window: ApplicationWindow,
    list: ListBox,
    path_entry: Entry,
    status: Label,
    back_button: Button,
    forward_button: Button,
    up_button: Button,
    home_button: Button,
    hidden_button: ToggleButton,
    /// Paths in the same order as the rows currently in `list`.
    entries: RefCell<Vec<PathBuf>>,
    state: RefCell<Navigator>,
}

impl Browser {
    fn new(app: &Application) -> Rc<Self> {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Dolphy")
            .default_width(1000)
            .default_height(700)
            .build();

        let header = HeaderBar::new();
        let title = Label::new(Some("Dolphy"));
        title.add_css_class("title");
        header.set_title_widget(Some(&title));

        let back_button = icon_button("go-previous-symbolic", "Back (Alt+Left)");
        back_button.set_sensitive(false);
        let forward_button = icon_button("go-next-symbolic", "Forward (Alt+Right)");
        forward_button.set_sensitive(false);
        let up_button = icon_button("go-up-symbolic", "Parent folder (Alt+Up)");
        let home_button = icon_button("go-home-symbolic", "Home folder");

        header.pack_start(&back_button);
        header.pack_start(&forward_button);
        header.pack_start(&up_button);
        header.pack_start(&home_button);

        let hidden_button = ToggleButton::builder()
            .icon_name("view-reveal-symbolic")
            .tooltip_text("Show hidden files (Ctrl+H)")
            .build();
        header.pack_end(&hidden_button);

        window.set_titlebar(Some(&header));

        let path_entry = Entry::builder()
            .hexpand(true)
            .placeholder_text("Enter a path…")
            .build();

        let list = ListBox::builder()
            .selection_mode(SelectionMode::Single)
            .activate_on_single_click(false)
            .build();

        let scroller = ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&list)
            .build();

        let status = Label::builder().xalign(0.0).build();
        status.add_css_class("dim-label");

        let status_bar = gtk::Box::new(Orientation::Horizontal, 0);
        status_bar.set_margin_top(6);
        status_bar.set_margin_bottom(6);
        status_bar.set_margin_start(12);
        status_bar.set_margin_end(12);
        status_bar.append(&status);

        let path_bar = gtk::Box::new(Orientation::Horizontal, 0);
        path_bar.set_margin_top(6);
        path_bar.set_margin_bottom(6);
        path_bar.set_margin_start(12);
        path_bar.set_margin_end(12);
        path_bar.append(&path_entry);

        let content = gtk::Box::new(Orientation::Vertical, 0);
        content.append(&path_bar);
        content.append(&gtk::Separator::new(Orientation::Horizontal));
        content.append(&scroller);
        content.append(&gtk::Separator::new(Orientation::Horizontal));
        content.append(&status_bar);

        window.set_child(Some(&content));

        let browser = Rc::new(Self {
            window,
            list,
            path_entry,
            status,
            back_button,
            forward_button,
            up_button,
            home_button,
            hidden_button,
            entries: RefCell::new(Vec::new()),
            state: RefCell::new(Navigator::new(glib::home_dir())),
        });

        browser.connect_signals();
        browser.install_actions(app);
        browser.refresh();
        browser.window.present();

        browser
    }

    fn connect_signals(self: &Rc<Self>) {
        let browser = Rc::clone(self);
        self.back_button.connect_clicked(move |_| {
            let moved = browser.state.borrow_mut().go_back();
            if moved {
                browser.refresh();
            }
        });

        let browser = Rc::clone(self);
        self.forward_button.connect_clicked(move |_| {
            let moved = browser.state.borrow_mut().go_forward();
            if moved {
                browser.refresh();
            }
        });

        let browser = Rc::clone(self);
        self.up_button.connect_clicked(move |_| {
            let moved = browser.state.borrow_mut().go_up();
            if moved {
                browser.refresh();
            }
        });

        let browser = Rc::clone(self);
        self.home_button.connect_clicked(move |_| browser.go_home());

        let browser = Rc::clone(self);
        self.hidden_button.connect_toggled(move |button| {
            browser.state.borrow_mut().show_hidden = button.is_active();
            browser.refresh();
        });

        let browser = Rc::clone(self);
        self.path_entry
            .connect_activate(move |entry| browser.navigate_from_entry(entry));

        let browser = Rc::clone(self);
        self.list.connect_row_activated(move |_, row| {
            let path = browser.entries.borrow().get(row.index() as usize).cloned();
            if let Some(path) = path {
                browser.open(&path);
            }
        });
    }

    fn install_actions(self: &Rc<Self>, app: &Application) {
        self.add_action("back", |browser| {
            if browser.state.borrow_mut().go_back() {
                browser.refresh();
            }
        });
        self.add_action("forward", |browser| {
            if browser.state.borrow_mut().go_forward() {
                browser.refresh();
            }
        });
        self.add_action("up", |browser| {
            if browser.state.borrow_mut().go_up() {
                browser.refresh();
            }
        });
        self.add_action("home", |browser| browser.go_home());
        self.add_action("refresh", |browser| browser.refresh());
        self.add_action("location", |browser| {
            browser.path_entry.grab_focus();
            browser.path_entry.select_region(0, -1);
        });
        self.add_action("toggle-hidden", |browser| {
            browser
                .hidden_button
                .set_active(!browser.hidden_button.is_active());
        });

        app.set_accels_for_action("win.back", &["<alt>Left"]);
        app.set_accels_for_action("win.forward", &["<alt>Right"]);
        app.set_accels_for_action("win.up", &["<alt>Up"]);
        app.set_accels_for_action("win.home", &["<alt>Home"]);
        app.set_accels_for_action("win.refresh", &["F5"]);
        app.set_accels_for_action("win.location", &["<control>l"]);
        app.set_accels_for_action("win.toggle-hidden", &["<control>h"]);
    }

    fn add_action<F>(self: &Rc<Self>, name: &str, callback: F)
    where
        F: Fn(&Self) + 'static,
    {
        let action = gio::SimpleAction::new(name, None);
        let browser = Rc::clone(self);
        action.connect_activate(move |_, _| callback(&browser));
        self.window.add_action(&action);
    }

    fn go_home(&self) {
        self.state.borrow_mut().navigate_to(glib::home_dir());
        self.refresh();
    }

    fn navigate_from_entry(&self, entry: &Entry) {
        let text = entry.text().to_string();
        let trimmed = text.trim();

        if trimmed.is_empty() {
            self.show_current_path();
            return;
        }

        let current = self.state.borrow().current().to_path_buf();
        let mut path = expand_path(trimmed);
        if !path.is_absolute() {
            path = current.join(path);
        }

        if path.is_dir() {
            self.state.borrow_mut().navigate_to(path);
            self.refresh();
        } else {
            self.status
                .set_text(&format!("Not a folder: {}", path.display()));
            self.show_current_path();
        }
    }

    fn show_current_path(&self) {
        let current = self.state.borrow().current().to_path_buf();
        self.path_entry.set_text(&current.display().to_string());
    }

    fn open(&self, path: &Path) {
        if path.is_dir() {
            self.state.borrow_mut().navigate_to(path.to_path_buf());
            self.refresh();
            return;
        }

        let file = gio::File::for_path(path);
        let launcher = gtk::FileLauncher::new(Some(&file));
        let status = self.status.clone();
        launcher.launch(
            Some(&self.window),
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(error) = result {
                    status.set_text(&format!("Failed to open file: {error}"));
                }
            },
        );
    }

    fn refresh(&self) {
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

        self.back_button.set_sensitive(can_back);
        self.forward_button.set_sensitive(can_forward);
        self.up_button.set_sensitive(can_up);
        self.path_entry.set_text(&path.display().to_string());

        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        self.entries.borrow_mut().clear();

        let dir = match std::fs::read_dir(&path) {
            Ok(dir) => dir,
            Err(error) => {
                self.status
                    .set_text(&format!("Cannot read {}: {error}", path.display()));
                return;
            }
        };

        let mut entries = Vec::new();
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !show_hidden && name.starts_with('.') {
                continue;
            }

            let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            let size = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
            entries.push(FileEntry {
                path: entry.path(),
                name,
                is_dir,
                size,
            });
        }

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        self.entries
            .borrow_mut()
            .extend(entries.iter().map(|entry| entry.path.clone()));

        for entry in &entries {
            self.append_row(entry);
        }

        let count = entries.len();
        let word = if count == 1 { "item" } else { "items" };
        self.status.set_text(&format!("{count} {word}"));
    }

    fn append_row(&self, entry: &FileEntry) {
        let row = ListBoxRow::new();

        let hbox = gtk::Box::new(Orientation::Horizontal, 12);
        hbox.set_margin_top(6);
        hbox.set_margin_bottom(6);
        hbox.set_margin_start(12);
        hbox.set_margin_end(12);

        let icon_name = if entry.is_dir {
            "folder"
        } else {
            "text-x-generic"
        };
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(20);

        let name = Label::builder()
            .label(&entry.name)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(EllipsizeMode::Middle)
            .build();

        hbox.append(&icon);
        hbox.append(&name);

        if !entry.is_dir {
            let size = Label::new(Some(&human_size(entry.size)));
            size.set_xalign(1.0);
            size.add_css_class("dim-label");
            hbox.append(&size);
        }

        row.set_child(Some(&hbox));
        self.list.append(&row);
    }
}

fn icon_button(icon_name: &str, tooltip: &str) -> Button {
    let button = Button::from_icon_name(icon_name);
    button.set_tooltip_text(Some(tooltip));
    button
}

fn expand_path(input: &str) -> PathBuf {
    if input == "~" {
        glib::home_dir()
    } else if let Some(rest) = input.strip_prefix("~/") {
        glib::home_dir().join(rest)
    } else {
        PathBuf::from(input)
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
