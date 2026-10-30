use std::time::Duration;

#[cfg(not(target_family = "wasm"))]
static AGENT: std::sync::LazyLock<ureq::Agent> =
    std::sync::LazyLock::new(ureq::Agent::new_with_defaults);

#[cfg(not(target_family = "wasm"))]
pub fn get(url: &str, headers: &[(&str, &str)], timeout: Duration) -> Result<Vec<u8>, String> {
    let asked = headers.iter().fold(AGENT.get(url), |asked, (name, value)| {
        asked.header(*name, *value)
    });
    asked
        .config()
        .timeout_global(Some(timeout))
        .build()
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_vec()
        .map_err(|e| e.to_string())
}

#[cfg(not(target_family = "wasm"))]
pub fn stream(url: &str, timeout: Duration) -> Result<(Option<u64>, impl std::io::Read), String> {
    let body = AGENT
        .get(url)
        .config()
        .timeout_global(Some(timeout))
        .build()
        .call()
        .map_err(|e| e.to_string())?
        .into_body();
    Ok((body.content_length(), body.into_with_config().reader()))
}

#[cfg(not(target_family = "wasm"))]
pub fn post(
    url: &str,
    content_type: &str,
    body: &[u8],
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let mut answer = AGENT
        .post(url)
        .header("Content-Type", content_type)
        .config()
        .http_status_as_error(false)
        .timeout_global(Some(timeout))
        .build()
        .send(body)
        .map_err(|e| e.to_string())?;
    let status = answer.status().as_u16();
    answer
        .body_mut()
        .read_to_vec()
        .map(|bytes| (status, bytes))
        .map_err(|e| e.to_string())
}

#[cfg(not(target_family = "wasm"))]
pub fn send(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let answer = match method {
        "PUT" => headers
            .iter()
            .fold(AGENT.put(url), |asked, (name, value)| {
                asked.header(*name, *value)
            })
            .config()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .send(body.unwrap_or_default()),
        "POST" => headers
            .iter()
            .fold(AGENT.post(url), |asked, (name, value)| {
                asked.header(*name, *value)
            })
            .config()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .send(body.unwrap_or_default()),
        "DELETE" => headers
            .iter()
            .fold(AGENT.delete(url), |asked, (name, value)| {
                asked.header(*name, *value)
            })
            .config()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .call(),
        "HEAD" => headers
            .iter()
            .fold(AGENT.head(url), |asked, (name, value)| {
                asked.header(*name, *value)
            })
            .config()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .call(),
        _ => headers
            .iter()
            .fold(AGENT.get(url), |asked, (name, value)| {
                asked.header(*name, *value)
            })
            .config()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .call(),
    };
    let mut answer = answer.map_err(|e| e.to_string())?;
    let status = answer.status().as_u16();
    answer
        .body_mut()
        .read_to_vec()
        .map(|bytes| (status, bytes))
        .map_err(|e| e.to_string())
}

#[cfg(target_family = "wasm")]
enum Failure {
    Unreachable(String),
    Refused(String),
}

#[cfg(target_family = "wasm")]
impl Failure {
    fn text(self) -> String {
        match self {
            Failure::Unreachable(text) | Failure::Refused(text) => text,
        }
    }
}

#[cfg(target_family = "wasm")]
fn thrown(error: &wasm_bindgen::JsValue) -> (String, String) {
    match wasm_bindgen::JsCast::dyn_ref::<js_sys::Error>(error) {
        Some(thrown) => (String::from(thrown.name()), String::from(thrown.message())),
        None => (
            String::new(),
            error.as_string().unwrap_or_else(|| format!("{error:?}")),
        ),
    }
}

#[cfg(target_family = "wasm")]
fn exchange_once(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), Failure> {
    let host = url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string());
    let failed = |error: wasm_bindgen::JsValue| match thrown(&error) {
        (name, _) if name == "NetworkError" => {
            Failure::Unreachable(format!("No answer from {host}"))
        }
        (_, message) => Failure::Refused(message),
    };
    let request = web_sys::XmlHttpRequest::new().map_err(failed)?;
    request
        .open_with_async(method, url, false)
        .map_err(failed)?;
    request.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);
    request.set_timeout(u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX));
    headers
        .iter()
        .try_for_each(|(name, value)| request.set_request_header(name, value))
        .map_err(failed)?;
    request.send_with_opt_u8_array(body).map_err(failed)?;
    let bytes = js_sys::Uint8Array::new(&request.response().map_err(failed)?).to_vec();
    Ok((request.status().map_err(failed)?, bytes))
}

