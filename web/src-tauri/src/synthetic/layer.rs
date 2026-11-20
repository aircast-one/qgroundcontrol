use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_int, c_void};

use groundstation::nativehost::{qgc_synthetic_attach, qgc_synthetic_release};
use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_core_graphics::CGColor;
use objc2_quartz_core::CAMetalLayer;
use serde::Deserialize;
use tauri::async_runtime::spawn_blocking;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow};
use tokio::sync::oneshot;

use super::Views;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub viewport_height: f64,
    pub shown: bool,
}

pub fn frame_in(placement: &Placement, webview: CGRect, flipped: bool) -> CGRect {
    let y = match flipped {
        true => webview.origin.y + webview.size.height - placement.viewport_height + placement.y,
        false => webview.origin.y + placement.viewport_height - placement.y - placement.height,
    };
    CGRect::new(
        CGPoint::new(webview.origin.x + placement.x, y),
        CGSize::new(placement.width, placement.height),
    )
}

struct Shown {
    view: Retained<NSView>,
    layer: Retained<CAMetalLayer>,
    orbit: bool,
    pixels: (u32, u32),
}

impl Shown {
    fn key(&self) -> usize {
        Retained::as_ptr(&self.layer) as usize
    }

    fn place(&mut self, placement: &Placement, webview: &NSView) -> Result<(), String> {
        let parent = unsafe { webview.superview() }
            .ok_or("The page is no longer in a window, so the 3D view has nowhere to go.")?;
        self.view
            .setFrame(frame_in(placement, webview.frame(), parent.isFlipped()));
        self.layer.setContentsScale(placement.scale);
        self.view.setHidden(!placement.shown);
        let pixels = (placement.pixel_width, placement.pixel_height);
        if !placement.shown || pixels == self.pixels || pixels.0 == 0 || pixels.1 == 0 {
            return Ok(());
        }
        let side = |pixels: u32| c_int::try_from(pixels).unwrap_or(c_int::MAX);
        let attached = unsafe {
            qgc_synthetic_attach(
                Retained::as_ptr(&self.layer).cast_mut().cast::<c_void>(),
                side(pixels.0),
                side(pixels.1),
                c_int::from(self.orbit),
            )
        };
        if attached == 0 {
            return Err("The core could not draw into the 3D view.".to_string());
        }
        self.pixels = pixels;
        Ok(())
    }
}

thread_local! {
    static SHOWN: RefCell<HashMap<usize, Shown>> = RefCell::default();
}

fn forget(key: usize) -> bool {
    SHOWN
        .with_borrow_mut(|shown| shown.remove(&key))
        .inspect(|gone| gone.view.removeFromSuperview())
        .is_some()
}

pub fn forget_all() -> Vec<usize> {
    let keys: Vec<usize> = SHOWN.with_borrow(|shown| shown.keys().copied().collect());
    keys.into_iter().filter(|key| forget(*key)).collect()
}

pub fn release_each(keys: Vec<usize>) {
    keys.into_iter()
        .for_each(|key| qgc_synthetic_release(key as *mut c_void));
}

pub fn page_left<R: Runtime>(app: &AppHandle<R>) {
    if let Err(error) = app.run_on_main_thread(|| {
        let keys = forget_all();
        spawn_blocking(move || release_each(keys));
    }) {
        log::warn!(target: "desktop.synthetic", "event=layers_not_released reason=\"{error}\"");
    }
}

async fn on_main<R: Runtime, T: Send + 'static>(
    window: &WebviewWindow<R>,
    work: impl FnOnce(&NSView) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (answer, answered) = oneshot::channel();
    window
        .with_webview(move |platform| {
            let webview = unsafe { &*platform.inner().cast::<NSView>() };
            if answer.send(work(webview)).is_err() {
                log::debug!(target: "desktop.synthetic", "event=layer_answer_unwanted");
            }
        })
        .map_err(|error| error.to_string())?;
    answered.await.map_err(|error| error.to_string())?
}

