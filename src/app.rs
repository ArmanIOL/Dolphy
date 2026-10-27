use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::Application;

use crate::icons::preferred_icon_theme;
use crate::settings::Settings;
use crate::theme::install_css;
use crate::ui::window::Browser;

pub(crate) const APP_ID: &str = "io.github.ArmanIOL.Dolphy";
pub(crate) const APP_NAME: &str = "Dolphy";
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const DEFAULT_SIDEBAR_WIDTH: i32 = 235;
pub(crate) const MIN_SIDEBAR_WIDTH: i32 = 90;
pub(crate) const DEFAULT_WINDOW_WIDTH: i32 = 1100;
pub(crate) const DEFAULT_WINDOW_HEIGHT: i32 = 750;
pub(crate) const DEFAULT_ICON_SIZE: i32 = 22;
pub(crate) const RECENT_LIMIT: usize = 12;

pub(crate) const DEFAULT_PLACE_IDS: [&str; 8] = [
    "home",
    "desktop",
    "documents",
    "downloads",
    "music",
    "pictures",
    "videos",
    "trash",
];

pub(crate) fn run() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_interface_color_scheme(gtk::InterfaceColorScheme::Dark);
        if let Some(theme) = preferred_icon_theme() {
            settings.set_gtk_icon_theme_name(Some(theme));
        }
    }

    static CSS_INIT: std::sync::Once = std::sync::Once::new();
    CSS_INIT.call_once(install_css);

    let settings = Rc::new(RefCell::new(Settings::load()));
    Browser::new(app, settings);
}
