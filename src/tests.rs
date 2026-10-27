use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::app::{
    DEFAULT_ICON_SIZE, DEFAULT_PLACE_IDS, DEFAULT_SIDEBAR_WIDTH, DEFAULT_WINDOW_HEIGHT,
    DEFAULT_WINDOW_WIDTH, MIN_SIDEBAR_WIDTH,
};
use crate::filesystem::{
    decode_by_label, is_pseudo_filesystem, is_system_mount, parse_mounts, unescape_mount_field,
    DeviceSpec,
};
use crate::navigation::{
    build_breadcrumbs, current_place_icon, path_anchors, resolve_path_text, BreadcrumbAnchor,
    Navigator,
};
use crate::places::{compose_sidebar_rows, default_places, reorder_place_ids, PlaceTarget};
use crate::settings::{CustomPlace, SectionId, Settings};
use crate::ui::sidebar::{section_header_text, sidebar_width_from_paned_position};
use crate::util::{deduplicate_paths, human_size, plural_ru};

use gio::prelude::*;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "dolphy-test-{}-{unique}-{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create test dir");
        Self { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn sample_settings() -> Settings {
    let mut settings = Settings::defaults();
    settings.sidebar_visible = false;
    settings.sidebar_width = 321;
    settings.sidebar_icon_size = 22;
    settings.hidden_sections = vec![SectionId::Remote];
    settings.custom_places.push(CustomPlace {
        id: String::from("custom:7"),
        name: String::from("Проекты"),
        path: PathBuf::from("/tmp"),
        icon: String::from("folder-projects"),
    });
    settings.next_custom_id = 8;
    settings.places_order.push(String::from("custom:7"));
    settings.edited_places.push(CustomPlace {
        id: String::from("home"),
        name: String::from("Личное"),
        path: PathBuf::from("/tmp"),
        icon: String::from("folder-home"),
    });
    settings.hidden_places = vec![
        String::from("music"),
        String::from("custom:7"),
        String::from("device:/mnt/windows"),
    ];
    settings.show_hidden = true;
    settings.show_full_path = true;
    settings.window_width = 1440;
    settings.window_height = 900;
    settings.last_location = Some(PathBuf::from("/tmp"));
    settings.recent_locations = vec![PathBuf::from("/tmp"), PathBuf::from("/")];
    settings.recent_files = vec![PathBuf::from("/tmp/report.txt")];
    settings.normalize();
    settings
}

#[test]
fn settings_round_trip_preserves_all_state() {
    let dir = TempDir::new("round-trip");
    let path = dir.path.join("dolphy.conf");
    let settings = sample_settings();

    settings.save_to_path(&path).expect("save settings");
    let loaded = Settings::load_from_path(&path);

    assert_eq!(loaded, settings);
    let data = std::fs::read_to_string(&path).expect("read settings");
    assert!(data.contains("version=1"));
    assert!(!data.contains("sidebar_width_placeholder"));
    assert!(data.contains("width=321"));
}

#[test]
fn missing_config_uses_defaults_without_backup() {
    let dir = TempDir::new("missing");
    let path = dir.path.join("dolphy.conf");
    let loaded = Settings::load_from_path(&path);
    assert_eq!(loaded, Settings::defaults());
    assert!(!path.with_file_name("dolphy.conf.bak").exists());
}

#[test]
fn corrupt_config_is_backed_up_and_defaults_are_used() {
    let dir = TempDir::new("corrupt");
    let path = dir.path.join("dolphy.conf");
    std::fs::write(&path, "это не key file\n[[[bad").expect("write corrupt config");

    let loaded = Settings::load_from_path(&path);
    assert_eq!(loaded, Settings::defaults());
    assert!(!path.exists());
    assert!(path.with_file_name("dolphy.conf.bak").is_file());
}

#[test]
fn incompatible_config_version_falls_back_to_defaults() {
    let dir = TempDir::new("version");
    let path = dir.path.join("dolphy.conf");
    std::fs::write(&path, "[General]\nversion=99\n").expect("write versioned config");

    let loaded = Settings::load_from_path(&path);
    assert_eq!(loaded, Settings::defaults());
    assert!(path.with_file_name("dolphy.conf.bak").is_file());
}

#[test]
fn resolve_path_text_handles_all_input_forms() {
    let dir = TempDir::new("resolve");
    let folder = dir.path.join("папка с пробелом");
    let file = dir.path.join("файл.txt");
    std::fs::create_dir_all(&folder).expect("create folder");
    std::fs::write(&file, "data").expect("create file");
    let canonical_dir = dir.path.canonicalize().expect("canonical dir");
    let canonical_folder = folder.canonicalize().expect("canonical folder");
    let canonical_file = file.canonicalize().expect("canonical file");

    assert!(resolve_path_text("", &canonical_dir, &canonical_dir).is_err());
    assert!(resolve_path_text("   ", &canonical_dir, &canonical_dir).is_err());
    assert_eq!(
        resolve_path_text("~", &canonical_dir, &canonical_dir).unwrap(),
        canonical_dir
    );
    assert_eq!(
        resolve_path_text("папка с пробелом", &canonical_dir, &canonical_dir).unwrap(),
        canonical_folder
    );
    assert_eq!(
        resolve_path_text(
            &canonical_file.to_string_lossy(),
            &canonical_dir,
            &canonical_dir
        )
        .unwrap(),
        canonical_file
    );
    let uri = gio::File::for_path(&folder).uri();
    assert_eq!(
        resolve_path_text(&uri, &canonical_dir, &canonical_dir).unwrap(),
        canonical_folder
    );

    let missing = resolve_path_text("нет-такой-папки", &canonical_dir, &canonical_dir).unwrap_err();
    assert!(missing.contains("не найдена"), "{missing}");

    let remote =
        resolve_path_text("smb://server/share", &canonical_dir, &canonical_dir).unwrap_err();
    assert!(remote.contains("локальные"), "{remote}");
}

#[test]
fn breadcrumbs_start_at_deepest_place_and_keep_unicode_names() {
    let home = PathBuf::from("/home/user");
    let documents = home.join("Документы");
    let windows = PathBuf::from("/mnt/windows");
    let anchors = vec![
        BreadcrumbAnchor {
            label: String::from("Домашняя папка"),
            path: home.clone(),
            icon: String::from("user-home"),
        },
        BreadcrumbAnchor {
            label: String::from("Документы"),
            path: documents.clone(),
            icon: String::from("folder-documents"),
        },
        BreadcrumbAnchor {
            label: String::from("Basic data partition"),
            path: windows.clone(),
            icon: String::from("drive-harddisk"),
        },
    ];

    assert_eq!(
        build_breadcrumbs(&documents, &anchors, false),
        vec![(String::from("Документы"), documents.clone())]
    );

    let nested = documents.join("Отчёты").join("2026");
    assert_eq!(
        build_breadcrumbs(&nested, &anchors, false),
        vec![
            (String::from("Документы"), documents.clone()),
            (String::from("Отчёты"), documents.join("Отчёты")),
            (String::from("2026"), nested.clone()),
        ]
    );

    assert_eq!(
        build_breadcrumbs(&home.join("Загрузки"), &anchors, false),
        vec![
            (String::from("Домашняя папка"), home.clone()),
            (String::from("Загрузки"), home.join("Загрузки")),
        ]
    );

    assert_eq!(
        build_breadcrumbs(&windows.join("Users"), &anchors, false),
        vec![
            (String::from("Basic data partition"), windows.clone()),
            (String::from("Users"), windows.join("Users")),
        ]
    );

    assert_eq!(
        build_breadcrumbs(Path::new("/var/log"), &anchors, false),
        vec![
            (String::from("/"), PathBuf::from("/")),
            (String::from("var"), PathBuf::from("/var")),
            (String::from("log"), PathBuf::from("/var/log")),
        ]
    );
}

#[test]
fn current_place_icon_uses_closest_anchor() {
    let anchors = vec![
        BreadcrumbAnchor {
            label: String::from("Домашняя папка"),
            path: PathBuf::from("/home/user"),
            icon: String::from("user-home"),
        },
        BreadcrumbAnchor {
            label: String::from("Документы"),
            path: PathBuf::from("/home/user/Документы"),
            icon: String::from("folder-documents"),
        },
    ];

    assert_eq!(
        current_place_icon(Path::new("/home/user/Документы/2026"), &anchors),
        "folder-documents"
    );
    assert_eq!(
        current_place_icon(Path::new("/home/user/Музыка"), &anchors),
        "user-home"
    );
    assert_eq!(current_place_icon(Path::new("/var"), &anchors), "folder");
}

#[test]
fn mounts_are_parsed_with_octal_escapes() {
    let mounts = "\
/dev/nvme0n1p3 /mnt/windows fuseblk rw,relatime 0 0
/dev/sda1 /media/My\\040Disk ext4 rw 0 0
proc /proc proc rw 0 0
";
    let parsed = parse_mounts(mounts);
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].source, "/dev/nvme0n1p3");
    assert_eq!(parsed[0].target, PathBuf::from("/mnt/windows"));
    assert_eq!(parsed[1].target, PathBuf::from("/media/My Disk"));
    assert_eq!(parsed[2].fstype, "proc");
    assert_eq!(unescape_mount_field("/a\\040b\\134c"), "/a b\\c");
    assert_eq!(decode_by_label("My\\x20Disk"), "My Disk");
    assert!(is_pseudo_filesystem("proc"));
    assert!(!is_pseudo_filesystem("ext4"));
    assert!(is_system_mount(Path::new("/boot/efi")));
}

