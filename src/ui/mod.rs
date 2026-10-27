pub(crate) mod dialogs;
pub(crate) mod file_view;
pub(crate) mod menu;
pub(crate) mod pathbar;
pub(crate) mod sidebar;
pub(crate) mod toolbar;
pub(crate) mod window;

#[derive(Clone, Copy)]
pub(crate) enum RecentKind {
    Files,
    Locations,
}
