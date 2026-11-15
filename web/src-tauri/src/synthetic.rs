use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use groundstation::nativehost::{synthetic_offscreen, synthetic_offscreen_release};
use tauri::async_runtime::spawn_blocking;
use tauri::ipc::{Channel, InvokeResponseBody};

type Deliver = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

#[derive(Default)]
pub struct Views {
    next: AtomicUsize,
    shown: Mutex<HashMap<usize, (bool, Deliver)>>,
}

fn attach(key: usize, size: (u32, u32), orbit: bool, deliver: Deliver) -> Result<(), String> {
    synthetic_offscreen(key, size, orbit, deliver)
        .then_some(())
        .ok_or_else(|| "The 3D view could not start.".to_string())
}

async fn attached(
    key: usize,
    size: (u32, u32),
    orbit: bool,
    deliver: Deliver,
) -> Result<(), String> {
    spawn_blocking(move || attach(key, size, orbit, deliver))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn synthetic_open(
    width: u32,
    height: u32,
    orbit: bool,
    frames: Channel<InvokeResponseBody>,
    views: tauri::State<'_, Views>,
) -> Result<usize, String> {
    let key = views.next.fetch_add(1, Ordering::Relaxed) + 1;
    let deliver: Deliver = Arc::new(move |jpeg| {
        if let Err(error) = frames.send(InvokeResponseBody::Raw(jpeg)) {
            log::debug!(target: "desktop.synthetic", "event=frame_not_delivered view={key} reason=\"{error}\"");
        }
    });
    attached(key, (width, height), orbit, deliver.clone()).await?;
    views
        .shown
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, (orbit, deliver));
    Ok(key)
}

#[tauri::command]
pub async fn synthetic_resize(
    key: usize,
    width: u32,
    height: u32,
    views: tauri::State<'_, Views>,
) -> Result<(), String> {
    let shown = views
        .shown
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        .cloned();
    match shown {
        Some((orbit, deliver)) => attached(key, (width, height), orbit, deliver).await,
        None => Ok(()),
    }
}

#[tauri::command]
pub async fn synthetic_close(key: usize, views: tauri::State<'_, Views>) -> Result<(), String> {
    views
        .shown
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&key);
    spawn_blocking(move || synthetic_offscreen_release(key))
        .await
        .map_err(|error| error.to_string())
}