#[test]
fn recent_paths_are_deduplicated_and_limited() {
    let paths = vec![
        PathBuf::from("/a"),
        PathBuf::from("/b"),
        PathBuf::from("/a"),
        PathBuf::new(),
        PathBuf::from("/c"),
    ];
    assert_eq!(
        deduplicate_paths(&paths, 2),
        vec![PathBuf::from("/a"), PathBuf::from("/b")]
    );
}

#[test]
fn normalization_repairs_corrupt_values() {
    let mut settings = Settings::defaults();
    settings.sidebar_width = 10;
    settings.sidebar_icon_size = 99;
    settings.window_width = 10;
    settings.window_height = 10;
    settings.hidden_sections = vec![SectionId::Devices, SectionId::Devices];
    settings.hidden_places = vec![
        String::from("missing"),
        String::from("home"),
        String::from("home"),
        String::from("network"),
        String::from("recent-files"),
        String::from("device:/mnt/windows"),
        String::from("device:/mnt/windows"),
    ];
    settings.places_order = vec![
        String::from("home"),
        String::from("home"),
        String::from("missing"),
        String::from("network"),
    ];
    settings.custom_places = vec![
        CustomPlace {
            id: String::from("home"),
            name: String::new(),
            path: PathBuf::from("/tmp/Мои документы"),
            icon: String::new(),
        },
        CustomPlace {
            id: String::from("home"),
            name: String::from("  Второе место  "),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder-temp"),
        },
    ];
    settings.next_custom_id = 3;
    settings.recent_locations = vec![
        PathBuf::from("/tmp"),
        PathBuf::from("/tmp"),
        PathBuf::from("/"),
        PathBuf::from("/"),
    ];

    settings.normalize();

    assert_eq!(settings.sidebar_width, MIN_SIDEBAR_WIDTH);
    assert_eq!(settings.sidebar_icon_size, DEFAULT_ICON_SIZE);
    assert_eq!(settings.window_width, DEFAULT_WINDOW_WIDTH);
    assert_eq!(settings.window_height, DEFAULT_WINDOW_HEIGHT);
    assert_eq!(settings.hidden_sections, vec![SectionId::Devices]);
    assert_eq!(
        settings.hidden_places,
        vec![
            String::from("home"),
            String::from("network"),
            String::from("recent-files"),
            String::from("device:/mnt/windows"),
        ]
    );
    assert!(settings.places_order.contains(&String::from("custom:3")));
    assert!(settings.places_order.contains(&String::from("custom:4")));
    assert!(!settings.places_order.contains(&String::from("network")));
    assert_eq!(settings.places_order.len(), DEFAULT_PLACE_IDS.len() + 2);
    assert_eq!(settings.custom_places[0].id, "custom:3");
    assert_eq!(settings.custom_places[0].name, "Мои документы");
    assert_eq!(settings.custom_places[0].icon, "folder");
    assert_eq!(settings.custom_places[1].id, "custom:4");
    assert_eq!(settings.custom_places[1].name, "Второе место");
    assert_eq!(settings.next_custom_id, 5);
    assert_eq!(
        settings.recent_locations,
        vec![PathBuf::from("/tmp"), PathBuf::from("/")]
    );
}

