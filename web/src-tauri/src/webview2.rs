use tauri::{Runtime, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION,
    COREWEBVIEW2_PERMISSION_STATE_ALLOW,
};
use webview2_com::PermissionRequestedEventHandler;

pub fn allow_location<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.with_webview(|webview| unsafe {
        let handler = PermissionRequestedEventHandler::create(Box::new(|_, request| {
            let Some(request) = request else {
                return Ok(());
            };
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            request.PermissionKind(&mut kind)?;
            if kind == COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION {
                request.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
            }
            Ok(())
        }));
        let mut token = Default::default();
        if let Ok(core) = webview.controller().CoreWebView2() {
            let _ = core.add_PermissionRequested(&handler, &mut token);
        }
    })
}
