use std::cell::{Cell, RefCell};

use serde_json::{Value, json};

use crate::linkconfig::{Kind, LinkConfig};
use crate::router::Backend;

pub const BRIDGE_WRITE_GUARD: &str = "QGC_DEBUG_API_ALLOW_ACTUATORS";
pub const AUTOPILOT_PX4: i64 = 12;
pub const AUTOPILOT_ARDUPILOTMEGA: i64 = 3;
pub const VEHICLE_TYPE_QUADROTOR: i64 = 2;
pub const MOCK_LINK_NAME: &str = "MockLink";
pub const WHOLE_PLAN: &str = "all";
pub const STATUS_OK: u16 = 200;
pub const STATUS_BAD_REQUEST: u16 = 400;
pub const STATUS_NOT_FOUND: u16 = 404;
pub const STATUS_UNAVAILABLE: u16 = 503;

pub const HOST_ROUTES: &[(&str, &str)] = &[
    ("/native/windows", "reads AppKit window geometry"),
    ("/native/menu", "reads the AppKit menu bar"),
    ("/native/menu/invoke", "clicks an AppKit menu item"),
    ("/native/bridge", "reads the native head's own bridge counters"),
    ("/native/probe", "probes a native accessibility element"),
    ("/native/type", "types into a native window"),
    ("/native/click", "clicks a point in a native window"),
    ("/grab", "captures a window image"),
    ("/screenshot", "captures a window image"),
    ("/status", "reads the head's VideoManager and window layout"),
    ("/switch", "switches the head's active video source"),
    ("/record", "starts and stops the head's video recording"),
    ("/video/setting", "reads and writes VideoSettings facts"),
    ("/vehicle", "reads the head's active Vehicle object"),
    ("/vehicle/params", "reads the head's ParameterManager"),
    ("/vehicle/params/set", "writes a parameter through the head's ParameterManager"),
    ("/vehicle/params/save", "writes the head's parameter file"),
    ("/vehicle/params/load", "loads the head's parameter file"),
    ("/vehicle/calibrate", "runs a head-side calibration"),
    ("/vehicle/motortest", "spins a motor through the head"),
    ("/vehicle/messages", "reads the head's message log"),
    ("/vehicle/rc", "reads the head's RC channel values"),
    ("/ui/tree", "walks the QQuick scene graph"),
    ("/ui/click", "synthesises a mouse event into a window"),
    ("/ui/doubleclick", "synthesises a mouse event into a window"),
    ("/ui/drag", "synthesises a mouse event into a window"),
    ("/ui/hover", "synthesises a mouse event into a window"),
    ("/ui/type", "synthesises a key event into a window"),
    ("/ui/key", "synthesises a key event into a window"),
    ("/ui/press", "synthesises a mouse event into a window"),
    ("/ui/move", "synthesises a mouse event into a window"),
    ("/ui/release", "synthesises a mouse event into a window"),
    ("/ui/tap", "synthesises a touch event into a window"),
    ("/ui/pinch", "synthesises a touch event into a window"),
    ("/ui/dismiss", "closes QQuick popups"),
    ("/ui/prop", "reads a QQuick item property"),
    ("/ui/setprop", "writes a QQuick item property"),
    ("/ui/at", "hit-tests a scene coordinate"),
    ("/ui/resize", "resizes a window"),
    ("/ui/watch", "streams one sample per animation tick"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    BridgeGet,
    BridgeSet,
    BridgeInvoke,
    Links,
    LinkConnect,
    LinkDisconnect,
    MockLink,
    MissionUpload,
    MissionDownload,
    Logging,
}

pub const ROUTES: &[(&str, Route, &str)] = &[
    ("/bridge/get", Route::BridgeGet, "path"),
    ("/bridge/set", Route::BridgeSet, "path,value"),
    ("/bridge/invoke", Route::BridgeInvoke, "path,args"),
    ("/links", Route::Links, ""),
    ("/links/connect", Route::LinkConnect, "host,port,name"),
    ("/links/disconnect", Route::LinkDisconnect, "name"),
    ("/links/mocklink", Route::MockLink, "autopilot,add"),
    ("/mission/upload", Route::MissionUpload, "file"),
    ("/mission/download", Route::MissionDownload, "file"),
    ("/logging", Route::Logging, "rules"),
];

pub fn route_of(path: &str) -> Option<Route> {
    ROUTES.iter().find(|(known, _, _)| *known == path).map(|(_, route, _)| *route)
}

pub fn host_route(path: &str) -> Option<&'static str> {
    HOST_ROUTES.iter().find(|(known, _)| *known == path).map(|(_, why)| *why)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16,
    pub body: Value,
}

impl Response {
    pub fn body_text(&self) -> String {
        self.body.to_string()
    }
}

fn ok(body: Value) -> Response {
    Response { status: STATUS_OK, body }
}

fn refuse(reason: impl Into<String>) -> Response {
    Response { status: STATUS_BAD_REQUEST, body: json!({ "error": reason.into() }) }
}

fn later(reason: impl Into<String>) -> Response {
    Response { status: STATUS_UNAVAILABLE, body: json!({ "error": reason.into(), "retry": true }) }
}

fn missing() -> Response {
    Response { status: STATUS_NOT_FOUND, body: json!({ "error": "not found" }) }
}

fn host_owned(path: &str, why: &str) -> Response {
    Response { status: STATUS_NOT_FOUND, body: json!({ "error": format!("{path} is served by the host, not the core: {why}"), "hostOwned": true }) }
}

pub trait Host {
    fn bridge_get(&self, path: &str) -> String;
    fn bridge_set(&self, path: &str, payload_json: &str) -> String;
    fn bridge_invoke(&self, path: &str, args_json: &str) -> String;
    fn link_configurations(&self) -> Value;
    fn link_open(&self, config_json: &str) -> Result<u32, String>;
    fn link_close(&self, name: Option<&str>) -> Result<usize, String>;
    fn mission(&self, request_json: &str) -> Result<(), String>;
    fn set_logging_rules(&self, rules: &str);
    fn bridge_writes_allowed(&self) -> bool;
    fn mock_links_available(&self) -> bool;
    fn mock_link_present(&self) -> bool;
    fn vehicle_connected(&self) -> bool;
}

pub fn query_pairs(query: &str) -> Vec<(String, String)> {
    url::form_urlencoded::parse(query.trim_start_matches('?').as_bytes()).map(|(k, v)| (k.into_owned(), v.into_owned())).collect()
}

fn first<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn given<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    first(pairs, key).filter(|v| !v.is_empty())
}