#[test]
fn sidebar_uses_fixed_sections_with_headers_on_first_row() {
    let mut settings = Settings::defaults();
    settings.hidden_places.clear();
    settings.hidden_sections.clear();
    let devices = vec![DeviceSpec {
        mount: PathBuf::from("/mnt/windows"),
        label: String::from("Windows"),
        used_fraction: Some(0.5),
        tooltip: String::from("Windows"),
    }];

    let rows = compose_sidebar_rows(
        &settings,
        &devices,
        false,
        Path::new("/definitely/not/a/place"),
    );
    let sections: Vec<SectionId> =
        rows.iter()
            .map(|row| row.section)
            .fold(Vec::new(), |mut result, section| {
                if result.last() != Some(&section) {
                    result.push(section);
                }
                result
            });
    assert_eq!(sections, SectionId::ALL.to_vec());

    let mut seen = Vec::new();
    for row in &rows {
        if seen.contains(&row.section) {
            assert!(
                row.header.is_none(),
                "duplicate header for {:?}",
                row.section
            );
        } else {
            assert_eq!(row.header, Some(row.section));
            seen.push(row.section);
        }
    }
}

#[test]
fn places_order_can_move_entries_before_and_after_a_target() {
    let mut order = ["home", "desktop", "documents", "downloads"]
        .iter()
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();

    assert!(reorder_place_ids(&mut order, "home", "documents", true));
    assert_eq!(order, ["desktop", "documents", "home", "downloads"]);

    assert!(reorder_place_ids(&mut order, "downloads", "desktop", false));
    assert_eq!(order, ["downloads", "desktop", "documents", "home"]);

    assert!(!reorder_place_ids(&mut order, "home", "home", true));
    assert!(!reorder_place_ids(&mut order, "missing", "home", true));
    assert!(!reorder_place_ids(&mut order, "home", "network", false));
}

