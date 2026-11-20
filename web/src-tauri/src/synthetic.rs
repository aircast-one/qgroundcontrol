use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use groundstation::nativehost::{synthetic_offscreen, synthetic_offscreen_release};
use tauri::async_runtime::spawn_blocking;
use tauri::ipc::{Channel, InvokeResponseBody};

#[cfg(target_os = "macos")]
pub mod layer;

type Deliver = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

#[derive(Default)]
pub struct Views {
    next: AtomicUsize,
    page: AtomicU64,
    shown: Mutex<HashMap<usize, (bool, Deliver)>>,
}

impl Views {
    fn shown(&self) -> std::sync::MutexGuard<'_, HashMap<usize, (bool, Deliver)>> {
        self.shown.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn page(&self) -> u64 {
        self.page.load(Ordering::Relaxed)
    }

    pub fn forget_all(&self) -> Vec<usize> {
        let mut shown = self.shown();
        self.page.fetch_add(1, Ordering::Relaxed);
        shown.drain().map(|(key, _)| key).collect()
    }

    fn keep(&self, key: usize, page: u64, view: (bool, Deliver)) -> bool {
        let mut shown = self.shown();
        let current = self.page() == page;
        if current {
            shown.insert(key, view);
        }
        current
    }

    pub fn release_each(keys: Vec<usize>) {
        keys.into_iter().for_each(synthetic_offscreen_release);
    }
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
    let page = views.page();
    let key = views.next.fetch_add(1, Ordering::Relaxed) + 1;
    let deliver: Deliver = Arc::new(move |jpeg| {
        if let Err(error) = frames.send(InvokeResponseBody::Raw(jpeg)) {
            log::debug!(target: "desktop.synthetic", "event=frame_not_delivered view={key} reason=\"{error}\"");
        }
    });
    attached(key, (width, height), orbit, deliver.clone()).await?;
    if views.keep(key, page, (orbit, deliver)) {
        return Ok(key);
    }
    spawn_blocking(move || synthetic_offscreen_release(key))
        .await
        .map_err(|error| error.to_string())?;
    Err("The page reloaded before the 3D view opened.".to_string())
}

#[tauri::command]
pub async fn synthetic_resize(
    key: usize,
    width: u32,
    height: u32,
    views: tauri::State<'_, Views>,
) -> Result<(), String> {
    let Some((orbit, deliver)) = views.shown().get(&key).cloned() else {
        return Ok(());
    };
    attached(key, (width, height), orbit, deliver).await?;
    if views.shown().contains_key(&key) {
        return Ok(());
    }
    spawn_blocking(move || synthetic_offscreen_release(key))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn synthetic_close(key: usize, views: tauri::State<'_, Views>) -> Result<(), String> {
    views.shown().remove(&key);
    spawn_blocking(move || synthetic_offscreen_release(key))
        .await
        .map_err(|error| error.to_string())
}
