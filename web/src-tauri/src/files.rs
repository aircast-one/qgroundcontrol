use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
use tauri::async_runtime::spawn_blocking;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Manager, Runtime};

fn root<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|data| data.join("staged"))
        .map_err(|error| error.to_string())
}

fn inside(root: &Path, path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path.trim_start_matches('/'));
    relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
        .then(|| root.join(relative))
        .ok_or_else(|| format!("{path} is not a file the app staged."))
}

fn failed(error: std::io::Error) -> String {
    error.to_string()
}

async fn off_the_window<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    spawn_blocking(work)
        .await
        .map_err(|error| error.to_string())?
}

fn written(target: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    std::fs::write(target, bytes).map_err(failed)
}

fn removed_prefixed(root: &Path, prefix: &str, kept: Option<&Path>) -> Result<Vec<String>, String> {
    let (folder, stem) = prefix.rsplit_once('/').unwrap_or(("", prefix));
    let folder = inside(root, folder)?;
    let entries = match std::fs::read_dir(&folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        listed => listed.map_err(failed)?,
    };
    Ok(entries
        .map(|entry| entry.map(|entry| entry.path()).map_err(failed))
        .filter(|path| {
            path.as_ref().map_or(true, |path| {
                Some(path.as_path()) != kept
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with(stem))
            })
        })
        .filter_map(|path| {
            let path = match path {
                Ok(path) => path,
                Err(unreadable) => return Some(unreadable),
            };
            match path.is_dir() {
                true => std::fs::remove_dir_all(&path),
                false => std::fs::remove_file(&path),
            }
            .err()
            .map(|error| format!("{}: {error}", path.display()))
        })
        .collect())
}

fn listed(folder: &Path) -> Result<Value, String> {
    let entries = match std::fs::read_dir(folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Value::Array(Vec::new()));
        }
        listed => listed.map_err(failed)?,
    };
    Ok(entries
        .filter_map(|entry| {
            let entry = entry
                .inspect_err(|error| log::warn!(target: "desktop.files", "event=entry_unreadable folder=\"{}\" reason=\"{error}\"", folder.display()))
                .ok()?;
            let meta = entry
                .metadata()
                .inspect_err(|error| log::warn!(target: "desktop.files", "event=entry_unreadable file=\"{}\" reason=\"{error}\"", entry.path().display()))
                .ok()?;
            let modified = meta
                .modified()
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .map(|since| since.as_millis());
            Some(json!({
                "name": entry.file_name().to_string_lossy(),
                "size": meta.len(),
                "dir": meta.is_dir(),
                "modified": modified,
            }))
        })
        .collect())
}

fn inside_saves(path: &str) -> Result<PathBuf, String> {
    let saves = groundstation::settingsstore::save_root()
        .map(PathBuf::from)
        .ok_or("The app has no save folder.")?;
    let wanted = PathBuf::from(path).canonicalize().map_err(failed)?;
    let saves = saves.canonicalize().map_err(failed)?;
    wanted
        .starts_with(&saves)
        .then_some(wanted)
        .ok_or_else(|| format!("{path} is outside the save folder."))
}

#[tauri::command]
pub fn staged_root<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    Ok(root(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn staged_write<R: Runtime>(
    app: AppHandle<R>,
    request: Request<'_>,
) -> Result<(), String> {
    let path = request
        .headers()
        .get("path")
        .and_then(|value| value.to_str().ok())
        .map(|value| percent_decode_str(value).decode_utf8_lossy().into_owned())
        .ok_or("The staged file has no name.")?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("The staged file has no bytes.".to_string());
    };
    let (target, bytes) = (inside(&root(&app)?, &path)?, bytes.clone());
    off_the_window(move || written(&target, &bytes)).await
}

#[tauri::command]
pub async fn staged_read<R: Runtime>(app: AppHandle<R>, path: String) -> Result<Response, String> {
    let target = inside(&root(&app)?, &path)?;
    off_the_window(move || std::fs::read(target).map(Response::new).map_err(failed)).await
}

#[tauri::command]
pub async fn staged_prepare<R: Runtime>(app: AppHandle<R>, path: String) -> Result<String, String> {
    let target = inside(&root(&app)?, &path)?;
    off_the_window(move || {
        if let Some(folder) = target.parent() {
            std::fs::create_dir_all(folder).map_err(failed)?;
        }
        Ok(target.to_string_lossy().into_owned())
    })
    .await
}

#[tauri::command]
pub async fn staged_remove<R: Runtime>(app: AppHandle<R>, path: String) -> Result<bool, String> {
    let target = inside(&root(&app)?, &path)?;
    off_the_window(move || match std::fs::remove_file(&target) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => {
            log::warn!(target: "desktop.files", "event=staged_not_removed file=\"{}\" reason=\"{error}\"", target.display());
            Ok(false)
        }
    })
    .await
}