#[test]
fn hidden_entries_and_sections_stay_out_unless_show_all_or_current() {
    let mut settings = Settings::defaults();
    settings.hidden_places = vec![String::from("home"), String::from("documents")];
    settings.hidden_sections = vec![SectionId::Remote];

    let hidden_rows =
        compose_sidebar_rows(&settings, &[], false, Path::new("/definitely/not/a/place"));
    assert!(!hidden_rows.iter().any(|row| row.place.id == "home"));
    assert!(!hidden_rows.iter().any(|row| row.place.id == "network"));
    assert!(!hidden_rows
        .iter()
        .any(|row| row.section == SectionId::Remote));

    // The current place remains reachable even when it is hidden.
    let home = glib::home_dir();
    let current_rows = compose_sidebar_rows(&settings, &[], false, &home);
    let home_row = current_rows
        .iter()
        .find(|row| row.place.id == "home")
        .expect("current hidden home place");
    assert!(home_row.hidden);
    assert_eq!(home_row.header, Some(SectionId::Places));

    let all_rows = compose_sidebar_rows(&settings, &[], true, &home);
    let home_row = all_rows
        .iter()
        .find(|row| row.place.id == "home")
        .expect("home in show-all mode");
    assert!(home_row.hidden);
    let network_row = all_rows
        .iter()
        .find(|row| row.place.id == "network")
        .expect("network in show-all mode");
    assert!(network_row.section_hidden);
    assert_eq!(
        section_header_text(SectionId::Remote, true),
        "В сети (скрыт)"
    );
}

#[test]
fn full_path_breadcrumbs_and_anchors_ignore_visibility() {
    let mut settings = Settings::defaults();
    settings.sidebar_visible = false;
    settings.hidden_sections = SectionId::ALL.to_vec();
    settings.custom_places.push(CustomPlace {
        id: String::from("custom:5"),
        name: String::from("Временное"),
        path: PathBuf::from("/tmp"),
        icon: String::from("folder-temp"),
    });
    settings.places_order.push(String::from("custom:5"));
    settings.normalize();

    let anchors = path_anchors(&settings, &[]);
    assert!(anchors
        .iter()
        .any(|anchor| anchor.path == Path::new("/tmp") && anchor.label == "Временное"));

    let path = PathBuf::from("/tmp/Отчёты/2026");
    assert_eq!(
        build_breadcrumbs(&path, &anchors, true),
        vec![
            (String::from("/"), PathBuf::from("/")),
            (String::from("tmp"), PathBuf::from("/tmp")),
            (String::from("Отчёты"), PathBuf::from("/tmp/Отчёты")),
            (String::from("2026"), path.clone()),
        ]
    );
}

