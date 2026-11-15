use tauri::{Runtime, WebviewWindow};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION,
    COREWEBVIEW2_PERMISSION_STATE_ALLOW,
};
use webview2_com::{PermissionRequestedEventHandler, take_pwstr};

pub fn allow_location<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let origin = window.url()?.origin().ascii_serialization();
    window.with_webview(move |webview| unsafe {
        let handler = PermissionRequestedEventHandler::create(Box::new(move |_, request| {
            let Some(request) = request else {
                return Ok(());
            };
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            request.PermissionKind(&mut kind)?;
            let mut asking = Default::default();
            request.Uri(&mut asking)?;
            let asked_origin = url::Url::parse(&take_pwstr(asking))
                .map(|asked| asked.origin().ascii_serialization())
                .ok();
            if kind == COREWEBVIEW2_PERMISSION_KIND_GEOLOCATION
                && asked_origin.as_deref() == Some(origin.as_str())
            {
                request.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
            }
            Ok(())
        }));
        let mut token = Default::default();
        if let Err(error) = webview
            .controller()
            .CoreWebView2()
            .and_then(|core| core.add_PermissionRequested(&handler, &mut token))
        {
            log::warn!(target: "desktop.window", "event=location_grant_unavailable reason=\"{error}\"");
        }
    })
}