#[tauri::command]
pub async fn staged_remove_prefixed<R: Runtime>(
    app: AppHandle<R>,
    prefix: String,
    keep: Option<String>,
) -> Result<(), String> {
    let root = root(&app)?;
    let kept = keep.map(|path| inside(&root, &path)).transpose()?;
    let stem = prefix.clone();
    let refused = off_the_window(move || removed_prefixed(&root, &stem, kept.as_deref())).await?;
    if let Some(first) = refused.first() {
        log::warn!(target: "desktop.files", "event=staged_cleanup_incomplete prefix=\"{prefix}\" refused={} first=\"{first}\"", refused.len());
    }
    Ok(())
}

#[tauri::command]
pub async fn staged_list<R: Runtime>(app: AppHandle<R>, dir: String) -> Result<Value, String> {
    let folder = inside(&root(&app)?, &dir)?;
    off_the_window(move || listed(&folder)).await
}

#[tauri::command]
pub async fn saved_read(path: String) -> Result<Response, String> {
    off_the_window(move || {
        std::fs::read(inside_saves(&path)?)
            .map(Response::new)
            .map_err(failed)
    })
    .await
}

#[tauri::command]
pub async fn saved_list(path: String) -> Result<Value, String> {
    off_the_window(move || listed(&inside_saves(&path)?)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_names_stay_inside_the_staging_folder() {
        let root = Path::new("/data/staged");
        assert_eq!(
            inside(root, "/replay-links/link_1/flight.tlog").unwrap(),
            Path::new("/data/staged/replay-links/link_1/flight.tlog")
        );
        assert!(inside(root, "/scripts/../../etc/passwd").is_err());
        assert!(inside(root, "/").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_keeps_the_named_file_and_reports_what_it_could_not_remove() {
        let folder = std::env::temp_dir().join(format!("aircast-staged-{}", std::process::id()));
        let (kept, stale, locked) = (
            folder.join("flight.tlog"),
            folder.join("old.tlog"),
            folder.join("locked"),
        );
        written(&kept, b"kept").unwrap();
        written(&stale, b"stale").unwrap();
        written(&locked.join("inside.tlog"), b"held").unwrap();
        written(&folder.join("shape.dbf"), b"part").unwrap();
        written(&folder.join("shapes.txt"), b"other").unwrap();
        let mode = |bits| std::os::unix::fs::PermissionsExt::from_mode(bits);
        std::fs::set_permissions(&locked, mode(0o555)).unwrap();
        let root = folder.parent().unwrap();
        let named =
            |name: &str| format!("/{}/{name}", folder.file_name().unwrap().to_string_lossy());
        assert!(
            removed_prefixed(root, &named("shape."), None)
                .unwrap()
                .is_empty()
        );
        assert!(
            !folder.join("shape.dbf").exists() && folder.join("shapes.txt").exists(),
            "a prefix matches file names, as the browser's file store does, not whole folders"
        );
        let refused = removed_prefixed(root, &named(""), Some(&kept)).unwrap();
        std::fs::set_permissions(&locked, mode(0o755)).unwrap();
        assert!(kept.exists() && !stale.exists());
        assert_eq!(
            refused.len(),
            1,
            "a folder whose files cannot be deleted is reported, and the rest is still cleaned"
        );
        assert!(refused[0].contains("locked"));
        assert!(
            removed_prefixed(root, "/missing/", None)
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