#[cfg(any(target_family = "wasm", test))]
static PROXIED: std::sync::Mutex<std::collections::BTreeSet<String>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

#[cfg(any(target_family = "wasm", test))]
fn origin(url: &str) -> String {
    url::Url::parse(url)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

#[cfg(any(target_family = "wasm", test))]
fn proxied() -> std::sync::MutexGuard<'static, std::collections::BTreeSet<String>> {
    PROXIED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(any(target_family = "wasm", test))]
#[derive(Debug, PartialEq, Eq)]
enum Route {
    Direct,
    DirectThenDevice(String),
    Device(String),
}

#[cfg(any(target_family = "wasm", test))]
fn route(url: &str) -> Route {
    match crate::device::proxy().filter(|_| crate::device::foreign(url)) {
        None => Route::Direct,
        Some(proxy) if proxied().contains(&origin(url)) => Route::Device(proxy),
        Some(proxy) => Route::DirectThenDevice(proxy),
    }
}

#[cfg(target_family = "wasm")]
fn exchange(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let through = |proxy: &str| {
        exchange_once(
            method,
            proxy,
            &[headers, &[(crate::device::PROXY_TARGET, url)]].concat(),
            body,
            timeout,
        )
        .map_err(Failure::text)
    };
    match route(url) {
        Route::Device(proxy) => through(&proxy),
        Route::Direct => exchange_once(method, url, headers, body, timeout).map_err(Failure::text),
        Route::DirectThenDevice(proxy) => {
            match exchange_once(method, url, headers, body, timeout) {
                Err(Failure::Unreachable(_)) => {
                    proxied().insert(origin(url));
                    through(&proxy)
                }
                other => other.map_err(Failure::text),
            }
        }
    }
}

#[cfg(target_family = "wasm")]
pub fn send(
    method: &str,
    url: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    exchange(method, url, headers, body, timeout)
}

#[cfg(target_family = "wasm")]
pub fn get(url: &str, headers: &[(&str, &str)], timeout: Duration) -> Result<Vec<u8>, String> {
    match exchange("GET", url, headers, None, timeout)? {
        (status, bytes) if (200..300).contains(&status) => Ok(bytes),
        (status, _) => Err(format!("http status: {status}")),
    }
}

#[cfg(target_family = "wasm")]
async fn fetch_once(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), Failure> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let host = url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .unwrap_or_else(|| url.to_string());
    let failed = |error: wasm_bindgen::JsValue| match thrown(&error) {
        (name, _) if ["TypeError", "TimeoutError", "AbortError"].contains(&name.as_str()) => {
            Failure::Unreachable(format!("No answer from {host}"))
        }
        (_, message) => Failure::Refused(format!("{host}: {message}")),
    };
    let sent = web_sys::Headers::new().map_err(failed)?;
    headers
        .iter()
        .try_for_each(|(name, value)| sent.append(name, value))
        .map_err(failed)?;
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    init.set_headers(&sent);
    if let Some(body) = body {
        init.set_body(&js_sys::Uint8Array::from(body));
    }
    init.set_signal(Some(&web_sys::AbortSignal::timeout_with_u32(
        u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX),
    )));
    let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
    let response: web_sys::Response = JsFuture::from(scope.fetch_with_str_and_init(url, &init))
        .await
        .map_err(failed)?
        .unchecked_into();
    let answer = JsFuture::from(response.array_buffer().map_err(failed)?)
        .await
        .map_err(failed)?;
    Ok((response.status(), js_sys::Uint8Array::new(&answer).to_vec()))
}