#[test]
fn edited_system_place_overrides_defaults_and_persists() {
    let mut settings = Settings::defaults();
    settings.edited_places = vec![
        CustomPlace {
            id: String::from("home"),
            name: String::from("  Личное  "),
            path: PathBuf::from("/tmp"),
            icon: String::from("  folder-home  "),
        },
        CustomPlace {
            id: String::from("home"),
            name: String::from("Дубликат"),
            path: PathBuf::from("/"),
            icon: String::from("folder"),
        },
        CustomPlace {
            id: String::from("bogus"),
            name: String::from("Неизвестно"),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder"),
        },
        CustomPlace {
            id: String::from("custom:1"),
            name: String::from("Своя запись"),
            path: PathBuf::from("/tmp"),
            icon: String::from("folder"),
        },
    ];
    settings.normalize();

    assert_eq!(settings.edited_places.len(), 1);
    assert_eq!(settings.edited_places[0].name, "Личное");
    assert_eq!(settings.edited_places[0].icon, "folder-home");

    let places = default_places(&settings);
    let home = places
        .iter()
        .find(|place| place.id == "home")
        .expect("home place");
    assert_eq!(home.label, "Личное");
    assert_eq!(home.icon, "folder-home");
    match &home.target {
        PlaceTarget::Directory(path) => assert_eq!(path, Path::new("/tmp")),
        _ => panic!("edited place lost its directory target"),
    }

    let dir = TempDir::new("edited-places");
    let path = dir.path.join("dolphy.conf");
    settings.save_to_path(&path).expect("save settings");
    let loaded = Settings::load_from_path(&path);
    assert_eq!(loaded.edited_places, settings.edited_places);
}

#[test]
fn icon_size_accepts_supported_values_including_48() {
    let mut settings = Settings::defaults();
    settings.sidebar_icon_size = 48;
    settings.normalize();
    assert_eq!(settings.sidebar_icon_size, 48);
    settings.sidebar_icon_size = 17;
    settings.normalize();
    assert_eq!(settings.sidebar_icon_size, DEFAULT_ICON_SIZE);
}

#[test]
fn sidebar_width_ignores_degenerate_and_extreme_paned_positions() {
    assert_eq!(sidebar_width_from_paned_position(0), None);
    assert_eq!(
        sidebar_width_from_paned_position(MIN_SIDEBAR_WIDTH - 1),
        None
    );
    assert_eq!(
        sidebar_width_from_paned_position(MIN_SIDEBAR_WIDTH),
        Some(MIN_SIDEBAR_WIDTH)
    );
    assert_eq!(sidebar_width_from_paned_position(300), Some(300));
    assert_eq!(sidebar_width_from_paned_position(9999), Some(4000));
}

#[test]
fn navigator_history_truncates_forward_entries() {
    let mut navigator = Navigator::new(PathBuf::from("/a"), false);
    assert!(!navigator.can_go_back());
    assert!(!navigator.can_go_forward());

    navigator.navigate_to(PathBuf::from("/a/b"));
    navigator.navigate_to(PathBuf::from("/a/b/c"));
    assert!(navigator.can_go_back());
    assert_eq!(navigator.current(), Path::new("/a/b/c"));

    assert!(navigator.go_back());
    assert_eq!(navigator.current(), Path::new("/a/b"));
    assert!(navigator.can_go_forward());

    navigator.navigate_to(PathBuf::from("/other"));
    assert_eq!(navigator.current(), Path::new("/other"));
    assert!(!navigator.can_go_forward());
    assert!(navigator.can_go_up());
    assert!(navigator.go_up());
    assert_eq!(navigator.current(), Path::new("/"));
    assert!(!navigator.can_go_up());
}

#[test]
fn formatting_helpers_keep_russian_plural_rules() {
    assert_eq!(plural_ru(1, "объект", "объекта", "объектов"), "1 объект");
    assert_eq!(plural_ru(2, "объект", "объекта", "объектов"), "2 объекта");
    assert_eq!(plural_ru(5, "объект", "объекта", "объектов"), "5 объектов");
    assert_eq!(
        plural_ru(11, "объект", "объекта", "объектов"),
        "11 объектов"
    );
    assert_eq!(plural_ru(21, "объект", "объекта", "объектов"), "21 объект");
    assert_eq!(human_size(0), "0 Б");
    assert_eq!(human_size(1024), "1.0 КиБ");
    assert_eq!(human_size(1536), "1.5 КиБ");
}

#[test]
fn config_with_wrong_value_types_uses_defaults_for_those_keys() {
    let dir = TempDir::new("types");
    let path = dir.path.join("dolphy.conf");
    std::fs::write(
            &path,
            "[General]\nversion=1\nshow_hidden=maybe\n[Sidebar]\nwidth=wide\nicon_size=17\nvisible=perhaps\n",
        )
        .expect("write config");

    let loaded = Settings::load_from_path(&path);
    assert_eq!(loaded.sidebar_width, DEFAULT_SIDEBAR_WIDTH);
    assert_eq!(loaded.sidebar_icon_size, DEFAULT_ICON_SIZE);
    assert!(!loaded.show_hidden);
    assert!(loaded.sidebar_visible);
}
