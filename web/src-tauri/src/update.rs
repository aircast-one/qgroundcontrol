use std::path::{Path, PathBuf};

use serde_json::Value;
use tauri::{AppHandle, Runtime};

const APP_UPDATE: &str = "view.appUpdate";
const READY: &str = "ready";
const NOT_READY: &str = "No update is ready to install.";
const DISCONNECT_FIRST: &str = "Disconnect from the vehicle first";

#[cfg(target_os = "macos")]
const REPLACE_BUNDLE: &str = r#"
pid="$1" dmg="$2" bundle="$3"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
mount="$(mktemp -d)"
if ! hdiutil attach -quiet -nobrowse -readonly -mountpoint "$mount" "$dmg"; then
  open "$dmg"
  exit 1
fi
app="$(find "$mount" -maxdepth 1 -name '*.app' | head -n 1)"
if [ -n "$app" ] && ditto "$app" "$bundle.updating"; then
  rm -rf "$bundle" && mv "$bundle.updating" "$bundle"
fi
hdiutil detach -quiet "$mount" -force
open -n "$bundle"
"#;

pub fn ready_installer(update: &Value) -> Result<PathBuf, String> {
    let file = update["file"]
        .as_str()
        .filter(|file| !file.is_empty() && update["phase"] == READY)
        .ok_or(NOT_READY)?;
    match update["canInstall"].as_bool() {
        Some(true) => Ok(PathBuf::from(file)),
        _ => Err(DISCONNECT_FIRST.to_string()),
    }
}

#[cfg(any(target_os = "macos", test))]
pub fn bundle_of(executable: &Path) -> Option<PathBuf> {
    executable
        .ancestors()
        .find(|folder| {
            folder
                .extension()
                .is_some_and(|extension| extension == "app")
        })
        .map(Path::to_path_buf)
}

#[cfg(target_os = "macos")]
fn writable(folder: &Path) -> bool {
    let probe = folder.join(format!(".aircast-update-{}", std::process::id()));
    let made = std::fs::File::create(&probe).is_ok();
    let _ = std::fs::remove_file(&probe);
    made
}

#[cfg(target_os = "macos")]
fn start_installer(installer: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let replaceable = bundle_of(&executable).filter(|bundle| bundle.parent().is_some_and(writable));
    let Some(bundle) = replaceable else {
        return std::process::Command::new("open")
            .arg(installer)
            .spawn()
            .map(drop)
            .map_err(|error| error.to_string());
    };
    std::process::Command::new("/bin/sh")
        .args(["-c", REPLACE_BUNDLE, "aircast-update"])
        .arg(std::process::id().to_string())
        .arg(installer)
        .arg(&bundle)
        .process_group(0)
        .spawn()
        .map(drop)
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn start_installer(installer: &Path) -> Result<(), String> {
    std::process::Command::new(installer)
        .arg("/S")
        .spawn()
        .map(drop)
        .map_err(|error| error.to_string())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn start_installer(_: &Path) -> Result<(), String> {
    Err("Aircast cannot install updates on this system.".to_string())
}

#[tauri::command]
pub fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let installer = ready_installer(&crate::bridge::core_value(APP_UPDATE))?;
    if let Some(unsaved) = crate::window::work_at_risk().first() {
        return Err(format!("{unsaved} Save it, then install the update."));
    }
    start_installer(&installer).inspect_err(|error| {
        log::warn!(target: "desktop.update", "event=update_not_started file=\"{}\" reason=\"{error}\"", installer.display());
    })?;
    log::info!(target: "desktop.update", "event=update_installing file=\"{}\"", installer.display());
    crate::window::quit_now(&app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_a_downloaded_update_installs_and_never_with_a_vehicle_connected() {
        let ready = json!({ "phase": "ready", "file": "/u/Updates/AircastQGCNative-v5.6.6-beta.1.dmg", "canInstall": true });
        assert_eq!(
            ready_installer(&ready),
            Ok(PathBuf::from(
                "/u/Updates/AircastQGCNative-v5.6.6-beta.1.dmg"
            ))
        );
        assert_eq!(
            ready_installer(&json!({ "phase": "ready", "file": "/u/x.dmg", "canInstall": false })),
            Err(DISCONNECT_FIRST.to_string())
        );
        assert_eq!(
            ready_installer(
                &json!({ "phase": "downloading", "file": "/u/x.dmg", "canInstall": true })
            ),
            Err(NOT_READY.to_string())
        );
        assert_eq!(ready_installer(&Value::Null), Err(NOT_READY.to_string()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_update_replaces_the_bundle_once_the_app_has_quit_and_opens_it_again() {
        let base = std::env::temp_dir().join(format!("aircast-update-{}", std::process::id()));
        let (installed, shipped, tools) = (
            base.join("Applications"),
            base.join("shipped"),
            base.join("bin"),
        );
        let app = |folder: &Path, marker: &str| {
            let contents = folder.join("Aircast.app/Contents");
            std::fs::create_dir_all(&contents).unwrap();
            std::fs::write(contents.join("version"), marker).unwrap();
        };
        app(&installed, "old");
        app(&shipped, "new");
        std::fs::create_dir_all(&tools).unwrap();
        let opened = base.join("opened");
        std::fs::write(
            tools.join("open"),
            format!("#!/bin/sh\necho \"$@\" >> '{}'\n", opened.display()),
        )
        .unwrap();
        std::fs::set_permissions(
            tools.join("open"),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        let dmg = base.join("update.dmg");
        assert!(
            std::process::Command::new("hdiutil")
                .args(["create", "-quiet", "-fs", "HFS+", "-srcfolder"])
                .arg(&shipped)
                .arg(&dmg)
                .status()
                .unwrap()
                .success()
        );
        let mut running = std::process::Command::new("sleep")
            .arg("1")
            .spawn()
            .unwrap();
        let quitting = running.id();
        let quit = std::thread::spawn(move || running.wait().unwrap());
        let bundle = installed.join("Aircast.app");
        let finished = std::process::Command::new("/bin/sh")
            .args(["-c", REPLACE_BUNDLE, "aircast-update"])
            .arg(quitting.to_string())
            .arg(&dmg)
            .arg(&bundle)
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin:/usr/sbin", tools.display()),
            )
            .status()
            .unwrap();
        assert!(
            quit.is_finished(),
            "the bundle is only replaced after the app has quit"
        );
        assert!(finished.success());
        assert_eq!(
            std::fs::read_to_string(bundle.join("Contents/version")).unwrap(),
            "new"
        );
        assert!(!installed.join("Aircast.app.updating").exists());
        assert_eq!(
            std::fs::read_to_string(&opened).unwrap().trim(),
            format!("-n {}", bundle.display())
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn the_running_bundle_is_found_from_its_executable() {
        assert_eq!(
            bundle_of(Path::new(
                "/Applications/Aircast.app/Contents/MacOS/aircast-desktop"
            )),
            Some(PathBuf::from("/Applications/Aircast.app"))
        );
        assert_eq!(bundle_of(Path::new("/usr/local/bin/aircast-desktop")), None);
    }
}