#[cfg(target_family = "wasm")]
pub async fn send_later(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    let target = [
        headers,
        &[(crate::device::PROXY_TARGET.to_string(), url.to_string())],
    ]
    .concat();
    match route(url) {
        Route::Device(proxy) => fetch_once(method, &proxy, &target, body, timeout)
            .await
            .map_err(Failure::text),
        Route::Direct => fetch_once(method, url, headers, body, timeout)
            .await
            .map_err(Failure::text),
        Route::DirectThenDevice(proxy) => {
            match fetch_once(method, url, headers, body, timeout).await {
                Err(Failure::Unreachable(_)) => {
                    proxied().insert(origin(url));
                    fetch_once(method, &proxy, &target, body, timeout)
                        .await
                        .map_err(Failure::text)
                }
                other => other.map_err(Failure::text),
            }
        }
    }
}

#[cfg(target_family = "wasm")]
pub async fn get_later(
    url: &str,
    headers: &[(String, String)],
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    match send_later("GET", url, headers, None, timeout).await? {
        (status, bytes) if (200..300).contains(&status) => Ok(bytes),
        (status, _) => Err(format!("http status: {status} from {url}")),
    }
}

#[cfg(target_family = "wasm")]
pub async fn stream_lines(
    url: &str,
    limit: usize,
    mut line: impl FnMut(&str) -> bool,
) -> Result<usize, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let failed = |error: wasm_bindgen::JsValue| thrown(&error).1;
    let through = crate::device::proxy().filter(|_| crate::device::foreign(url));
    let headers = web_sys::Headers::new().map_err(failed)?;
    if through.is_some() {
        headers
            .append(crate::device::PROXY_TARGET, url)
            .map_err(failed)?;
    }
    let init = web_sys::RequestInit::new();
    init.set_headers(&headers);
    let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
    let response: web_sys::Response =
        JsFuture::from(scope.fetch_with_str_and_init(through.as_deref().unwrap_or(url), &init))
            .await
            .map_err(failed)?
            .unchecked_into();
    if !response.ok() {
        return Err(format!("http status: {}", response.status()));
    }
    let reader: web_sys::ReadableStreamDefaultReader = response
        .body()
        .ok_or("the answer has no body")?
        .get_reader()
        .unchecked_into();
    let mut pending = Vec::new();
    let mut lines = 0;
    loop {
        let chunk = JsFuture::from(reader.read()).await.map_err(failed)?;
        if js_sys::Reflect::get(&chunk, &"done".into())
            .ok()
            .and_then(|done| done.as_bool())
            .unwrap_or(true)
        {
            return Ok(lines);
        }
        let bytes = js_sys::Reflect::get(&chunk, &"value".into())
            .map(|value| js_sys::Uint8Array::new(&value).to_vec())
            .unwrap_or_default();
        for text in crate::hostsocket::lines(&mut pending, &bytes, limit) {
            lines += 1;
            if !line(&text) {
                let _ = reader.cancel();
                return Ok(lines);
            }
        }
    }
}

#[cfg(target_family = "wasm")]
pub fn post(
    url: &str,
    content_type: &str,
    body: &[u8],
    timeout: Duration,
) -> Result<(u16, Vec<u8>), String> {
    exchange(
        "POST",
        url,
        &[("Content-Type", content_type)],
        Some(body),
        timeout,
    )
}

pub fn get_text(url: &str, timeout: Duration) -> Result<String, String> {
    get(url, &[], timeout).and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paired_browser_goes_through_the_device_only_for_hosts_it_cannot_reach_itself() {
        let _pairing = crate::device::TEST_PAIRING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::device::unpair();
        assert_eq!(
            route("https://tile.openstreetmap.org/1/0/0.png"),
            Route::Direct,
            "without a device there is nothing to go through"
        );
        crate::device::pair("http://falcon.local/fly/").unwrap();
        assert_eq!(
            route("http://falcon.local/fly/files/Aircast/a.plan"),
            Route::Direct,
            "the device's own files are already same-origin"
        );
        assert_eq!(
            route("https://logs.px4.io/upload"),
            Route::DirectThenDevice("http://falcon.local/fly/proxy".into())
        );
        proxied().insert(origin("https://logs.px4.io/upload"));
        assert_eq!(
            route("https://logs.px4.io/browse"),
            Route::Device("http://falcon.local/fly/proxy".into()),
            "a host that failed once goes straight through the device"
        );
        assert_eq!(
            route("https://firmware.ardupilot.org/x"),
            Route::DirectThenDevice("http://falcon.local/fly/proxy".into()),
            "other hosts still try directly first"
        );
        proxied().clear();
        crate::device::unpair();
    }
}