fn opened(webview: &NSView, orbit: bool, placement: &Placement) -> Result<usize, String> {
    let main = MainThreadMarker::new().ok_or("The 3D view can only open on the main thread.")?;
    let parent = unsafe { webview.superview() }
        .ok_or("The page is not in a window yet, so the 3D view has nowhere to go.")?;
    let layer = CAMetalLayer::new();
    layer.setBackgroundColor(Some(&CGColor::new_generic_gray(0.0, 1.0)));
    let view = NSView::initWithFrame(NSView::alloc(main), CGRect::ZERO);
    view.setLayer(Some(&layer));
    view.setWantsLayer(true);
    parent.addSubview_positioned_relativeTo(&view, NSWindowOrderingMode::Below, Some(webview));
    let mut shown = Shown {
        view,
        layer,
        orbit,
        pixels: (0, 0),
    };
    if let Err(reason) = shown.place(placement, webview) {
        shown.view.removeFromSuperview();
        return Err(reason);
    }
    let key = shown.key();
    SHOWN.with_borrow_mut(|all| all.insert(key, shown));
    Ok(key)
}

#[tauri::command]
pub async fn synthetic_layer_open<R: Runtime>(
    window: WebviewWindow<R>,
    orbit: bool,
    placement: Placement,
    views: tauri::State<'_, Views>,
) -> Result<usize, String> {
    let page = views.page();
    let app = window.app_handle().clone();
    on_main(&window, move |webview| {
        if app.state::<Views>().page() != page {
            return Err("The page reloaded before the 3D view opened.".to_string());
        }
        opened(webview, orbit, &placement)
    })
    .await
}

#[tauri::command]
pub async fn synthetic_layer_place<R: Runtime>(
    window: WebviewWindow<R>,
    key: usize,
    placement: Placement,
) -> Result<(), String> {
    on_main(&window, move |webview| {
        SHOWN.with_borrow_mut(|all| {
            all.get_mut(&key)
                .map_or(Ok(()), |shown| shown.place(&placement, webview))
        })
    })
    .await
}

#[tauri::command]
pub async fn synthetic_layer_close<R: Runtime>(
    window: WebviewWindow<R>,
    key: usize,
) -> Result<(), String> {
    if on_main(&window, move |_| Ok(forget(key))).await? {
        spawn_blocking(move || release_each(vec![key]))
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> CGRect {
        CGRect::new(CGPoint::new(x, y), CGSize::new(width, height))
    }

    fn placement(x: f64, y: f64, width: f64, height: f64, viewport_height: f64) -> Placement {
        Placement {
            x,
            y,
            width,
            height,
            scale: 2.0,
            pixel_width: (width * 2.0) as u32,
            pixel_height: (height * 2.0) as u32,
            viewport_height,
            shown: true,
        }
    }

    #[test]
    fn a_page_rect_lands_on_the_same_spot_in_appkit_bottom_up_coordinates() {
        let webview = rect(0.0, 0.0, 1440.0, 868.0);
        assert_eq!(
            frame_in(&placement(0.0, 0.0, 1440.0, 868.0, 868.0), webview, false),
            webview,
            "a 3D view filling the page fills the webview"
        );
        assert_eq!(
            frame_in(&placement(100.0, 50.0, 400.0, 300.0, 868.0), webview, false),
            rect(100.0, 518.0, 400.0, 300.0),
            "the page counts down from the top, AppKit counts up from the bottom"
        );
    }

    #[test]
    fn a_webview_reaching_under_the_title_bar_starts_the_page_below_it() {
        let under_title_bar = rect(0.0, 0.0, 1440.0, 900.0);
        assert_eq!(
            frame_in(
                &placement(0.0, 0.0, 1440.0, 868.0, 868.0),
                under_title_bar,
                false
            ),
            rect(0.0, 0.0, 1440.0, 868.0),
            "the 32 points the title bar hides hold no page"
        );
        assert_eq!(
            frame_in(
                &placement(0.0, 0.0, 1440.0, 868.0, 868.0),
                under_title_bar,
                true
            ),
            rect(0.0, 32.0, 1440.0, 868.0),
            "a flipped window view counts down like the page, from below the title bar"
        );
    }

    #[test]
    fn the_webviews_own_offset_in_its_window_view_counts() {
        assert_eq!(
            frame_in(
                &placement(100.0, 50.0, 400.0, 300.0, 844.0),
                rect(10.0, 28.0, 1420.0, 844.0),
                false
            ),
            rect(110.0, 522.0, 400.0, 300.0)
        );
    }
}
