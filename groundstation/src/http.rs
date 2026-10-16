use std::time::Duration;

#[cfg(not(target_family = "wasm"))]
static AGENT: std::sync::LazyLock<ureq::Agent> = std::sync::LazyLock::new(ureq::Agent::new_with_defaults);

#[cfg(not(target_family = "wasm"))]
pub fn get(url: &str, headers: &[(&str, &str)], timeout: Duration) -> Result<Vec<u8>, String> {
    let asked = headers.iter().fold(AGENT.get(url), |asked, (name, value)| asked.header(*name, *value));
    asked.config().timeout_global(Some(timeout)).build().call().map_err(|e| e.to_string())?.body_mut().read_to_vec().map_err(|e| e.to_string())
}

#[cfg(not(target_family = "wasm"))]
pub fn post(url: &str, content_type: &str, body: &[u8], timeout: Duration) -> Result<(u16, Vec<u8>), String> {
    let mut answer = AGENT.post(url).header("Content-Type", content_type).config().http_status_as_error(false).timeout_global(Some(timeout)).build().send(body).map_err(|e| e.to_string())?;
    let status = answer.status().as_u16();
    answer.body_mut().read_to_vec().map(|bytes| (status, bytes)).map_err(|e| e.to_string())
}

#[cfg(target_family = "wasm")]
fn exchange(method: &str, url: &str, headers: &[(&str, &str)], body: Option<&[u8]>, timeout: Duration) -> Result<(u16, Vec<u8>), String> {
    let failed = |error: wasm_bindgen::JsValue| error.as_string().unwrap_or_else(|| format!("{error:?}"));
    let request = web_sys::XmlHttpRequest::new().map_err(failed)?;
    request.open_with_async(method, url, false).map_err(failed)?;
    request.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);
    request.set_timeout(u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX));
    headers.iter().try_for_each(|(name, value)| request.set_request_header(name, value)).map_err(failed)?;
    request.send_with_opt_u8_array(body).map_err(failed)?;
    let bytes = js_sys::Uint8Array::new(&request.response().map_err(failed)?).to_vec();
    Ok((request.status().map_err(failed)?, bytes))
}

#[cfg(target_family = "wasm")]
pub fn get(url: &str, headers: &[(&str, &str)], timeout: Duration) -> Result<Vec<u8>, String> {
    match exchange("GET", url, headers, None, timeout)? {
        (status, bytes) if (200..300).contains(&status) => Ok(bytes),
        (status, _) => Err(format!("http status: {status}")),
    }
}

#[cfg(target_family = "wasm")]
pub fn post(url: &str, content_type: &str, body: &[u8], timeout: Duration) -> Result<(u16, Vec<u8>), String> {
    exchange("POST", url, &[("Content-Type", content_type)], Some(body), timeout)
}

pub fn get_text(url: &str, timeout: Duration) -> Result<String, String> {
    get(url, &[], timeout).and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
}
