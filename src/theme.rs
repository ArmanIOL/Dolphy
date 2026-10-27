use gtk::CssProvider;

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
    padding: 3px 6px;
}

.toolbar button {
    min-width: 26px;
    min-height: 26px;
    padding: 2px 5px;
}

.toolbar menubutton > button {
    min-width: 26px;
    min-height: 26px;
}

.pathbar {
    background-color: transparent;
    padding: 2px 0;
}

.pathbar button {
    min-height: 26px;
    padding: 2px 5px;
    border-radius: 4px;
}

.pathbar .path-edit-button {
    min-width: 26px;
}

.breadcrumb button {
    padding: 2px 6px;
    border-radius: 4px;
    min-height: 24px;
}

.breadcrumb button:hover {
    background-color: rgba(255, 255, 255, 0.08);
}

.crumb-separator {
    color: #7f8287;
    margin: 0 1px;
}

.path-entry {
    min-height: 26px;
    background-color: transparent;
    border: none;
    box-shadow: none;
}

.path-entry.error {
    box-shadow: inset 0 0 0 1px #da4453;
}

.path-field {
    background-color: #1a1b1e;
    border-radius: 4px;
    padding: 1px 2px;
}

.path-field scrolledwindow {
    background-color: transparent;
}

.pathbar .crumb-filler {
    min-width: 0;
    padding: 0;
    margin: 0;
    background-color: transparent;
    border: none;
    box-shadow: none;
}

.pathbar .crumb-filler:hover {
    background-color: transparent;
}

.sidebar {
    background-color: #25262a;
    padding: 0;
}

.sidebar row {
    min-height: 24px;
    padding: 3px 4px;
    margin: 0;
    border-radius: 0;
}

.sidebar row:hover {
    background-color: rgba(255, 255, 255, 0.05);
}

.sidebar row:selected {
    background-color: #3daee9;
    color: #ffffff;
}

.sidebar row:selected label {
    color: #ffffff;
}

.sidebar row.drop-target {
    background-color: rgba(61, 174, 233, 0.25);
}

.sidebar .group-label {
    color: #8d9095;
    font-size: 0.9em;
    font-weight: 400;
    margin: 8px 2px 2px 2px;
}

.sidebar .place-line {
    min-height: 22px;
}

.sidebar progressbar.capacity-bar {
    min-height: 3px;
    margin: 1px 4px 2px 0;
}

.sidebar progressbar.capacity-bar trough {
    min-height: 3px;
    border-radius: 2px;
    background-color: #3a3b40;
}

.sidebar progressbar.capacity-bar trough > progress {
    min-height: 3px;
    border-radius: 2px;
    background-color: #3daee9;
}

.places-popover,
.recent-popover {
    background-color: #2a2b2f;
}

.places-list row,
.recent-list row {
    min-height: 28px;
    padding: 3px 8px;
    border-radius: 4px;
}

.places-list row:hover,
.recent-list row:hover {
    background-color: rgba(255, 255, 255, 0.06);
}

.popover-section {
    color: #8d9095;
    font-size: 0.82em;
    font-weight: 700;
    margin: 8px 10px 3px 10px;
}

.menu-row {
    min-height: 28px;
    padding: 4px 10px;
    border-radius: 4px;
}

.menu-row:hover {
    background-color: rgba(255, 255, 255, 0.07);
}

.menu-row:disabled {
    opacity: 0.45;
}

.menu-row.danger label {
    color: #ef7d84;
}

.menu-row .menu-check {
    color: #3daee9;
    font-weight: 700;
}

.menu-separator {
    background-color: #3a3b40;
    min-height: 1px;
    margin: 5px 8px;
}

.menu-empty {
    color: #9a9da2;
    padding: 10px 12px;
}

.add-place-error {
    color: #ef7d84;
}

.recent-name {
    font-weight: 600;
}

.recent-path {
    color: #9a9da2;
    font-size: 0.85em;
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

pub(crate) fn install_css() {
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