fn present(pairs: &[(String, String)], key: &str) -> bool {
    pairs.iter().any(|(k, _)| k == key)
}

fn bridge_body(answer: String) -> Value {
    serde_json::from_str(&answer).unwrap_or_else(|_| json!({ "raw": answer }))
}

fn literal(raw: &str) -> Value {
    match raw.is_empty() {
        true => Value::Null,
        false => serde_json::from_str::<Value>(&format!("[{raw}]")).ok().and_then(|v| v.as_array().and_then(|a| a.first().cloned())).unwrap_or_else(|| Value::String(raw.to_string())),
    }
}

fn configured_links(host: &dyn Host) -> Vec<Value> {
    host.link_configurations()
        .get("elements")
        .and_then(Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .enumerate()
                .map(|(i, element)| {
                    let link = crate::links::link_json(i, element);
                    json!({
                        "index": i,
                        "name": link["name"].clone(),
                        "type": link["type"].clone(),
                        "connected": link["connected"].clone(),
                        "heardVehicle": link["heardVehicle"].clone(),
                        "lastError": link["lastError"].clone(),
                        "host": link["host"].clone(),
                        "port": link["port"].clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn named_link(links: &[Value], name: &str) -> Option<Value> {
    links.iter().find(|link| link["name"].as_str() == Some(name)).cloned()
}

fn plan_summary(text: &str) -> Result<Value, String> {
    let root: Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    match root.get("fileType").and_then(Value::as_str) {
        Some("Plan") => crate::planfile::parse(text).map(|plan| {
            json!({
                "itemCount": plan.items.len(),
                "complexItems": plan.items.iter().filter(|item| item["type"] != "SimpleItem").count(),
                "fencePolygons": plan.fence_polygons,
                "fenceCircles": plan.fence_circles,
                "rallyPoints": plan.rally_points,
            })
        }),
        _ => crate::mission::parse(text).map(|mission| json!({ "itemCount": mission.items.len(), "complexItems": 0, "fencePolygons": 0, "fenceCircles": 0, "rallyPoints": 0 })),
    }
}

fn plan_is_empty(summary: &Value) -> bool {
    ["itemCount", "fencePolygons", "fenceCircles", "rallyPoints"].iter().all(|key| summary[key] == json!(0))
}

fn mission_file(host: &dyn Host, pairs: &[(String, String)]) -> Result<String, Response> {
    match (host.vehicle_connected(), given(pairs, "file")) {
        (false, _) => Err(refuse("no vehicle connected")),
        (true, None) => Err(refuse("file required")),
        (true, Some(file)) => Ok(file.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    Idle,
    Waiting,
    Write(String),
    Empty(String),
}

struct BusyGuard<'a>(&'a Cell<bool>);

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

#[derive(Default)]
pub struct DebugApi {
    busy: Cell<bool>,
    transfers: Cell<u64>,
    pending_download: RefCell<Option<(u64, String)>>,
}

impl DebugApi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pending_download(&self) -> Option<String> {
        self.pending_download.borrow().clone().map(|(_, file)| file)
    }

    pub fn dispatch(&self, host: &dyn Host, method: &str, path: &str, query: &str, body: &str) -> Response {
        if self.busy.get() {
            return later("busy: another request is still being processed");
        }
        self.busy.set(true);
        let _guard = BusyGuard(&self.busy);
        if method != "GET" {
            return refuse("bad request");
        }
        let pairs = query_pairs(query);
        match (route_of(path), host_route(path), path.starts_with("/bridge/")) {
            (Some(route), _, _) => self.handle(host, route, &pairs, body),
            (None, Some(why), _) => host_owned(path, why),
            (None, None, true) => refuse(format!("unknown bridge route: {path}")),
            (None, None, false) => missing(),
        }
    }

    fn handle(&self, host: &dyn Host, route: Route, pairs: &[(String, String)], body: &str) -> Response {
        match route {
            Route::BridgeGet | Route::BridgeSet | Route::BridgeInvoke => self.bridge(host, route, pairs, body),
            Route::Links => ok(json!({ "links": configured_links(host) })),
            Route::LinkConnect => self.link_connect(host, pairs),
            Route::LinkDisconnect => self.link_disconnect(host, pairs),
            Route::MockLink => self.mock_link(host, pairs),
            Route::MissionUpload => self.mission_upload(host, pairs),
            Route::MissionDownload => self.mission_download(host, pairs),
            Route::Logging => self.logging(host, pairs),
        }
    }

    fn bridge(&self, host: &dyn Host, route: Route, pairs: &[(String, String)], body: &str) -> Response {
        let Some(target) = given(pairs, "path") else {
            return refuse("path is required, e.g. path=settings.appSettings");
        };
        if route == Route::BridgeGet {
            return ok(bridge_body(host.bridge_get(target)));
        }
        if !host.bridge_writes_allowed() {
            return refuse(format!("bridge writes disabled; set {BRIDGE_WRITE_GUARD}=1 (props off!)"));
        }
        match route {
            Route::BridgeSet => match first(pairs, "value").map(str::to_string).or_else(|| (!body.is_empty()).then(|| body.to_string())) {
                None => refuse("value is required"),
                Some(raw) => ok(bridge_body(host.bridge_set(target, &json!({ "value": literal(&raw) }).to_string()))),
            },
            _ => {
                let args = given(pairs, "args").map(str::to_string).or_else(|| (!body.is_empty()).then(|| body.to_string())).unwrap_or_else(|| "[]".to_string());
                ok(bridge_body(host.bridge_invoke(target, &args)))
            }
        }
    }

    fn link_connect(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        let Some(target) = given(pairs, "host") else {
            return refuse("host required");
        };
        let Some(raw_port) = given(pairs, "port") else {
            return refuse("port required, e.g. port=5760");
        };
        let Some(port) = raw_port.parse::<u16>().ok().filter(|p| *p != 0) else {
            return refuse(format!("port must be 1-65535, got: {raw_port}"));
        };
        let name = given(pairs, "name").map(str::to_string).unwrap_or_else(|| format!("debug-api {target}:{port}"));
        let existing = named_link(&configured_links(host), &name);
        if let Some(link) = &existing {
            if link["type"] != json!("tcp") {
                return refuse(format!("link {name} exists but is not tcp"));
            }
            let (live_host, live_port) = (link["host"].as_str().unwrap_or(""), link["port"].as_u64().unwrap_or(0));
            if link["connected"] == json!(true) {
                return match (live_host == target, live_port == u64::from(port)) {
                    (true, true) => ok(json!({ "name": name, "host": live_host, "port": live_port, "connected": true, "existing": true, "id": Value::Null })),
                    _ => refuse(format!("link {name} is already connected to {live_host}:{live_port}; disconnect it first")),
                };
            }
        }
        let config = LinkConfig { name: name.clone(), auto_connect: false, high_latency: false, kind: Kind::Tcp { host: target.to_string(), port } };
        let mut request = crate::linkconfig::to_json(&config);
        request["viaLinkManager"] = json!(true);
        match host.link_open(&request.to_string()) {
            Ok(id) => ok(json!({ "name": name, "host": target, "port": port, "connected": true, "existing": existing.is_some(), "id": id })),
            Err(reason) => refuse(reason),
        }
    }

    fn link_disconnect(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        let Some(name) = given(pairs, "name") else {
            return match host.link_close(None) {
                Ok(count) => ok(json!({ "disconnected": "all", "count": count })),
                Err(reason) => refuse(reason),
            };
        };
        let connected = named_link(&configured_links(host), name).is_some_and(|link| link["connected"] == json!(true));
        if !connected {
            return refuse(format!("no connected link named {name}"));
        }
        match host.link_close(Some(name)) {
            Ok(count) => ok(json!({ "disconnected": name, "count": count })),
            Err(reason) => refuse(reason),
        }
    }

    fn mock_link(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        if !host.mock_links_available() {
            return refuse("mock links exist only in debug builds");
        }
        let Some(autopilot) = given(pairs, "autopilot").map(str::to_lowercase) else {
            return refuse("autopilot must be px4 or apm");
        };
        if !present(pairs, "add") && host.mock_link_present() {
            return ok(json!({ "started": false, "existing": true, "autopilot": autopilot, "id": Value::Null }));
        }
        let firmware = match autopilot.as_str() {
            "px4" => AUTOPILOT_PX4,
            "apm" | "arducopter" => AUTOPILOT_ARDUPILOTMEGA,
            other => return refuse(format!("autopilot must be px4 or apm, got: {other}")),
        };
        let config = LinkConfig {
            name: MOCK_LINK_NAME.to_string(),
            auto_connect: false,
            high_latency: false,
            kind: Kind::Mock { firmware_type: firmware, vehicle_type: VEHICLE_TYPE_QUADROTOR, send_status_text: false, increment_vehicle_id: true, failure_mode: 0 },
        };
        let mut request = crate::linkconfig::to_json(&config);
        request["dynamic"] = json!(true);
        request["viaLinkManager"] = json!(true);
        match host.link_open(&request.to_string()) {
            Ok(id) => ok(json!({ "started": true, "existing": false, "autopilot": autopilot, "id": id })),
            Err(reason) => refuse(format!("could not start mock link: {reason}")),
        }
    }

    fn mission_upload(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        let file = match mission_file(host, pairs) {
            Ok(file) => file,
            Err(response) => return response,
        };
        *self.pending_download.borrow_mut() = None;
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(reason) => return refuse(format!("cannot read {file}: {reason}")),
        };
        let summary = match plan_summary(&text) {
            Ok(summary) => summary,
            Err(reason) => return refuse(reason),
        };
        if plan_is_empty(&summary) {
            return refuse(format!("nothing to upload in {file}: no mission items, no fence and no rally points"));
        }
        match host.mission(&json!({ "action": "write", "plan": WHOLE_PLAN, "file": file }).to_string()) {
            Ok(()) => {
                let mut body = summary;
                body["uploading"] = json!(file);
                ok(body)
            }
            Err(reason) => refuse(reason),
        }
    }

    fn mission_download(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        let file = match mission_file(host, pairs) {
            Ok(file) => file,
            Err(response) => return response,
        };
        let transfer = self.transfers.get() + 1;
        self.transfers.set(transfer);
        *self.pending_download.borrow_mut() = Some((transfer, file.clone()));
        match host.mission(&json!({ "action": "load", "plan": WHOLE_PLAN }).to_string()) {
            Ok(()) => ok(json!({ "downloading": file, "transfer": transfer })),
            Err(reason) => {
                *self.pending_download.borrow_mut() = None;
                refuse(reason)
            }
        }
    }

    pub fn settle_download(&self, transfer: u64, in_progress: bool, item_count: usize) -> Settled {
        let Some((armed, file)) = self.pending_download.borrow().clone() else { return Settled::Idle };
        if armed != transfer {
            return Settled::Idle;
        }
        if in_progress {
            return Settled::Waiting;
        }
        *self.pending_download.borrow_mut() = None;
        match item_count {
            0 => Settled::Empty(file),
            _ => Settled::Write(file),
        }
    }

    fn logging(&self, host: &dyn Host, pairs: &[(String, String)]) -> Response {
        let Some(rules) = given(pairs, "rules") else {
            return refuse("rules required, e.g. qgc.videomanager.*.debug=true");
        };
        host.set_logging_rules(&rules.replace(';', "\n"));
        ok(json!({ "rules": rules }))
    }
}

pub const DEPS: &[&str] = &[];

pub fn debug_api_view(_backend: &dyn Backend, args: &[String]) -> Value {
    let table = json!({
        "kind": "object",
        "class": "DebugApiRoutes",
        "count": ROUTES.len(),
        "routes": ROUTES.iter().map(|(path, _, keys)| json!({ "method": "GET", "path": path, "query": keys })).collect::<Vec<_>>(),
        "hostRoutes": HOST_ROUTES.iter().map(|(path, why)| json!({ "path": path, "reason": why })).collect::<Vec<_>>(),
    });
    let Some(method) = args.first().filter(|m| !m.is_empty()) else {
        return table;
    };
    let Some(path) = args.get(1).filter(|p| !p.is_empty()) else {
        return crate::read::refused("this needs a method and a path to resolve, e.g. GET,/links/connect");
    };
    let pairs = query_pairs(args.get(2).map(String::as_str).unwrap_or(""));
    json!({
        "kind": "object",
        "class": "DebugApiRoute",
        "method": method,
        "path": path,
        "accepted": method == "GET" && route_of(path).is_some(),
        "route": route_of(path).map(|route| format!("{route:?}")),
        "hostOwned": host_route(path),
        "status": match (method == "GET", route_of(path).is_some()) {
            (false, _) => STATUS_BAD_REQUEST,
            (true, true) => STATUS_OK,
            (true, false) => STATUS_NOT_FOUND,
        },
        "query": pairs.iter().map(|(k, v)| json!({ "key": k, "value": v })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Fake {
        calls: Mutex<Vec<String>>,
        links: Value,
        writes_allowed: bool,
        mock_available: bool,
        mock_present: bool,
        vehicle: bool,
        open: Option<Result<u32, String>>,
        mission: Option<Result<(), String>>,
    }

    impl Fake {
        fn note(&self, call: String) {
            self.calls.lock().unwrap().push(call);
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl Host for Fake {
        fn bridge_get(&self, path: &str) -> String {
            self.note(format!("get {path}"));
            json!({ "kind": "value", "value": 7 }).to_string()
        }
        fn bridge_set(&self, path: &str, payload_json: &str) -> String {
            self.note(format!("set {path} {payload_json}"));
            json!({ "ok": true }).to_string()
        }
        fn bridge_invoke(&self, path: &str, args_json: &str) -> String {
            self.note(format!("invoke {path} {args_json}"));
            json!({ "ok": true }).to_string()
        }
        fn link_configurations(&self) -> Value {
            self.links.clone()
        }
        fn link_open(&self, config_json: &str) -> Result<u32, String> {
            self.note(format!("open {config_json}"));
            self.open.clone().unwrap_or(Ok(4))
        }
        fn link_close(&self, name: Option<&str>) -> Result<usize, String> {
            self.note(format!("close {name:?}"));
            Ok(name.map(|_| 1).unwrap_or(3))
        }
        fn mission(&self, request_json: &str) -> Result<(), String> {
            self.note(format!("mission {request_json}"));
            self.mission.clone().unwrap_or(Ok(()))
        }
        fn set_logging_rules(&self, rules: &str) {
            self.note(format!("logging {rules}"));
        }
        fn bridge_writes_allowed(&self) -> bool {
            self.writes_allowed
        }
        fn mock_links_available(&self) -> bool {
            self.mock_available
        }
        fn mock_link_present(&self) -> bool {
            self.mock_present
        }
        fn vehicle_connected(&self) -> bool {
            self.vehicle
        }
    }

    fn tcp(name: &str, connected: bool, host: &str, port: u16, heard: bool, last_error: &str) -> Value {
        json!({
            "name": name,
            "settingsURL": "TcpSettings.qml",
            "host": host,
            "port": port,
            "heardVehicle": heard,
            "lastError": last_error,
            "children": match connected {
                true => vec!["link"],
                false => Vec::new(),
            },
        })
    }

    fn model(elements: &[Value]) -> Value {
        json!({ "elements": elements })
    }

    fn plain(links: &[(&str, bool)]) -> Value {
        model(&links.iter().map(|(name, connected)| tcp(name, *connected, "127.0.0.1", 5760, *connected, "")).collect::<Vec<_>>())
    }

    fn get(api: &DebugApi, host: &Fake, path: &str, query: &str) -> Response {
        api.dispatch(host, "GET", path, query, "")
    }

    #[test]
    fn an_unknown_path_is_missing_and_a_host_route_names_its_owner() {
        let api = DebugApi::new();
        let host = Fake::default();
        assert_eq!(get(&api, &host, "/nowhere", "").status, STATUS_NOT_FOUND, "a path the core does not serve answers 404, never a silent 200");
        assert_eq!(get(&api, &host, "/nowhere", "").body["error"], "not found");
        let native = get(&api, &host, "/native/windows", "");
        assert_eq!(native.status, STATUS_NOT_FOUND, "native routes are the host's and the core refuses them");
        assert_eq!(
            (native.body["error"].clone(), native.body["hostOwned"].clone()),
            (json!("/native/windows is served by the host, not the core: reads AppKit window geometry"), json!(true)),
            "a bare \"not found\" cannot be told apart from a typo, and the two have opposite next steps: fix the URL versus wire the host"
        );
        let grab = get(&api, &host, "/grab", "");
        assert!(grab.body["error"].as_str().unwrap().contains("served by the host"), "every path in HOST_ROUTES answers with its owner, not only the native ones");
        assert!(host_route("/status").is_some() && host_route("/vehicle/params").is_some() && host_route("/video/setting").is_some(), "the routes the port left with the head are still documented, or a head author reads the view and concludes they were deleted");
        assert_eq!(host_route("/grab"), Some("captures a window image"), "the split is documented so a head knows which side owns a route");
        let bridge = get(&api, &host, "/bridge/nonsense", "path=x");
        assert_eq!(bridge.status, STATUS_BAD_REQUEST);
        assert_eq!(bridge.body["error"], "unknown bridge route: /bridge/nonsense");
        assert_eq!(api.dispatch(&host, "POST", "/links", "", "").body["error"], "bad request", "only GET is served, exactly as the C++ answered");
    }

    #[test]
    fn a_query_value_is_fully_decoded_including_an_encoded_slash() {
        let pairs = query_pairs("?path=%2Ftmp%2Fout.json&value=a+b&empty=");
        assert_eq!(first(&pairs, "path"), Some("/tmp/out.json"), "an encoded slash has to survive decoding or the app writes a file by the literal escaped name");
        assert_eq!(first(&pairs, "value"), Some("a b"));
        assert_eq!(first(&pairs, "empty"), Some(""), "a key given with no value is present");
        assert_eq!(given(&pairs, "empty"), None, "but an empty value is not a given one");
        assert!(present(&pairs, "empty") && !present(&pairs, "absent"), "presence and value are separate questions");
    }

    #[test]
    fn bridge_get_proxies_and_writes_stay_shut_until_the_guard_is_lifted() {
        let api = DebugApi::new();
        let host = Fake::default();
        assert_eq!(get(&api, &host, "/bridge/get", "").body["error"], "path is required, e.g. path=settings.appSettings");
        let read = get(&api, &host, "/bridge/get", "path=settings.appSettings");
        assert_eq!((read.status, read.body["value"].clone()), (STATUS_OK, json!(7)), "a read is a straight proxy over the ABI the core already answers");
        let refused = get(&api, &host, "/bridge/set", "path=x&value=1");
        assert_eq!(refused.status, STATUS_BAD_REQUEST);
        assert!(refused.body["error"].as_str().unwrap().contains(BRIDGE_WRITE_GUARD), "a write is refused by name so the caller knows which guard to lift");
        assert_eq!(host.calls(), vec!["get settings.appSettings".to_string()], "a refused write never reaches the bridge");
    }

    #[test]
    fn a_permitted_bridge_write_wraps_a_json_literal_and_an_empty_value_clears_the_fact() {
        let api = DebugApi::new();
        let host = Fake { writes_allowed: true, ..Fake::default() };
        assert_eq!(get(&api, &host, "/bridge/set", "path=x").body["error"], "value is required");
        get(&api, &host, "/bridge/set", "path=a.b&value=1.5");
        get(&api, &host, "/bridge/set", "path=a.b&value=hello");
        get(&api, &host, "/bridge/set", "path=a.b&value=");
        get(&api, &host, "/bridge/invoke", "path=a.b");
        get(&api, &host, "/bridge/invoke", "path=a.b&args=%5B1%2C2%5D");
        assert_eq!(
            host.calls(),
            vec![
                "set a.b {\"value\":1.5}".to_string(),
                "set a.b {\"value\":\"hello\"}".to_string(),
                "set a.b {\"value\":null}".to_string(),
                "invoke a.b []".to_string(),
                "invoke a.b [1,2]".to_string(),
            ],
            "a number stays a number, bare text becomes a string, an empty value is the null the C++ sent rather than an empty string that retypes the Fact, and an absent args list is the empty list"
        );
    }

    #[test]
    fn the_links_listing_says_whether_the_vehicle_was_heard_and_where_each_link_points() {
        let api = DebugApi::new();
        let host = Fake {
            links: model(&[tcp("SITL", true, "127.0.0.1", 5760, true, ""), tcp("Silent", true, "10.0.0.9", 5761, false, ""), tcp("Spare", false, "10.0.0.8", 5762, false, "connection refused")]),
            ..Fake::default()
        };
        let listed = get(&api, &host, "/links", "");
        assert_eq!(listed.body["links"][0]["name"], "SITL");
        assert_eq!(listed.body["links"][0]["type"], "tcp", "the type is a token the head can switch on, not a build-order integer");
        assert_eq!((listed.body["links"][0]["connected"].clone(), listed.body["links"][2]["connected"].clone()), (json!(true), json!(false)));
        assert_eq!(
            (listed.body["links"][0]["heardVehicle"].clone(), listed.body["links"][1]["heardVehicle"].clone()),
            (json!(true), json!(false)),
            "connected only means a socket opened, so without heardVehicle \"link up, radio silent\" and \"link up, vehicle talking\" are the same answer and the next hour goes to the wrong equipment"
        );
        assert_eq!(listed.body["links"][2]["lastError"], "connection refused", "lastError separates never-connected from connect-refused");
        assert_eq!((listed.body["links"][1]["host"].clone(), listed.body["links"][1]["port"].clone()), (json!("10.0.0.9"), json!(5761)), "the endpoint travels so a caller can see where a link actually points");
    }

    #[test]
    fn connecting_needs_a_host_and_a_usable_port_and_names_the_link_when_the_caller_does_not() {
        let api = DebugApi::new();
        let host = Fake::default();
        assert_eq!(get(&api, &host, "/links/connect", "port=5760").body["error"], "host required");
        assert_eq!(get(&api, &host, "/links/connect", "host=127.0.0.1").body["error"], "port required, e.g. port=5760");
        assert_eq!(get(&api, &host, "/links/connect", "host=127.0.0.1&port=0").body["error"], "port must be 1-65535, got: 0", "port zero is not a port");
        assert_eq!(
            get(&api, &host, "/links/connect", "host=127.0.0.1&port=70000").body["error"],
            "port must be 1-65535, got: 70000",
            "\"host and port required\" is a false statement when a port was supplied, and it sends the caller back to re-check the host"
        );
        assert_eq!(get(&api, &host, "/links/connect", "host=127.0.0.1&port=abc").body["error"], "port must be 1-65535, got: abc");
        let opened = get(&api, &host, "/links/connect", "host=127.0.0.1&port=5760");
        assert_eq!(opened.body["name"], "debug-api 127.0.0.1:5760");
        assert_eq!((opened.body["connected"].clone(), opened.body["id"].clone()), (json!(true), json!(4)));
        assert_eq!(
            host.calls(),
            vec![r#"open {"auto":false,"highLatency":false,"host":"127.0.0.1","kind":"tcp","name":"debug-api 127.0.0.1:5760","port":5760,"viaLinkManager":true}"#.to_string()],
            "viaLinkManager puts the routing decision in the payload: without it the host opens a core-owned transport that /links cannot list and /links/disconnect?name= cannot close"
        );
    }

    #[test]
    fn connecting_a_link_that_is_already_up_reports_where_it_really_points() {
        let api = DebugApi::new();
        let host = Fake { links: model(&[tcp("SITL", true, "127.0.0.1", 5760, true, "")]), ..Fake::default() };
        let again = get(&api, &host, "/links/connect", "name=SITL&host=127.0.0.1&port=5760");
        assert_eq!((again.body["connected"].clone(), again.body["existing"].clone()), (json!(true), json!(true)));
        assert_eq!(again.body["id"], Value::Null, "no link was opened, so there is no id to report and zero would be a lie");
        assert!(host.calls().is_empty(), "an already connected link is left alone");
        let repointed = get(&api, &host, "/links/connect", "name=SITL&host=192.168.1.50&port=5760");
        assert_eq!(
            (repointed.status, repointed.body["error"].clone()),
            (STATUS_BAD_REQUEST, json!("link SITL is already connected to 127.0.0.1:5760; disconnect it first")),
            "echoing the endpoint the caller asked for reports a link to a machine it was never pointed at; a debug API stating a fact that is not true is worse than refusing"
        );
        assert!(host.calls().is_empty(), "and nothing was repointed, so nothing reached the host either");
    }

    #[test]
    fn connecting_a_name_held_by_another_kind_of_link_is_refused_and_a_down_link_is_repointed() {
        let api = DebugApi::new();
        let serial = Fake { links: model(&[json!({ "name": "Radio", "settingsURL": "SerialSettings.qml", "children": [] })]), ..Fake::default() };
        assert_eq!(
            get(&api, &serial, "/links/connect", "name=Radio&host=127.0.0.1&port=5760").body["error"],
            "link Radio exists but is not tcp",
            "two configurations under one name make /links/disconnect?name= ambiguous, and the name is the only key it indexes by"
        );
        assert!(serial.calls().is_empty());
        let down = Fake { links: model(&[tcp("SITL", false, "127.0.0.1", 5760, false, "")]), ..Fake::default() };
        let moved = get(&api, &down, "/links/connect", "name=SITL&host=192.168.1.50&port=5761");
        assert_eq!((moved.body["existing"].clone(), moved.body["host"].clone(), moved.body["port"].clone()), (json!(true), json!("192.168.1.50"), json!(5761)));
        assert!(down.calls()[0].contains(r#""host":"192.168.1.50","kind":"tcp","name":"SITL","port":5761"#), "a configuration that is not connected takes the new endpoint, the way the C++ set host and port on the saved config");
    }

    #[test]
    fn a_failed_open_is_refused_with_the_reason_the_core_gave() {
        let api = DebugApi::new();
        let host = Fake { open: Some(Err("port 14550 is already served by a Qt link".to_string())), ..Fake::default() };
        let failed = get(&api, &host, "/links/connect", "host=127.0.0.1&port=5760");
        assert_eq!(failed.status, STATUS_BAD_REQUEST);
        assert_eq!(failed.body["error"], "port 14550 is already served by a Qt link", "the reason travels to the caller unrewritten");
    }

    #[test]
    fn disconnecting_by_name_needs_a_connected_link_and_no_name_drops_them_all() {
        let api = DebugApi::new();
        let host = Fake { links: plain(&[("SITL", true), ("Spare", false)]), ..Fake::default() };
        assert_eq!(get(&api, &host, "/links/disconnect", "name=Spare").body["error"], "no connected link named Spare");
        assert_eq!(get(&api, &host, "/links/disconnect", "name=Ghost").body["error"], "no connected link named Ghost");
        let named = get(&api, &host, "/links/disconnect", "name=SITL");
        assert_eq!((named.body["disconnected"].clone(), named.body["count"].clone()), (json!("SITL"), json!(1)));
        let all = get(&api, &host, "/links/disconnect", "");
        assert_eq!((all.body["disconnected"].clone(), all.body["count"].clone()), (json!("all"), json!(3)));
        assert_eq!(host.calls(), vec!["close Some(\"SITL\")".to_string(), "close None".to_string()], "a name that is not connected never reaches the host");
    }

    #[test]
    fn a_mock_link_needs_a_debug_build_and_a_known_autopilot() {
        let api = DebugApi::new();
        let release = Fake::default();
        assert_eq!(get(&api, &release, "/links/mocklink", "autopilot=px4").body["error"], "mock links exist only in debug builds");
        let host = Fake { mock_available: true, ..Fake::default() };
        assert_eq!(get(&api, &host, "/links/mocklink", "").body["error"], "autopilot must be px4 or apm");
        assert_eq!(get(&api, &host, "/links/mocklink", "autopilot=inav").body["error"], "autopilot must be px4 or apm, got: inav");
        let started = get(&api, &host, "/links/mocklink", "autopilot=PX4");
        assert_eq!((started.body["started"].clone(), started.body["autopilot"].clone()), (json!(true), json!("px4")));
        get(&api, &host, "/links/mocklink", "autopilot=arducopter");
        let opened = host.calls();
        assert!(opened[0].contains("\"firmwareType\":12"), "px4 is MAV_AUTOPILOT_PX4 = 12, the number that decides which firmware plugin loads");
        assert!(opened[1].contains("\"firmwareType\":3"), "apm is MAV_AUTOPILOT_ARDUPILOTMEGA = 3");
        assert!(opened[0].contains("\"vehicleType\":2"), "MAV_TYPE_QUADROTOR = 2");
        assert!(opened[0].contains("\"dynamic\":true"));
        assert!(
            opened[0].contains("\"viaLinkManager\":true"),
            "the core owns no Mock transport, so without the flag the host has to sniff the kind and a straight forward answers a Rust Debug dump of an internal enum"
        );
        assert_eq!((AUTOPILOT_PX4, AUTOPILOT_ARDUPILOTMEGA, VEHICLE_TYPE_QUADROTOR), (12, 3, 2), "the constants are the MAVLink numbers, pinned here rather than asserted against themselves");
    }

    #[test]
    fn a_mock_configuration_is_listed_as_a_mock_link() {
        let api = DebugApi::new();
        let host = Fake { links: model(&[json!({ "name": MOCK_LINK_NAME, "settingsURL": "MockLinkSettings.qml", "children": ["link"] })]), ..Fake::default() };
        assert_eq!(
            get(&api, &host, "/links", "").body["links"][0]["type"],
            "mock",
            "a caller that just started a mock link has to be able to find it in the listing, and \"other\" names no link kind"
        );
    }

    #[test]
    fn an_existing_mock_link_is_reported_not_duplicated_unless_add_is_asked_for() {
        let api = DebugApi::new();
        let host = Fake { mock_available: true, mock_present: true, ..Fake::default() };
        let existing = get(&api, &host, "/links/mocklink", "autopilot=px4");
        assert_eq!((existing.body["started"].clone(), existing.body["existing"].clone()), (json!(false), json!(true)));
        assert!(host.calls().is_empty());
        let added = get(&api, &host, "/links/mocklink", "autopilot=px4&add=1");
        assert_eq!((added.body["started"].clone(), added.body["existing"].clone()), (json!(true), json!(false)), "add is the caller asking for a second one on purpose");
        assert_eq!(host.calls().len(), 1);
    }

    fn write_plan(name: &str, body: &str) -> String {
        let path = std::env::temp_dir().join(format!("qgc-debugapi-{name}.plan"));
        std::fs::write(&path, body).unwrap();
        path.to_string_lossy().into_owned()
    }

    const PLAN: &str = r#"{"fileType":"Plan","version":1,"mission":{"version":2,"firmwareType":12,"vehicleType":2,"plannedHomePosition":[47.3,8.5,0],
        "items":[{"type":"SimpleItem","frame":3,"command":22,"params":[0,0,0,0,47.3,8.5,10],"autoContinue":true},
                 {"type":"SimpleItem","frame":3,"command":16,"params":[0,0,0,0,47.4,8.6,20],"autoContinue":true}]}}"#;

    const SURVEY_PLAN: &str = r#"{"fileType":"Plan","version":1,
        "geoFence":{"version":2,"polygons":[{"inclusion":true,"polygon":[[47.3,8.5],[47.4,8.5],[47.4,8.6]]}],"circles":[{"inclusion":false,"circle":{"center":[47.35,8.55],"radius":30}}]},
        "rallyPoints":{"version":2,"points":[[47.31,8.51,25]]},
        "mission":{"version":2,"firmwareType":12,"vehicleType":2,"plannedHomePosition":[47.3,8.5,0],
        "items":[{"type":"SimpleItem","frame":3,"command":22,"params":[0,0,0,0,47.3,8.5,10],"autoContinue":true},
                 {"type":"ComplexItem","complexItemType":"survey","angle":0,"TransectStyleComplexItem":{"version":1,"Items":[]}}]}}"#;

    #[test]
    fn an_upload_hands_the_whole_plan_file_to_the_host_and_says_what_is_in_it() {
        let api = DebugApi::new();
        let grounded = Fake::default();
        assert_eq!(get(&api, &grounded, "/mission/upload", "file=/tmp/x.plan").body["error"], "no vehicle connected");
        let host = Fake { vehicle: true, ..Fake::default() };
        assert_eq!(get(&api, &host, "/mission/upload", "").body["error"], "file required");
        let file = write_plan("upload", PLAN);
        let sent = get(&api, &host, "/mission/upload", &format!("file={file}"));
        assert_eq!((sent.status, sent.body["itemCount"].clone()), (STATUS_OK, json!(2)));
        let request: Value = serde_json::from_str(host.calls()[0].trim_start_matches("mission ")).unwrap();
        assert_eq!((request["action"].clone(), request["file"].clone()), (json!("write"), json!(file)), "the C++ handed the path to PlanMasterController, which expands complex items and chains mission then geofence then rally");
        assert_eq!(
            request["plan"],
            "all",
            "a mission-only write drops the geofence and the rally points while answering 200, so the scope is named and a host wired to the mission transfer alone fails loudly instead"
        );
        assert_eq!(request["items"], Value::Null, "the core no longer emits items, so there is nothing for it to silently leave out");
    }

    #[test]
    fn a_survey_plan_uploads_with_its_fence_and_rally_points_counted() {
        let api = DebugApi::new();
        let host = Fake { vehicle: true, ..Fake::default() };
        let file = write_plan("survey", SURVEY_PLAN);
        let sent = get(&api, &host, "/mission/upload", &format!("file={file}"));
        assert_eq!(
            (sent.status, sent.body["complexItems"].clone()),
            (STATUS_OK, json!(1)),
            "QGC saves every survey, corridor scan, structure scan and landing pattern as a ComplexItem, and refusing them removes the plan an operator most often uploads"
        );
        assert_eq!(
            (sent.body["fencePolygons"].clone(), sent.body["fenceCircles"].clone(), sent.body["rallyPoints"].clone()),
            (json!(1), json!(1), json!(1)),
            "the aircraft must not fly with no fence while the caller reads an unqualified success, so every section of the plan is named in the answer"
        );
        assert_eq!(sent.body["uploading"], json!(file));
    }

    #[test]
    fn an_upload_needs_a_readable_file_with_something_in_it() {
        let api = DebugApi::new();
        let host = Fake { vehicle: true, ..Fake::default() };
        assert!(get(&api, &host, "/mission/upload", "file=/nope/missing.plan").body["error"].as_str().unwrap().starts_with("cannot read /nope/missing.plan: "));
        let directory = std::env::temp_dir().to_string_lossy().into_owned();
        let unreadable = get(&api, &host, "/mission/upload", &format!("file={directory}"));
        assert_eq!(unreadable.status, STATUS_BAD_REQUEST);
        assert!(
            !unreadable.body["error"].as_str().unwrap().contains("no such file"),
            "a directory, a permission error and non-UTF-8 content are not a missing path, and calling them one sends the engineer hunting a typo while the file sits right there"
        );
        let empty = write_plan("empty", r#"{"fileType":"Plan","version":1,"mission":{"version":2,"items":[]}}"#);
        assert!(get(&api, &host, "/mission/upload", &format!("file={empty}")).body["error"].as_str().unwrap().starts_with("nothing to upload"));
        let fence_only = write_plan("fenceonly", r#"{"fileType":"Plan","version":1,"geoFence":{"polygons":[{"inclusion":true,"polygon":[[47.3,8.5]]}]},"mission":{"version":2,"items":[]}}"#);
        assert_eq!(get(&api, &host, "/mission/upload", &format!("file={fence_only}")).status, STATUS_OK, "a plan whose only content is a fence is still a plan worth sending");
        let legacy = write_plan("legacy", r#"{"version":2,"firmwareType":12,"items":[{"type":"SimpleItem","frame":3,"command":16,"params":[0,0,0,0,47.3,8.5,10],"autoContinue":true}]}"#);
        assert_eq!(get(&api, &host, "/mission/upload", &format!("file={legacy}")).body["itemCount"], json!(1), "a bare .mission file still counts, the way loadFromFile took either shape");
    }

    #[test]
    fn a_download_asks_for_the_whole_plan_and_an_empty_transfer_writes_nothing() {
        let api = DebugApi::new();
        let host = Fake { vehicle: true, ..Fake::default() };
        assert_eq!(api.settle_download(1, false, 0), Settled::Idle, "nothing was asked for, so there is nothing to settle");
        let started = get(&api, &host, "/mission/download", "file=/tmp/out.plan");
        assert_eq!((started.status, started.body["downloading"].clone()), (STATUS_OK, json!("/tmp/out.plan")));
        let transfer = started.body["transfer"].as_u64().unwrap();
        assert_eq!(
            host.calls(),
            vec![r#"mission {"action":"load","plan":"all"}"#.to_string()],
            "loadFromVehicle fetched the mission, the geofence and the rally points, and a mission-only load writes a .plan missing two of the three"
        );
        assert_eq!(api.settle_download(transfer, true, 0), Settled::Waiting, "a transfer still running is not an empty one");
        assert_eq!(
            api.settle_download(transfer, false, 0),
            Settled::Empty("/tmp/out.plan".to_string()),
            "an empty download must not produce a file, because callers poll for the file as the success signal"
        );
        assert_eq!(api.settle_download(transfer, false, 3), Settled::Idle, "settling clears the pending file, so a later transfer cannot write it again");
        let again = get(&api, &host, "/mission/download", "file=/tmp/out.plan");
        assert_eq!(api.settle_download(again.body["transfer"].as_u64().unwrap(), false, 3), Settled::Write("/tmp/out.plan".to_string()));
        assert_eq!(api.pending_download(), None);
    }

    #[test]
    fn a_download_that_never_finishes_cannot_be_settled_by_a_later_transfer() {
        let api = DebugApi::new();
        let host = Fake { vehicle: true, ..Fake::default() };
        let abandoned = get(&api, &host, "/mission/download", "file=/tmp/first.plan").body["transfer"].as_u64().unwrap();
        let second = get(&api, &host, "/mission/download", "file=/tmp/second.plan").body["transfer"].as_u64().unwrap();
        assert_ne!(abandoned, second, "each download is its own transfer, or a stale completion satisfies the wrong request");
        assert_eq!(api.settle_download(abandoned, false, 5), Settled::Idle, "a completion carrying the abandoned transfer writes nothing");
        assert_eq!(api.settle_download(second, false, 5), Settled::Write("/tmp/second.plan".to_string()));

        let stranded = get(&api, &host, "/mission/download", "file=/tmp/stranded.plan").body["transfer"].as_u64().unwrap();
        let file = write_plan("clears", PLAN);
        get(&api, &host, "/mission/upload", &format!("file={file}"));
        assert_eq!(api.pending_download(), None, "an upload clears the latch, or the sync it starts resolves as the old download and writes the uploaded plan to the download path");
        assert_eq!(api.settle_download(stranded, false, 2), Settled::Idle);
    }

    #[test]
    fn a_download_the_core_refuses_leaves_no_pending_file_behind() {
        let api = DebugApi::new();
        let host = Fake { vehicle: true, mission: Some(Err("A plan transfer is still in progress.".to_string())), ..Fake::default() };
        let refused = get(&api, &host, "/mission/download", "file=/tmp/out.plan");
        assert_eq!((refused.status, refused.body["error"].clone()), (STATUS_BAD_REQUEST, json!("A plan transfer is still in progress.")));
        assert_eq!(api.pending_download(), None, "a refused start leaves no latch that a later unrelated transfer would satisfy");
        assert_eq!(api.settle_download(1, false, 5), Settled::Idle);
    }

    #[test]
    fn logging_rules_are_required_and_semicolons_become_lines() {
        let api = DebugApi::new();
        let host = Fake::default();
        assert_eq!(get(&api, &host, "/logging", "").body["error"], "rules required, e.g. qgc.videomanager.*.debug=true");
        let set = get(&api, &host, "/logging", "rules=qgc.a.debug%3Dtrue%3Bqgc.b.debug%3Dfalse");
        assert_eq!(set.body["rules"], "qgc.a.debug=true;qgc.b.debug=false", "the caller gets back what it asked for");
        assert_eq!(host.calls(), vec!["logging qgc.a.debug=true\nqgc.b.debug=false".to_string()], "Qt wants one rule per line");
    }

    struct Reentrant {
        api: DebugApi,
        inner: Cell<u16>,
        retry: Cell<Value>,
    }

    impl Host for Reentrant {
        fn bridge_get(&self, _path: &str) -> String {
            let nested = self.api.dispatch(self, "GET", "/links", "", "");
            self.inner.set(nested.status);
            self.retry.set(nested.body["retry"].clone());
            json!({ "kind": "value", "value": 1 }).to_string()
        }
        fn bridge_set(&self, _path: &str, _payload_json: &str) -> String {
            String::new()
        }
        fn bridge_invoke(&self, _path: &str, _args_json: &str) -> String {
            String::new()
        }
        fn link_configurations(&self) -> Value {
            Value::Null
        }
        fn link_open(&self, _config_json: &str) -> Result<u32, String> {
            Ok(0)
        }
        fn link_close(&self, _name: Option<&str>) -> Result<usize, String> {
            Ok(0)
        }
        fn mission(&self, _request_json: &str) -> Result<(), String> {
            Ok(())
        }
        fn set_logging_rules(&self, _rules: &str) {}
        fn bridge_writes_allowed(&self) -> bool {
            false
        }
        fn mock_links_available(&self) -> bool {
            false
        }
        fn mock_link_present(&self) -> bool {
            false
        }
        fn vehicle_connected(&self) -> bool {
            false
        }
    }

    #[test]
    fn a_request_arriving_inside_another_one_is_told_to_come_back_and_the_guard_clears_itself() {
        let host = Reentrant { api: DebugApi::new(), inner: Cell::new(0), retry: Cell::new(Value::Null) };
        let outer = host.api.dispatch(&host, "GET", "/bridge/get", "path=x", "");
        assert_eq!(outer.status, STATUS_OK);
        assert_eq!(
            (host.inner.get(), host.retry.replace(Value::Null)),
            (STATUS_UNAVAILABLE, json!(true)),
            "busy is the one refusal here that is retryable, and an agent that reads 400 as \"my request was wrong\" abandons a diagnosis that would have succeeded a moment later"
        );
        let after = host.api.dispatch(&host, "GET", "/links", "", "");
        assert_eq!(after.status, STATUS_OK, "the busy flag is cleared on the way out, so it is a guard and not a latch");
    }

    #[test]
    fn the_view_lists_the_split_and_resolves_a_request_without_performing_it() {
        let core = crate::router::Core::new(Bare);
        let table: Value = serde_json::from_str(&core.get("view.debugApi")).unwrap();
        assert_eq!(table["count"], 10, "the ten routes the core serves, written out rather than compared with the list they came from");
        assert_eq!(ROUTES.len(), 10);
        assert!(table["hostRoutes"].as_array().unwrap().iter().any(|r| r["path"] == "/native/probe"));
        assert!(table["hostRoutes"].as_array().unwrap().iter().any(|r| r["path"] == "/vehicle/rc"));
        let resolved: Value = serde_json::from_str(&core.get("view.debugApi(GET,/links/connect,host=127.0.0.1&port=5760)")).unwrap();
        assert_eq!((resolved["accepted"].clone(), resolved["status"].clone()), (json!(true), json!(200)));
        assert_eq!(resolved["query"][1]["value"], "5760");
        let native: Value = serde_json::from_str(&core.get("view.debugApi(GET,/native/menu)")).unwrap();
        assert_eq!((native["accepted"].clone(), native["status"].clone()), (json!(false), json!(404)));
        assert_eq!(native["hostOwned"], "reads the AppKit menu bar");
        let posted: Value = serde_json::from_str(&core.get("view.debugApi(POST,/links)")).unwrap();
        assert_eq!(posted["status"], json!(400));
        let bare: Value = serde_json::from_str(&core.get("view.debugApi(GET)")).unwrap();
        assert!(bare["reason"].as_str().unwrap().contains("a method and a path"), "a method with no path is told what it was missing");
        assert_eq!((STATUS_OK, STATUS_BAD_REQUEST, STATUS_NOT_FOUND, STATUS_UNAVAILABLE), (200, 400, 404, 503), "the four codes the MCP client branches on, pinned to their numbers");
    }

    struct Bare;

    impl Backend for Bare {
        fn get(&self, _path: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn get_fields(&self, _path: &str, _fields: &str) -> String {
            json!({ "kind": "null" }).to_string()
        }
        fn set(&self, _path: &str, _value: &str) -> String {
            json!({ "ok": false }).to_string()
        }
        fn invoke(&self, _path: &str, _args: &str) -> String {
            json!({ "ok": false }).to_string()
        }
        fn watch(&self, _paths: &[String]) {}
    }
}
