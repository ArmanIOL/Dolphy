mod app;
mod filesystem;
mod icons;
mod navigation;
mod places;
mod settings;
mod theme;
mod ui;
mod util;

#[cfg(test)]
mod tests;

fn main() -> glib::ExitCode {
    app::run()
}
