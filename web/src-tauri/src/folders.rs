use std::path::{Path, PathBuf};

const ORGANIZATION: &str = "Aircast";
const DEFAULT_APPLICATION: &str = "Aircast QGC";
const INI: &str = "ini";

fn chosen(given: &[String]) -> bool {
    given
        .iter()
        .any(|argument| argument == "--settings" || argument == "--app-name")
}

pub fn settings_folder(home: &Path) -> PathBuf {
    home.join("Library/Application Support/settings")
        .join(ORGANIZATION)
}

pub fn mac_head_arguments(home: &Path, given: &[String], inis: &[String]) -> Vec<String> {
    if chosen(given) {
        return Vec::new();
    }
    let folder = settings_folder(home);
    let settings = inis.iter().min().map_or_else(
        || folder.join(format!("{DEFAULT_APPLICATION}.{INI}")),
        |ini| folder.join(ini),
    );
    let application = settings
        .file_stem()
        .map_or(DEFAULT_APPLICATION.into(), |stem| stem.to_string_lossy());
    let shown = |path: PathBuf| path.to_string_lossy().into_owned();
    vec![
        "--settings".to_string(),
        shown(settings.clone()),
        "--app-name".to_string(),
        application.to_string(),
        "--map-cache".to_string(),
        shown(home.join("Library/Caches/QGCMapCache/qgcMapCache.db")),
        "--save-path".to_string(),
        shown(home.join("Documents").join(application.as_ref())),
    ]
}

pub fn ini_names(folder: &Path) -> Vec<String> {
    std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == INI))
                .filter_map(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_app_on_macos_opens_the_settings_maps_and_files_the_native_app_kept() {
        let home = Path::new("/Users/pilot");
        assert_eq!(
            mac_head_arguments(home, &["aircast".to_string()], &[]),
            [
                "--settings",
                "/Users/pilot/Library/Application Support/settings/Aircast/Aircast QGC.ini",
                "--app-name",
                "Aircast QGC",
                "--map-cache",
                "/Users/pilot/Library/Caches/QGCMapCache/qgcMapCache.db",
                "--save-path",
                "/Users/pilot/Documents/Aircast QGC",
            ]
        );
        let kept = mac_head_arguments(
            home,
            &[],
            &["Zeta.ini".to_string(), "Aircast Field.ini".to_string()],
        );
        assert_eq!(
            (&kept[1], &kept[3], &kept[7]),
            (
                &"/Users/pilot/Library/Application Support/settings/Aircast/Aircast Field.ini"
                    .to_string(),
                &"Aircast Field".to_string(),
                &"/Users/pilot/Documents/Aircast Field".to_string()
            ),
            "like the native app, the first settings file by name is the one in use"
        );
        assert!(
            mac_head_arguments(home, &["--app-name".to_string(), "Bench".to_string()], &[])
                .is_empty(),
            "a run that names its own app keeps the core's folders for it"
        );
    }

    #[test]
    fn only_settings_files_are_counted() {
        let folder = std::env::temp_dir().join(format!("aircast-folders-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        ["Aircast QGC.ini", "notes.txt", "Bench.ini.bak"]
            .iter()
            .for_each(|name| std::fs::write(folder.join(name), b"").unwrap());
        assert_eq!(ini_names(&folder), ["Aircast QGC.ini"]);
        std::fs::remove_dir_all(&folder).unwrap();
        assert!(ini_names(&folder).is_empty());
    }
}
