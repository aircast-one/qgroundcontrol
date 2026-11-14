use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use groundstation::nativehost::{synthetic_offscreen, synthetic_offscreen_release};
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

#[tauri::command(async)]
pub fn synthetic_open(
    width: u32,
    height: u32,
    orbit: bool,
    frames: Channel<InvokeResponseBody>,
    views: tauri::State<'_, Views>,
) -> Result<usize, String> {
    let key = views.next.fetch_add(1, Ordering::Relaxed) + 1;
    let deliver: Deliver = Arc::new(move |jpeg| {
        let _ = frames.send(InvokeResponseBody::Raw(jpeg));
    });
    attach(key, (width, height), orbit, deliver.clone())?;
    views
        .shown
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, (orbit, deliver));
    Ok(key)
}

#[tauri::command(async)]
pub fn synthetic_resize(
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
        Some((orbit, deliver)) => attach(key, (width, height), orbit, deliver),
        None => Ok(()),
    }
}

#[tauri::command(async)]
pub fn synthetic_close(key: usize, views: tauri::State<'_, Views>) {
    views
        .shown
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&key);
    synthetic_offscreen_release(key);
}
