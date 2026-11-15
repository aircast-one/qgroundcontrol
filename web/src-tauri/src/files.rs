use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use percent_encoding::percent_decode_str;
use serde_json::{Value, json};
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

#[tauri::command]
pub fn staged_root<R: Runtime>(app: AppHandle<R>) -> Result<String, String> {
    Ok(root(&app)?.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn staged_write<R: Runtime>(app: AppHandle<R>, request: Request<'_>) -> Result<(), String> {
    let path = request
        .headers()
        .get("path")
        .and_then(|value| value.to_str().ok())
        .map(|value| percent_decode_str(value).decode_utf8_lossy().into_owned())
        .ok_or("The staged file has no name.")?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("The staged file has no bytes.".to_string());
    };
    let target = inside(&root(&app)?, &path)?;
    if let Some(folder) = target.parent() {
        std::fs::create_dir_all(folder).map_err(failed)?;
    }
    std::fs::write(target, bytes).map_err(failed)
}

#[tauri::command]
pub fn staged_read<R: Runtime>(app: AppHandle<R>, path: String) -> Result<Response, String> {
    std::fs::read(inside(&root(&app)?, &path)?)
        .map(Response::new)
        .map_err(failed)
}

#[tauri::command]
pub fn staged_remove<R: Runtime>(app: AppHandle<R>, path: String) -> Result<bool, String> {
    Ok(std::fs::remove_file(inside(&root(&app)?, &path)?).is_ok())
}

#[tauri::command]
pub fn staged_remove_prefixed<R: Runtime>(
    app: AppHandle<R>,
    prefix: String,
    keep: Option<String>,
) -> Result<(), String> {
    let root = root(&app)?;
    let kept = keep.map(|path| inside(&root, &path)).transpose()?;
    let folder = inside(&root, &prefix)?;
    let entries = match std::fs::read_dir(&folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        listed => listed.map_err(failed)?,
    };
    let refused: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| Some(path) != kept.as_ref())
        .filter_map(|path| {
            match path.is_dir() {
                true => std::fs::remove_dir_all(&path),
                false => std::fs::remove_file(&path),
            }
            .err()
            .map(|error| format!("{}: {error}", path.display()))
        })
        .collect();
    refused
        .is_empty()
        .then_some(())
        .ok_or_else(|| refused.join("; "))
}

#[tauri::command]
pub fn staged_list<R: Runtime>(app: AppHandle<R>, dir: String) -> Result<Value, String> {
    let entries = match std::fs::read_dir(inside(&root(&app)?, &dir)?) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Value::Array(Vec::new()));
        }
        listed => listed.map_err(failed)?,
    };
    Ok(entries
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
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

#[tauri::command]
pub fn saved_read(path: String) -> Result<Response, String> {
    let saves = groundstation::settingsstore::save_root()
        .map(PathBuf::from)
        .ok_or("The app has no save folder.")?;
    let wanted = PathBuf::from(&path).canonicalize().map_err(failed)?;
    let saves = saves.canonicalize().map_err(failed)?;
    wanted
        .starts_with(&saves)
        .then_some(())
        .ok_or_else(|| format!("{path} is outside the save folder."))?;
    std::fs::read(wanted).map(Response::new).map_err(failed)
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
}
