use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use regex::Regex;
use serde_json::{Value, json};

use crate::read::{flag, integer, text};
use crate::router::Backend;

pub const WIZARD_CHANGED: &str = "core.deviceWizard@changed";
pub const DEPS: &[&str] = &[WIZARD_CHANGED];
pub const WIZARD_OPEN: &str = "deviceWizard.open";
pub const WIZARD_SCAN: &str = "deviceWizard.scan";
pub const WIZARD_FIX_PORT: &str = "deviceWizard.fixPort";
pub const WIZARD_TEMPLATE: &str = "deviceWizard.template";
pub const WIZARD_NEXT: &str = "deviceWizard.next";
pub const WIZARD_FORM: &str = "deviceWizard.form";
pub const WIZARD_SIGN_IN: &str = "deviceWizard.signIn";
pub const WIZARD_APPLY: &str = "deviceWizard.apply";
pub const WIZARD_RESTART: &str = "deviceWizard.restart";
const WIZARD_ACTIONS: &[&str] = &[
    WIZARD_OPEN,
    WIZARD_SCAN,
    WIZARD_FIX_PORT,
    WIZARD_TEMPLATE,
    WIZARD_NEXT,
    WIZARD_FORM,
    WIZARD_SIGN_IN,
    WIZARD_APPLY,
    WIZARD_RESTART,
];

const PI_CAMERA_SOURCE: &str = "rpiCamera";
const SCANNING_NOTE: &str = "Looking for cameras on every port of this device — up to 20 seconds…";
const SCAN_FAILED_NOTE: &str = "Couldn't reach this device to run the scan — check you're still connected to it, or type the camera's address below.";
const NO_ANSWER_CHECKLIST: &str = "No camera answered — check it, then scan again";
const ALL_ADDED: &str = "Every camera that answered is already set up on this device — a newly plugged-in one has not answered yet.";
const VERIFYING: &str = "Verifying the stream is live…";
const SCAN_PORTS_PATH: &str = "/api/streams/scan/ports";
const SCAN_PATH: &str = "/api/streams/scan";
const VERIFY_ATTEMPTS: usize = 10;
const VERIFY_POLL: Duration = Duration::from_millis(1500);
const PORT_UP_ATTEMPTS: usize = 15;
const PORT_UP_POLL: Duration = Duration::from_secs(1);

fn pattern(source: &str) -> Regex {
    Regex::new(source).expect("a fixed pattern compiles")
}

static URL_SCHEME: LazyLock<Regex> = LazyLock::new(|| pattern(r"(?i)^[a-z][a-z0-9+.-]*://"));
static BARE_ADDRESS: LazyLock<Regex> =
    LazyLock::new(|| pattern(r"^[A-Za-z0-9_.-]+(:[0-9]+)?(/.*)?$"));
static TYPED_CIDR: LazyLock<Regex> = LazyLock::new(|| {
    pattern(r"(?i)^(?:rtsps?://)?([0-9]{1,3}(?:\.[0-9]{1,3}){3})/([0-9]|[12][0-9]|3[0-2])$")
});
static RTSP_URL: LazyLock<Regex> = LazyLock::new(|| pattern(r"(?i)^rtsps?://."));
static RTSP_SCHEME: LazyLock<Regex> = LazyLock::new(|| pattern(r"(?i)^rtsps?://"));
static LAST_OCTET: LazyLock<Regex> = LazyLock::new(|| pattern(r"\.[0-9]+$"));

#[derive(Clone, Debug, Default, PartialEq)]
struct Neighbor {
    addr: String,
    suggest: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Port {
    iface: String,
    label: String,
    kind: String,
    state: String,
    subnet: String,
    checked: i64,
    reason: String,
    tried_address: String,
    uplink: bool,
    neighbors: Vec<Neighbor>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Camera {
    host: String,
    url: String,
    needs_auth: bool,
    path_found: bool,
    used: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Subnet {
    cidr: String,
    label: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Other {
    name: String,
    source: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Template {
    id: String,
    label: String,
    hint: String,
    cidr: String,
    host: String,
    name: String,
    gimbal: String,
    gimbal_addr: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Wizard {
    name: String,
    is_new: bool,
    rtsp: String,
    source: String,
    subnets: Vec<Subnet>,
    others: Vec<Other>,
    templates: Vec<Template>,
}

#[derive(Clone, Debug, PartialEq)]
struct Scanned {
    port: Port,
    found: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Outcome {
    cams: Vec<Camera>,
    ports: Vec<Port>,
    todo: Vec<Scanned>,
    rest: Vec<Scanned>,
    note: String,
}

fn objects<'a>(json: &'a Value, key: &'static str) -> impl Iterator<Item = &'a Value> {
    json.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|it| it.is_object())
}

fn wizard_of(json: &Value) -> Wizard {
    Wizard {
        name: text(json, "name"),
        is_new: flag(json, "new"),
        rtsp: text(json, "rtsp"),
        source: text(json, "source"),
        subnets: objects(json, "subnets")
            .map(|it| Subnet {
                cidr: text(it, "cidr"),
                label: text(it, "label"),
            })
            .collect(),
        others: objects(json, "others")
            .map(|it| Other {
                name: text(it, "name"),
                source: text(it, "source"),
            })
            .collect(),
        templates: objects(json, "templates")
            .map(|it| Template {
                id: text(it, "id"),
                label: text(it, "label"),
                hint: text(it, "hint"),
                cidr: text(it, "cidr"),
                host: text(it, "host"),
                name: text(it, "name"),
                gimbal: text(it, "gimbal"),
                gimbal_addr: text(it, "gimbalAddr"),
            })
            .collect(),
    }
}

fn ports_of(json: &Value) -> Vec<Port> {
    objects(json, "ports")
        .map(|it| Port {
            iface: text(it, "interface"),
            label: text(it, "label"),
            kind: text(it, "type"),
            state: text(it, "state"),
            subnet: text(it, "subnet"),
            checked: integer(it, "checked").unwrap_or(0),
            reason: text(it, "reason"),
            tried_address: text(it, "triedAddress"),
            uplink: flag(it, "isUplink"),
            neighbors: objects(it, "neighbors")
                .map(|neighbor| Neighbor {
                    addr: text(neighbor, "addr"),
                    suggest: text(neighbor, "suggest"),
                })
                .collect(),
        })
        .collect()
}

fn cameras_of(json: &Value, wizard_name: &str) -> Vec<Camera> {
    objects(json, "cameras")
        .map(|it| Camera {
            host: text(it, "host"),
            url: text(it, "url"),
            needs_auth: flag(it, "needsAuth"),
            path_found: flag(it, "pathFound"),
            used: Some(text(it, "usedBy"))
                .filter(|used| used != wizard_name)
                .unwrap_or_default(),
        })
        .collect()
}

fn rtsp_host(rtsp: &str) -> String {
    url::Url::parse(rtsp)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_default()
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn suggest_ip(rtsp: &str) -> String {
    let host = rtsp_host(rtsp);
    let octets: Vec<&str> = host.split('.').collect();
    match octets.as_slice() {
        [a, b, c, d] if octets.iter().all(|octet| digits(octet)) => {
            format!("{a}.{b}.{c}.{}/24", if *d == "2" { "3" } else { "2" })
        }
        _ => String::new(),
    }
}

fn normalize_rtsp(raw: &str) -> String {
    let value = raw.trim();
    match value.is_empty() || URL_SCHEME.is_match(value) || !BARE_ADDRESS.is_match(value) {
        true => value.to_string(),
        false => format!("rtsp://{value}"),
    }
}

fn cidr_in(rtsp: &str) -> String {
    TYPED_CIDR
        .captures(rtsp.trim())
        .map(|parts| format!("{}/{}", &parts[1], &parts[2]))
        .unwrap_or_default()
}

fn ip_to_int(ip: &str) -> Option<u32> {
    let parts: Vec<&str> = ip.split('.').collect();
    (parts.len() == 4).then_some(())?;
    parts.iter().try_fold(0u32, |sum, part| {
        digits(part)
            .then(|| part.parse::<u32>().ok())
            .flatten()
            .filter(|octet| *octet <= 255)
            .map(|octet| sum * 256 + octet)
    })
}

fn same_subnet(host: &str, subnet: &str) -> bool {
    let (base, rest) = subnet.split_once('/').unwrap_or((subnet, ""));
    let bits = rest
        .split('/')
        .next()
        .and_then(|bits| bits.parse::<u32>().ok())
        .filter(|bits| *bits <= 32);
    match (ip_to_int(host), ip_to_int(base), bits) {
        (Some(a), Some(b), Some(bits)) => {
            let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
            a & mask == b & mask
        }
        _ => false,
    }
}

fn address_part(cidr: &str) -> &str {
    cidr.split('/').next().unwrap_or_default()
}

fn port_has_subnet(port: &Port, want: &str) -> bool {
    let host = address_part(want);
    !port.subnet.is_empty() && !host.is_empty() && same_subnet(host, &port.subnet)
}

fn first_suggest(port: &Port) -> String {
    port.neighbors
        .iter()
        .map(|neighbor| neighbor.suggest.clone())
        .find(|suggest| !suggest.is_empty())
        .unwrap_or_default()
}

fn subnet_name(subnets: &[Subnet], subnet: &str) -> String {
    match subnet.is_empty() {
        true => String::new(),
        false => subnets
            .iter()
            .find(|option| same_subnet(address_part(&option.cidr), subnet))
            .map(|option| option.label.clone())
            .unwrap_or_default(),
    }
}

fn idle_on_camera_subnet(subnets: &[Subnet], port: &Port) -> bool {
    !subnet_name(subnets, &port.subnet).is_empty()
        && !port.neighbors.is_empty()
        && first_suggest(port).is_empty()
}

fn row_note(cam: &Camera) -> String {
    match (cam.used.as_str(), cam.path_found, cam.needs_auth) {
        ("", false, _) => "answers on RTSP — add the stream path".into(),
        ("", true, true) => "needs a username and password".into(),
        ("", true, false) => "ready to use".into(),
        (used, _, _) => format!("already added as {used}"),
    }
}

fn neighbor_label(port: &Port) -> String {
    port.neighbors.first().map_or_else(String::new, |lead| {
        let more = match port.neighbors.len() {
            0 | 1 => String::new(),
            count => format!(" (and {} more)", count - 1),
        };
        let here = match port.subnet.is_empty() {
            true => String::new(),
            false => format!(", not on {}", port.subnet),
        };
        let advice = match lead.suggest.is_empty() {
            true => " — probably a router or switch, not a camera".to_string(),
            false => format!(" — set this port to {} to reach it", lead.suggest),
        };
        format!(
            "a device answered at {}{more} (not confirmed as a camera){here}{advice}",
            lead.addr
        )
    })
}

fn reason_label(reason: &str, address: &str) -> Option<String> {
    match reason {
        "address-unusable" => Some(format!(
            "{address} could not be used — another device on this network already has it, or nothing on that subnet answered"
        )),
        "address-expired" => Some(format!("{address} expired and could not be renewed")),
        "needs-password" => Some("this port needs a password before it can connect".into()),
        "config-failed" => {
            Some("the settings on this port were refused — set an address below".into())
        }
        _ => None,
    }
}

fn state_label(port: &Port) -> String {
    match port.state.as_str() {
        "searched" => format!(
            "checked {} addresses on {} — nothing answered",
            port.checked, port.subnet
        ),
        "no-carrier" if port.kind == "wi-fi" => "not connected to any network".into(),
        "no-carrier" => "nothing plugged in — check the cable and the camera's power".into(),
        "not-configured" => "cable is in, but this port has no address yet".into(),
        "configuring" => "still getting an address — try again in a moment".into(),
        "failed" => "could not get an address — set one below".into(),
        "no-address" => "no address on this port — set one below to reach the camera".into(),
        "subnet-too-big" => format!(
            "{} is too wide to sweep — type the camera's address below",
            port.subnet
        ),
        "no-hosts" => format!("{} has no other address on it", port.subnet),
        other => other.to_string(),
    }
}

fn port_label(port: &Port) -> String {
    Some(neighbor_label(port))
        .filter(|label| !label.is_empty())
        .or_else(|| {
            reason_label(
                &port.reason,
                Some(port.tried_address.as_str())
                    .filter(|address| !address.is_empty())
                    .unwrap_or("that address"),
            )
        })
        .unwrap_or_else(|| state_label(port))
}

fn cable_label(port: &Port, host: &str) -> String {
    match !host.is_empty() && !port.subnet.is_empty() && same_subnet(host, &port.subnet) {
        true => format!(
            "nothing answered at {host} on {} — the port is set right; check the cable goes to the camera, not a router or switch.",
            port.iface
        ),
        false => String::new(),
    }
}

fn port_actionable(port: &Port, found: bool) -> bool {
    !found && !port.uplink && (!port.neighbors.is_empty() || port.state != "searched")
}

fn port_detail(scanned: &Scanned, subnets: &[Subnet], source_host: &str) -> String {
    let port = &scanned.port;
    if scanned.found {
        "camera found here".into()
    } else if port.uplink {
        format!("{} — this is the device's own uplink", port_label(port))
    } else if idle_on_camera_subnet(subnets, port) {
        format!(
            "Set for {} ({}). Something at {} is on this cable — probably a router or switch, not a camera. Plug the camera in here and scan again.",
            subnet_name(subnets, &port.subnet),
            port.subnet,
            port.neighbors
                .first()
                .map(|neighbor| neighbor.addr.as_str())
                .unwrap_or_default()
        )
    } else {
        Some(cable_label(port, source_host))
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| port_label(port))
    }
}

fn scan_outcome(cams: Vec<Camera>, ports: Vec<Port>, source_host: &str) -> Outcome {
    let fresh = cams.iter().any(|cam| cam.used.is_empty());
    let (todo, rest): (Vec<(Scanned, bool)>, Vec<(Scanned, bool)>) = ports
        .iter()
        .map(|port| {
            let found = cams
                .iter()
                .any(|cam| !port.subnet.is_empty() && same_subnet(&cam.host, &port.subnet));
            let wired = !cable_label(port, source_host).is_empty() && !fresh;
            (
                Scanned {
                    port: port.clone(),
                    found,
                },
                wired || port_actionable(port, found || fresh),
            )
        })
        .partition(|(_, actionable)| *actionable);
    let note = if !cams.is_empty() {
        ""
    } else if !ports.is_empty() {
        "No camera answered."
    } else {
        "No camera port on this device — type the camera's address below."
    };
    Outcome {
        cams,
        ports,
        todo: todo.into_iter().map(|(scanned, _)| scanned).collect(),
        rest: rest.into_iter().map(|(scanned, _)| scanned).collect(),
        note: note.into(),
    }
}

fn fixer_open(outcome: &Outcome, subnets: &[Subnet], port: &Port) -> bool {
    !first_suggest(port).is_empty()
        || (outcome.todo.len() == 1 && !idle_on_camera_subnet(subnets, port))
}

fn adopted_camera(outcome: &Outcome, auto: bool, typed: &str) -> Option<Camera> {
    let fresh: Vec<&Camera> = outcome
        .cams
        .iter()
        .filter(|cam| cam.used.is_empty())
        .collect();
    match fresh.as_slice() {
        [only] if only.path_found && !(auto && !typed.trim().is_empty()) => Some((*only).clone()),
        _ => None,
    }
}

fn cam_heading(cams: &[Camera]) -> String {
    match cams.len() {
        1 => "1 camera".into(),
        count => format!("{count} cameras"),
    }
}

fn checklist_hint(ports: &[Port], subnets: &[Subnet]) -> String {
    let dead = ports
        .iter()
        .find(|port| port.kind == "ethernet" && port.state == "no-carrier");
    let idle = ports
        .iter()
        .find(|port| idle_on_camera_subnet(subnets, port));
    match (dead, idle) {
        (Some(dead), _) => format!("Nothing is plugged into the {}.", dead.label),
        (None, Some(idle)) => format!(
            "Something else is plugged into the {} — it looks like a router or switch, not a camera.",
            idle.label
        ),
        (None, None) => String::new(),
    }
}

fn todo_heading(todo: &[Scanned], subnets: &[Subnet]) -> &'static str {
    match todo {
        [only] if idle_on_camera_subnet(subnets, &only.port) => "Check the cable",
        [_] => "Try this",
        _ => "Ports you can still set up",
    }
}

fn subnet_options(port: &Port, ports: &[Port], subnets: &[Subnet]) -> Vec<Subnet> {
    let found = port
        .neighbors
        .iter()
        .filter(|neighbor| !neighbor.suggest.is_empty())
        .map(|neighbor| Subnet {
            cidr: neighbor.suggest.clone(),
            label: format!("found {}", neighbor.addr),
        });
    let tried = Some(&port.tried_address)
        .filter(|address| !address.is_empty())
        .map(|address| Subnet {
            cidr: address.clone(),
            label: "tried before".into(),
        });
    let claimed = |cidr: &str| {
        ports
            .iter()
            .any(|other| other.iface != port.iface && port_has_subnet(other, cidr))
    };
    let all: Vec<Subnet> = found.chain(tried).chain(subnets.iter().cloned()).collect();
    all.iter()
        .enumerate()
        .filter(|(index, option)| {
            !claimed(&option.cidr)
                && all.iter().position(|it| it.cidr == option.cidr) == Some(*index)
        })
        .map(|(_, option)| option.clone())
        .collect()
}

fn fixer_hint(suggest: &str) -> &'static str {
    match suggest.is_empty() {
        false => {
            "This device's address on the camera's subnet. Prefilled from the device that answered on this port."
        }
        true => {
            "This device's address on the camera's subnet — match the camera (a 192.168.144.25 camera needs 192.168.144.x here). Pick your camera's make below, or read its address from its app or manual."
        }
    }
}

fn fixer_go_label(port: &Port, suggest: &str) -> String {
    match suggest.is_empty() {
        false => format!("Set up {} and find the camera", port.iface),
        true => "Set up & scan again".into(),
    }
}

fn template_port(ports: &[Port], template: &Template) -> Option<Port> {
    ports
        .iter()
        .find(|port| !port.uplink && port_has_subnet(port, &template.cidr))
        .or_else(|| {
            ports
                .iter()
                .find(|port| !port.uplink && port.kind == "ethernet" && port.state != "no-carrier")
        })
        .or_else(|| {
            ports
                .iter()
                .find(|port| !port.uplink && port.kind == "ethernet")
        })
        .cloned()
}

fn template_ready_port<'a>(
    template: &Template,
    cams: &[Camera],
    ports: &'a [Port],
) -> Option<&'a Port> {
    match cams.iter().any(|cam| cam.host == template.host) {
        true => None,
        false => ports
            .iter()
            .find(|port| port_has_subnet(port, &template.cidr)),
    }
}

fn template_sub(template: &Template, ready: Option<&Port>) -> String {
    ready.map_or_else(
        || template.hint.clone(),
        |ready| {
            format!(
                "The {} is ready for a {} — nothing answered at {}. Check the cable.",
                ready.label, template.label, template.host
            )
        },
    )
}

fn template_action(busy: bool, ready: Option<&Port>) -> &'static str {
    match (busy, ready) {
        (true, _) => "Setting up…",
        (false, Some(_)) => "Scan again",
        (false, None) => "Set up",
    }
}

fn template_holder_error(holder: &Port, template: &Template) -> String {
    format!(
        "The {} already uses {}, so the camera port cannot take it too — move the {} to another subnet in its app, or disconnect {} first.",
        holder.label,
        LAST_OCTET.replace(address_part(&template.cidr), ".x"),
        template.label,
        holder.label
    )
}

fn uri_component(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => char::from(byte).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn cam_url(base: &str, user: &str, pass: &str) -> String {
    match base.is_empty() || (user.trim().is_empty() && pass.is_empty()) {
        true => base.to_string(),
        false => RTSP_SCHEME
            .replace(base, |scheme: &regex::Captures| {
                format!(
                    "{}{}{}@",
                    &scheme[0],
                    uri_component(user.trim()),
                    match pass.is_empty() {
                        true => String::new(),
                        false => format!(":{}", uri_component(pass)),
                    }
                )
            })
            .into_owned(),
    }
}

fn reachable_port<'a>(rtsp: &str, ports: &'a [Port]) -> Option<&'a Port> {
    let host = rtsp_host(rtsp);
    ports.iter().find(|port| {
        !host.is_empty()
            && !port.uplink
            && !port.subnet.is_empty()
            && same_subnet(&host, &port.subnet)
    })
}

fn commits_now(rtsp: &str, ports: &[Port]) -> bool {
    let typed = rtsp.trim();
    !typed.is_empty() && (typed == PI_CAMERA_SOURCE || reachable_port(typed, ports).is_some())
}

fn next_label(rtsp: &str, ports: &[Port], is_new: bool) -> &'static str {
    match (commits_now(rtsp, ports), is_new) {
        (false, _) => "Continue",
        (true, true) => "Add camera",
        (true, false) => "Set up & verify",
    }
}

fn port_to_suggest(ports: &[Port]) -> String {
    ports
        .iter()
        .find(|port| !port.uplink && port.state != "searched")
        .map(|port| port.iface.clone())
        .unwrap_or_default()
}

fn apply_note(iface: &str, ip: &str, current: &str) -> String {
    let ip = ip.trim();
    if iface.is_empty() || ip.is_empty() {
        String::new()
    } else if current == ip {
        format!("{iface} keeps {ip} — nothing on that port changes.")
    } else {
        format!("Sets {iface} to {ip} — anything else on that port may need reconnecting.")
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Refusal {
    text: String,
    warn: bool,
    field: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
enum Step {
    Refuse(Refusal),
    Apply,
    Port,
}

#[derive(Clone, Debug, PartialEq)]
struct Next {
    step: Step,
    rtsp: String,
    name: Option<String>,
    dup_ack: String,
}

fn refusal(text: String, field: Option<&'static str>) -> Refusal {
    Refusal {
        text,
        warn: false,
        field,
    }
}

fn next_step(
    typed: &str,
    typed_name: Option<&str>,
    others: &[Other],
    ports: &[Port],
    dup_ack: &str,
) -> Next {
    let cidr = cidr_in(typed);
    if !cidr.is_empty() {
        return Next {
            step: Step::Refuse(refusal(
                format!(
                    "{cidr} is an address for this device's port, not the camera. It belongs in the port box above — set the port to it, and the scan fills this field with the camera's own URL."
                ),
                Some("rtsp"),
            )),
            rtsp: typed.to_string(),
            name: typed_name.map(str::to_string),
            dup_ack: dup_ack.to_string(),
        };
    }
    let pi_camera = typed.trim() == PI_CAMERA_SOURCE;
    let rtsp = match pi_camera {
        true => PI_CAMERA_SOURCE.to_string(),
        false => normalize_rtsp(typed),
    };
    let name = typed_name.map(|name| name.trim().to_lowercase());
    let duplicate = others
        .iter()
        .find(|other| other.source.trim() == rtsp && dup_ack != rtsp);
    let refused = if name.as_deref() == Some("") {
        Some(refusal("Give the camera a name.".into(), Some("name")))
    } else if let Some(taken) = name.as_deref().filter(|name| {
        others
            .iter()
            .any(|other| other.name.to_lowercase() == *name)
    }) {
        Some(refusal(
            format!("A camera named “{taken}” already exists — pick another name."),
            Some("name"),
        ))
    } else if !pi_camera && !RTSP_URL.is_match(&rtsp) {
        Some(refusal(
            "Enter the camera's address, e.g. 192.168.8.10 or rtsp://192.168.8.10:554/stream"
                .into(),
            Some("rtsp"),
        ))
    } else {
        duplicate.map(|duplicate| Refusal {
            text: format!("Camera “{}” already uses this source. If that camera only allows one connection, this one won't get video. Click Continue again to add it anyway, or change the URL.", duplicate.name),
            warn: true,
            field: None,
        })
    };
    let dup_ack = match &refused {
        Some(Refusal { warn: true, .. }) => rtsp.clone(),
        _ => dup_ack.to_string(),
    };
    let step = match refused {
        Some(refused) => Step::Refuse(refused),
        None if pi_camera || reachable_port(&rtsp, ports).is_some() => Step::Apply,
        None => Step::Port,
    };
    Next {
        step,
        rtsp,
        name,
        dup_ack,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Found {
    n: u64,
    refusal: String,
    camera: Option<Camera>,
    name: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Applying {
    n: u64,
    state: &'static str,
    text: String,
    field: String,
    url: String,
    camera: String,
}

#[derive(Clone, Debug, Default)]
struct Session {
    id: u64,
    wizard: Wizard,
    scanning: bool,
    note: String,
    outcome: Option<Outcome>,
    scans: u64,
    adopt: Option<Camera>,
    no_port: bool,
    fixing: BTreeMap<String, (bool, String)>,
    template_busy: String,
    template_note: String,
    found: Found,
    applying: Applying,
    apply_job: u64,
    added: bool,
    notice: (u64, String),
}

static SESSION: LazyLock<Mutex<Session>> = LazyLock::new(|| Mutex::new(Session::default()));
static WIZARD_DIRTY: AtomicBool = AtomicBool::new(false);

fn session() -> MutexGuard<'static, Session> {
    SESSION.lock().unwrap_or_else(PoisonError::into_inner)
}

fn change_where(holds: impl FnOnce(&Session) -> bool, edit: impl FnOnce(&mut Session)) -> bool {
    let mut held = session();
    let current = holds(&held);
    if current {
        edit(&mut held);
        WIZARD_DIRTY.store(true, Ordering::Relaxed);
    }
    current
}

fn change(id: u64, edit: impl FnOnce(&mut Session)) -> bool {
    change_where(|held| held.id == id, edit)
}

pub fn take_changed() -> bool {
    WIZARD_DIRTY.swap(false, Ordering::Relaxed)
}

trait Device {
    async fn ask(&self, method: &str, path: &str, body: Option<Value>) -> Value;
    async fn pause(&self, wait: Duration);
}

struct Connected;

const SLOW_CALLS: &[(&str, u64)] = &[(SCAN_PATH, 60)];

fn device_request(method: &str, path: &str, body: Option<Value>) -> (&'static str, String) {
    let wait = SLOW_CALLS
        .iter()
        .find(|(slow, _)| *slow == path)
        .map(|(_, seconds)| json!(seconds));
    match method {
        "GET" => (crate::deviceviews::READ, json!([path]).to_string()),
        _ => (
            crate::deviceviews::CALL,
            json!([method, path, body.unwrap_or(Value::Null), wait]).to_string(),
        ),
    }
}

impl Device for Connected {
    #[cfg(not(target_family = "wasm"))]
    async fn ask(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let (action, args) = device_request(method, path, body);
        crate::deviceviews::run(action, &args)
    }

    #[cfg(target_family = "wasm")]
    async fn ask(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let (action, args) = device_request(method, path, body);
        crate::deviceviews::run_later(action, &args).await
    }

    #[cfg(not(target_family = "wasm"))]
    async fn pause(&self, wait: Duration) {
        std::thread::sleep(wait);
    }

    #[cfg(target_family = "wasm")]
    async fn pause(&self, wait: Duration) {
        crate::background::pause(wait).await;
    }
}

#[cfg(not(target_family = "wasm"))]
fn launch<F: Future<Output = ()>>(job: impl FnOnce() -> F + Send + 'static) {
    let _ = crate::background::spawn("groundstation-device-wizard", move || {
        crate::ubxbase::block_on(job())
    });
}

#[cfg(target_family = "wasm")]
fn launch<F: Future<Output = ()> + 'static>(job: impl FnOnce() -> F + Send + 'static) {
    wasm_bindgen_futures::spawn_local(job());
}

async fn read(device: &impl Device, path: &str) -> Option<Value> {
    let answer = device.ask("GET", path, None).await;
    flag(&answer, "ok").then(|| answer["result"].clone())
}

async fn ports_now(device: &impl Device) -> Vec<Port> {
    read(device, SCAN_PORTS_PATH)
        .await
        .map(|json| ports_of(&json))
        .unwrap_or_default()
}

fn refusal_of(answer: &Value, fallback: &str) -> Option<String> {
    match (flag(answer, "ok"), answer.get("result")) {
        (true, _) => None,
        (false, Some(body)) => Some(
            Some(text(body, "error"))
                .filter(|error| !error.is_empty())
                .unwrap_or_else(|| fallback.to_string()),
        ),
        (false, None) => Some(text(answer, "reason")),
    }
}

async fn set_port_address(device: &impl Device, iface: &str, address: &str) -> Option<String> {
    let answer = device
        .ask(
            "POST",
            "/api/setup/camera/port",
            Some(json!({ "interface": iface, "address": address })),
        )
        .await;
    refusal_of(&answer, "Could not set up this port.")
}

async fn wait_for_port(device: &impl Device, iface: &str, want: &str) {
    for _ in 0..PORT_UP_ATTEMPTS {
        let ports = ports_now(device).await;
        let settled = ports
            .iter()
            .find(|port| port.iface == iface)
            .is_none_or(|port| match want.is_empty() {
                true => port.state != "configuring",
                false => port_has_subnet(port, want),
            });
        if settled {
            return;
        }
        device.pause(PORT_UP_POLL).await;
    }
}

async fn scan(device: &impl Device, id: u64, auto: bool) -> Option<Outcome> {
    let wizard = session().wizard.clone();
    change(id, |held| {
        held.scanning = true;
        held.note = SCANNING_NOTE.into();
        held.outcome = None;
        held.fixing.clear();
    });
    let answer = device.ask("POST", SCAN_PATH, None).await;
    let outcome = flag(&answer, "ok").then(|| {
        scan_outcome(
            cameras_of(&answer["result"], &wizard.name),
            ports_of(&answer["result"]),
            &rtsp_host(&wizard.source),
        )
    });
    change(id, |held| {
        held.scanning = false;
        match &outcome {
            None => held.note = SCAN_FAILED_NOTE.into(),
            Some(found) => {
                held.note = found.note.clone();
                held.adopt = adopted_camera(found, auto, &wizard.rtsp);
                held.outcome = Some(found.clone());
                held.scans += 1;
            }
        }
    });
    outcome
}

async fn find_ports(device: &impl Device, id: u64) {
    let ports = ports_now(device).await;
    change(id, |held| {
        held.no_port = !ports.is_empty()
            && !ports
                .iter()
                .any(|port| !port.uplink && port.kind == "ethernet")
    });
}

async fn fix_port(device: &impl Device, id: u64, iface: String, want: String) {
    change(id, |held| {
        held.fixing
            .insert(iface.clone(), (true, format!("Setting up {iface}…")));
    });
    match set_port_address(device, &iface, &want).await {
        Some(refused) => {
            change(id, |held| {
                held.fixing.insert(iface.clone(), (false, refused));
            });
        }
        None => {
            change(id, |held| {
                held.fixing.insert(
                    iface.clone(),
                    (true, format!("Waiting for {iface} to come up on {want}…")),
                );
            });
            wait_for_port(device, &iface, &want).await;
            scan(device, id, false).await;
        }
    }
}

fn template_done(id: u64, refusal: String, camera: Option<Camera>, name: String) {
    change(id, |held| {
        held.template_busy.clear();
        held.template_note.clear();
        held.found = Found {
            n: held.found.n + 1,
            refusal,
            camera,
            name,
        };
    });
}

fn template_note(id: u64, note: String) {
    change(id, |held| held.template_note = note);
}

async fn set_up_template(device: &impl Device, id: u64, template: Template) {
    let fail = |refusal: String| template_done(id, refusal, None, String::new());
    let known = ports_now(device).await;
    let Some(port) = template_port(&known, &template) else {
        return fail(format!(
            "This device has no Ethernet port to plug the {} into.",
            template.label
        ));
    };
    if !port_has_subnet(&port, &template.cidr) {
        if let Some(holder) = known
            .iter()
            .find(|other| other.iface != port.iface && port_has_subnet(other, &template.cidr))
        {
            return fail(template_holder_error(holder, &template));
        }
        template_note(
            id,
            format!(
                "Putting {} on the {}'s network ({})…",
                port.iface, template.label, template.cidr
            ),
        );
        if let Some(refused) = set_port_address(device, &port.iface, &template.cidr).await {
            return fail(refused);
        }
        wait_for_port(device, &port.iface, &template.cidr).await;
    }
    template_note(
        id,
        format!("Looking for the {} at {}…", template.label, template.host),
    );
    let Some(camera) = scan(device, id, false).await.and_then(|outcome| {
        outcome
            .cams
            .into_iter()
            .find(|cam| cam.host == template.host && cam.path_found)
    }) else {
        return fail(format!(
            "No {} answered at {} — check its Ethernet cable and that it has power, then try again.",
            template.label, template.host
        ));
    };
    let taken = session()
        .wizard
        .others
        .iter()
        .any(|other| other.name.to_lowercase() == template.name);
    template_done(
        id,
        String::new(),
        Some(camera),
        match taken {
            true => String::new(),
            false => template.name.clone(),
        },
    );
    if !template.gimbal.is_empty() {
        let answer = device
            .ask(
                "PUT",
                "/api/gimbal/config",
                Some(json!({ "vendor": template.gimbal, "cameraAddr": template.gimbal_addr })),
            )
            .await;
        if !flag(&answer, "ok") {
            change(id, |held| {
                held.notice = (
                    held.notice.0 + 1,
                    format!(
                        "Gimbal control was not switched to this camera — {}",
                        text(&answer, "reason")
                    ),
                )
            });
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Setup {
    rtsp: String,
    camera: String,
    iface: String,
    ip: String,
    transport: String,
}

fn query(text: &str) -> String {
    url::form_urlencoded::byte_serialize(text.as_bytes()).collect()
}

async fn apply(device: &impl Device, id: u64, job: u64, setup: Setup) {
    let ours = |held: &Session| held.id == id && held.apply_job == job;
    let status = |state: &'static str, text: String, field: String| {
        change_where(ours, |held| {
            held.applying = Applying {
                state,
                text,
                field,
                ..held.applying.clone()
            }
        })
    };
    let (fresh, is_new) = {
        let held = session();
        (held.wizard.is_new && !held.added, held.wizard.is_new)
    };
    let answer = device
        .ask(
            "POST",
            "/api/setup/camera",
            Some(json!({
                "name": setup.camera,
                "new": fresh,
                "rtsp": setup.rtsp,
                "interface": setup.iface,
                "address": setup.ip.trim(),
                "transport": setup.transport,
            })),
        )
        .await;
    if !flag(&answer, "ok") {
        let reason = match answer.get("result") {
            None => "Network error — try again.".to_string(),
            Some(_) => refusal_of(&answer, "Setup failed.").unwrap_or_default(),
        };
        status("failed", reason, text(&answer, "field"));
        return;
    }
    if !change_where(ours, |held| held.added = true)
        || !status("working", VERIFYING.into(), String::new())
    {
        return;
    }
    let path = format!(
        "/api/setup/camera/status?source={}&name={}",
        query(&setup.rtsp),
        query(&setup.camera)
    );
    let finish = |state: &'static str, text: String, url: String| {
        change_where(ours, |held| {
            held.applying = Applying {
                n: held.applying.n + 1,
                state,
                text,
                field: String::new(),
                url,
                camera: setup.camera.clone(),
            }
        })
    };
    let mut reason = String::new();
    for _ in 0..VERIFY_ATTEMPTS {
        let answer = read(device, &path).await;
        match answer.as_ref().map(|it| text(it, "state")).as_deref() {
            Some("live") => {
                if finish("live", VERIFYING.into(), String::new()) {
                    change(id, |held| {
                        held.notice = (
                            held.notice.0 + 1,
                            match is_new {
                                true => format!("Camera {} added.", setup.camera),
                                false => "Stream is live — receiving video from the camera.".into(),
                            },
                        )
                    });
                }
                return;
            }
            Some("needs_auth") => {
                finish(
                    "signIn",
                    answer
                        .as_ref()
                        .map(|it| text(it, "reason"))
                        .unwrap_or_default(),
                    setup.rtsp.clone(),
                );
                return;
            }
            _ => {}
        }
        reason = answer.map_or(reason, |it| text(&it, "reason"));
        if !status("working", VERIFYING.into(), String::new()) {
            return;
        }
        device.pause(VERIFY_POLL).await;
    }
    status(
        "failed",
        Some(reason)
            .filter(|reason| !reason.is_empty())
            .unwrap_or_else(|| "No video yet — check the cable, IP and RTSP URL.".into()),
        String::new(),
    );
}

fn begin(wizard: Wizard) -> u64 {
    let mut held = session();
    let id = held.id + 1;
    *held = Session {
        id,
        wizard,
        ..Session::default()
    };
    WIZARD_DIRTY.store(true, Ordering::Relaxed);
    id
}

fn open(wizard: &Value, scan_now: bool) -> Value {
    let wizard = wizard_of(wizard);
    let is_new = wizard.is_new;
    let id = begin(wizard);
    if is_new {
        launch(move || find_ports_job(id));
    }
    if is_new || scan_now {
        launch(move || scan_job(id, is_new));
    }
    json!({ "ok": true, "result": { "session": id } })
}

async fn find_ports_job(id: u64) {
    find_ports(&Connected, id).await;
}

async fn scan_job(id: u64, auto: bool) {
    scan(&Connected, id, auto).await;
}

async fn fix_port_job(id: u64, iface: String, want: String) {
    fix_port(&Connected, id, iface, want).await;
}

async fn template_job(id: u64, template: Template) {
    set_up_template(&Connected, id, template).await;
}

async fn apply_job(id: u64, job: u64, setup: Setup) {
    apply(&Connected, id, job, setup).await;
}

fn start_template(chosen: &str) -> Value {
    let started = {
        let mut held = session();
        let template = held
            .wizard
            .templates
            .iter()
            .find(|template| template.id == chosen)
            .cloned();
        match (held.template_busy.is_empty(), template) {
            (false, _) => Err("A camera make is being set up.".to_string()),
            (true, None) => Err(format!("{chosen} is not a camera make this device offers.")),
            (true, Some(template)) => {
                held.template_busy = template.id.clone();
                held.template_note =
                    format!("Finding the Ethernet port for the {}…", template.label);
                WIZARD_DIRTY.store(true, Ordering::Relaxed);
                Ok((held.id, template))
            }
        }
    };
    match started {
        Err(reason) => json!({ "ok": false, "reason": reason }),
        Ok((id, template)) => {
            launch(move || template_job(id, template));
            json!({ "ok": true })
        }
    }
}

fn start_apply(setup: Setup) -> Value {
    let (id, job) = {
        let mut held = session();
        held.apply_job += 1;
        held.applying = Applying {
            n: held.applying.n,
            state: "working",
            text: "Applying network and stream settings…".into(),
            ..Applying::default()
        };
        WIZARD_DIRTY.store(true, Ordering::Relaxed);
        (held.id, held.apply_job)
    };
    launch(move || apply_job(id, job, setup));
    json!({ "ok": true })
}

fn restart() -> Value {
    let mut held = session();
    held.apply_job += 1;
    held.added = false;
    held.applying = Applying {
        n: held.applying.n,
        ..Applying::default()
    };
    WIZARD_DIRTY.store(true, Ordering::Relaxed);
    json!({ "ok": true })
}

fn scanned_ports(held: &Session) -> Vec<Port> {
    held.outcome
        .as_ref()
        .map(|outcome| outcome.ports.clone())
        .unwrap_or_default()
}

fn next(typed: &str, typed_name: &str, dup_ack: &str) -> Value {
    let held = session().clone();
    let ports = scanned_ports(&held);
    let next = next_step(
        typed,
        held.wizard.is_new.then_some(typed_name),
        &held.wizard.others,
        &ports,
        dup_ack,
    );
    let (kind, refused) = match &next.step {
        Step::Refuse(refused) => (
            "refuse",
            json!({ "text": refused.text, "warn": refused.warn, "field": refused.field }),
        ),
        Step::Apply => ("apply", Value::Null),
        Step::Port => ("port", Value::Null),
    };
    json!({
        "ok": true,
        "result": {
            "kind": kind,
            "rtsp": next.rtsp,
            "name": next.name,
            "camera": next.name.clone().unwrap_or_else(|| held.wizard.name.clone()),
            "refusal": refused,
            "dupAck": next.dup_ack,
            "iface": port_to_suggest(&ports),
            "ip": suggest_ip(&next.rtsp),
        },
    })
}

fn form(rtsp: &str, iface: &str, ip: &str, current: &str) -> Value {
    let held = session().clone();
    let ports = scanned_ports(&held);
    json!({
        "ok": true,
        "result": {
            "next": next_label(rtsp, &ports, held.wizard.is_new),
            "skipsPort": commits_now(rtsp, &ports),
            "applyNote": apply_note(iface, ip, current),
        },
    })
}

fn camera_json(cam: &Camera) -> Value {
    json!({
        "host": cam.host,
        "url": cam.url,
        "needsAuth": cam.needs_auth,
        "pathFound": cam.path_found,
        "used": cam.used,
        "note": row_note(cam),
        "action": if cam.used.is_empty() { "Use" } else { "Add again" },
    })
}

fn view_of(held: &Session) -> Value {
    let wizard = &held.wizard;
    let host = rtsp_host(&wizard.source);
    let none = Outcome::default();
    let outcome = held.outcome.as_ref().unwrap_or(&none);
    let (cams, ports) = (&outcome.cams, &outcome.ports);
    let quiet = cams.is_empty() && !ports.is_empty();
    let todo: Vec<Value> = outcome
        .todo
        .iter()
        .map(|scanned| {
            let port = &scanned.port;
            let suggest = first_suggest(port);
            let (busy, status) = held.fixing.get(&port.iface).cloned().unwrap_or_default();
            json!({
                "iface": port.iface,
                "label": port.label,
                "detail": port_detail(scanned, &wizard.subnets, &host),
                "open": fixer_open(outcome, &wizard.subnets, port),
                "suggest": suggest,
                "hint": fixer_hint(&suggest),
                "go": fixer_go_label(port, &suggest),
                "options": subnet_options(port, ports, &wizard.subnets).iter().map(|option| json!({
                    "cidr": option.cidr,
                    "text": match option.label.is_empty() {
                        true => option.cidr.clone(),
                        false => format!("{} · {}", option.cidr, option.label),
                    },
                })).collect::<Vec<_>>(),
                "busy": busy,
                "status": status,
            })
        })
        .collect();
    let templates: Vec<Value> = match held.no_port {
        true => Vec::new(),
        false => wizard
            .templates
            .iter()
            .map(|template| {
                let ready = template_ready_port(template, cams, ports);
                json!({
                    "id": template.id,
                    "label": template.label,
                    "sub": template_sub(template, ready),
                    "action": template_action(held.template_busy == template.id, ready),
                })
            })
            .collect(),
    };
    json!({
        "kind": "object",
        "class": "DeviceWizard",
        "session": held.id,
        "scanning": held.scanning,
        "scans": held.scans,
        "adopt": held.adopt.as_ref().map(camera_json),
        "note": if quiet { "" } else { held.note.as_str() },
        "checklist": match quiet && !held.note.is_empty() {
            true => Some(checklist_hint(ports, &wizard.subnets)).filter(|hint| !hint.is_empty()).unwrap_or_else(|| NO_ANSWER_CHECKLIST.into()),
            false => String::new(),
        },
        "cams": cams.iter().map(camera_json).collect::<Vec<_>>(),
        "camHeading": cam_heading(cams),
        "camsHint": if !cams.is_empty() && cams.iter().all(|cam| !cam.used.is_empty()) { ALL_ADDED } else { "" },
        "details": if ports.is_empty() { "" } else if cams.is_empty() { "Details for a technician" } else { "Scan details" },
        "todoHeading": todo_heading(&outcome.todo, &wizard.subnets),
        "todo": todo,
        "rest": outcome.rest.iter().map(|scanned| json!({
            "iface": scanned.port.iface,
            "label": scanned.port.label,
            "detail": port_detail(scanned, &wizard.subnets, &host),
        })).collect::<Vec<_>>(),
        "restTitle": format!("{} — nothing answered", outcome.rest.iter().map(|scanned| scanned.port.label.as_str()).collect::<Vec<_>>().join(", ")),
        "templates": templates,
        "templateBusy": !held.template_busy.is_empty(),
        "templateNote": held.template_note,
        "template": {
            "n": held.found.n,
            "refusal": held.found.refusal,
            "camera": held.found.camera.as_ref().map(camera_json),
            "name": held.found.name,
        },
        "apply": {
            "n": held.applying.n,
            "state": if held.applying.state.is_empty() { "idle" } else { held.applying.state },
            "text": held.applying.text,
            "field": held.applying.field,
            "url": held.applying.url,
            "camera": held.applying.camera,
        },
        "notice": { "n": held.notice.0, "text": held.notice.1 },
    })
}

pub fn device_wizard_view(_backend: &dyn Backend, _args: &[String]) -> Value {
    let held = session().clone();
    view_of(&held)
}

pub fn owns(path: &str) -> bool {
    WIZARD_ACTIONS.contains(&path)
}

pub fn run(path: &str, args: &str) -> Value {
    let given = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    let arg = |index: usize| {
        given
            .get(index)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    match path {
        WIZARD_OPEN => open(
            given.first().unwrap_or(&Value::Null),
            given.get(1).and_then(Value::as_bool).unwrap_or(false),
        ),
        WIZARD_SCAN => {
            let id = session().id;
            launch(move || scan_job(id, false));
            json!({ "ok": true })
        }
        WIZARD_FIX_PORT => {
            let (id, iface, want) = (session().id, arg(0), arg(1).trim().to_string());
            launch(move || fix_port_job(id, iface, want));
            json!({ "ok": true })
        }
        WIZARD_TEMPLATE => start_template(&arg(0)),
        WIZARD_NEXT => next(&arg(0), &arg(1), &arg(2)),
        WIZARD_FORM => form(&arg(0), &arg(1), &arg(2), &arg(3)),
        WIZARD_SIGN_IN => json!({ "ok": true, "result": cam_url(&arg(0), &arg(1), &arg(2)) }),
        WIZARD_APPLY => start_apply(Setup {
            rtsp: arg(0),
            camera: arg(1),
            iface: arg(2),
            ip: arg(3),
            transport: arg(4),
        }),
        WIZARD_RESTART => restart(),
        _ => json!({ "ok": false, "reason": format!("{path} is not a camera wizard action") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn port() -> Port {
        Port {
            iface: "eth0".into(),
            label: "Ethernet port (eth0)".into(),
            kind: "ethernet".into(),
            state: "searched".into(),
            ..Port::default()
        }
    }

    fn neighbor(addr: &str, suggest: &str) -> Neighbor {
        Neighbor {
            addr: addr.into(),
            suggest: suggest.into(),
        }
    }

    fn swept() -> Port {
        Port {
            subnet: "192.168.8.0/24".into(),
            checked: 253,
            ..port()
        }
    }

    fn uplink() -> Port {
        Port {
            iface: "wlan0".into(),
            label: "Wi-Fi (wlan0)".into(),
            kind: "wi-fi".into(),
            subnet: "192.168.1.0/24".into(),
            checked: 253,
            uplink: true,
            ..port()
        }
    }

    fn cam() -> Camera {
        Camera {
            host: "192.168.8.10".into(),
            url: "rtsp://192.168.8.10:554/stream1".into(),
            path_found: true,
            ..Camera::default()
        }
    }

    fn subnets() -> Vec<Subnet> {
        vec![
            Subnet {
                cidr: "192.168.144.2/24".into(),
                label: "SIYI, Topotek gimbals, Herelink".into(),
            },
            Subnet {
                cidr: "192.168.8.2/24".into(),
                label: "LTE routers".into(),
            },
        ]
    }

    fn siyi() -> Template {
        Template {
            id: "siyi".into(),
            label: "SIYI camera".into(),
            hint: "ZT30 — plugged into this device's Ethernet port".into(),
            cidr: "192.168.144.2/24".into(),
            host: "192.168.144.25".into(),
            name: "siyi".into(),
            gimbal: "siyi".into(),
            gimbal_addr: "192.168.144.25:37260".into(),
        }
    }

    fn scanned(port: Port, found: bool) -> Scanned {
        Scanned { port, found }
    }

    fn ifaces(scanned: &[Scanned]) -> Vec<&str> {
        scanned.iter().map(|it| it.port.iface.as_str()).collect()
    }

    #[test]
    fn reads_the_wizard_the_device_serves() {
        let wizard = wizard_of(&json!({
            "name": "cam2", "new": false, "source": "rtsp://192.168.8.10:554/stream", "rtsp": "",
            "others": [{ "name": "cam", "source": "rtsp://x" }],
            "subnets": [{ "cidr": "192.168.144.2/24", "label": "SIYI, Topotek gimbals, Herelink" }, { "cidr": "192.168.8.2/24", "label": "LTE routers" }],
            "templates": [{ "id": "siyi", "label": "SIYI camera", "hint": "ZT30 — plugged into this device's Ethernet port", "cidr": "192.168.144.2/24", "host": "192.168.144.25", "name": "siyi", "gimbal": "siyi", "gimbalAddr": "192.168.144.25:37260" }],
        }));
        assert_eq!(
            wizard.others,
            vec![Other {
                name: "cam".into(),
                source: "rtsp://x".into()
            }]
        );
        assert_eq!(wizard.subnets, subnets());
        assert_eq!(wizard.templates, vec![siyi()]);
        assert!(!wizard.is_new);
        assert_eq!(wizard_of(&Value::Null), Wizard::default());
    }

    #[test]
    fn reads_the_scan_and_a_camera_this_wizard_already_is_reads_as_fresh() {
        let found = json!({
            "cameras": [
                { "host": "192.168.8.4", "port": 554, "url": "rtsp://192.168.8.4:554/s", "pathFound": true, "usedBy": "cam" },
                { "host": "192.168.8.5", "port": 554, "url": "rtsp://192.168.8.5:554/s", "pathFound": true, "usedBy": "cam2" },
            ],
            "ports": [{ "interface": "eth1", "label": "Ethernet port (eth1)", "type": "ethernet", "state": "searched", "neighbors": [{ "addr": "192.168.144.25", "mac": "x", "suggest": "192.168.144.2/24" }] }],
        });
        assert_eq!(
            cameras_of(&found, "cam2")
                .iter()
                .map(|it| it.used.as_str())
                .collect::<Vec<_>>(),
            vec!["cam", ""]
        );
        let first = &ports_of(&found)[0];
        assert_eq!(first.iface, "eth1");
        assert_eq!(
            first.neighbors,
            vec![neighbor("192.168.144.25", "192.168.144.2/24")]
        );
    }

    #[test]
    fn takes_the_host_out_of_a_camera_url() {
        assert_eq!(rtsp_host("rtsp://192.168.8.10:554/stream"), "192.168.8.10");
        assert_eq!(rtsp_host("rtsp://cam.local/live"), "cam.local");
        assert_eq!(rtsp_host("not a url"), "");
        assert_eq!(rtsp_host(""), "");
    }

    #[test]
    fn supplies_the_scheme_and_never_rewrites_one_that_is_there() {
        assert_eq!(normalize_rtsp("192.168.8.10"), "rtsp://192.168.8.10");
        assert_eq!(
            normalize_rtsp("cam.local:554/live"),
            "rtsp://cam.local:554/live"
        );
        assert_eq!(
            normalize_rtsp("  192.168.8.10/stream  "),
            "rtsp://192.168.8.10/stream"
        );
        assert_eq!(
            normalize_rtsp("rtsp://192.168.8.10/x"),
            "rtsp://192.168.8.10/x"
        );
        assert_eq!(normalize_rtsp("http://192.168.8.10"), "http://192.168.8.10");
        assert_eq!(normalize_rtsp(""), "");
    }

    #[test]
    fn never_offers_the_address_the_camera_already_answers_on() {
        assert_eq!(suggest_ip("rtsp://192.168.8.2/live"), "192.168.8.3/24");
        assert_eq!(suggest_ip("rtsp://192.168.8.10/live"), "192.168.8.2/24");
        assert_eq!(suggest_ip("rtsp://cam.local/live"), "");
        assert_eq!(suggest_ip("garbage"), "");
    }

    #[test]
    fn spots_a_subnet_typed_where_a_camera_url_belongs() {
        assert_eq!(cidr_in("192.168.8.0/24"), "192.168.8.0/24");
        assert_eq!(cidr_in(" rtsp://192.168.8.0/24 "), "192.168.8.0/24");
        assert_eq!(cidr_in("192.168.8.10"), "");
        assert_eq!(cidr_in("rtsp://192.168.8.10/stream"), "");
        assert_eq!(cidr_in("192.168.8.0/33"), "");
    }

    #[test]
    fn refuses_anything_that_is_not_four_plain_octets() {
        assert_eq!(ip_to_int("0.0.0.0"), Some(0));
        assert_eq!(ip_to_int("255.255.255.255"), Some(4294967295));
        assert_eq!(ip_to_int("192.168.1.1"), Some(3232235777));
        assert_eq!(ip_to_int("1.2.3"), None);
        assert_eq!(ip_to_int("1.2.3.256"), None);
        assert_eq!(ip_to_int("1.2.3."), None);
        assert_eq!(ip_to_int("1.2.3. 4"), None);
        assert_eq!(ip_to_int("1.2.3.0x4"), None);
        assert_eq!(ip_to_int("1.2.3.+4"), None);
    }

    #[test]
    fn masks_both_sides_before_comparing_and_is_false_on_junk() {
        assert!(same_subnet("192.168.8.10", "192.168.8.0/24"));
        assert!(!same_subnet("192.168.9.10", "192.168.8.0/24"));
        assert!(same_subnet("192.168.8.10", "192.168.8.3/24"));
        assert!(same_subnet("10.1.2.3", "10.0.0.0/8"));
        assert!(same_subnet("1.2.3.4", "255.255.255.255/0"));
        assert!(!same_subnet("192.168.8.11", "192.168.8.10/32"));
        assert!(!same_subnet("cam.local", "192.168.8.0/24"));
        assert!(!same_subnet("192.168.8.10", "not-a-subnet"));
        assert!(!same_subnet("192.168.8.10", "192.168.8.0/33"));
        assert!(!same_subnet("", ""));
    }

    #[test]
    fn names_a_port_by_what_is_wrong_with_it_not_by_a_missing_camera() {
        assert_eq!(
            port_label(&swept()),
            "checked 253 addresses on 192.168.8.0/24 — nothing answered"
        );
        assert_eq!(
            port_label(&Port {
                state: "no-carrier".into(),
                ..port()
            }),
            "nothing plugged in — check the cable and the camera's power"
        );
        assert_eq!(
            port_label(&Port {
                state: "no-carrier".into(),
                kind: "wi-fi".into(),
                ..port()
            }),
            "not connected to any network"
        );
        assert!(
            port_label(&Port {
                state: "failed".into(),
                reason: "address-unusable".into(),
                tried_address: "192.168.8.4/24".into(),
                ..port()
            })
            .starts_with("192.168.8.4/24 could not be used")
        );
        assert_eq!(
            port_label(&Port {
                subnet: "192.168.1.0/24".into(),
                neighbors: vec![neighbor("192.168.144.25", "192.168.144.2/24")],
                ..port()
            }),
            "a device answered at 192.168.144.25 (not confirmed as a camera), not on 192.168.1.0/24 — set this port to 192.168.144.2/24 to reach it"
        );
        assert_eq!(
            port_detail(&scanned(uplink(), false), &subnets(), ""),
            "checked 253 addresses on 192.168.1.0/24 — nothing answered — this is the device's own uplink"
        );
        assert!(
            port_detail(&scanned(swept(), false), &subnets(), "192.168.8.10")
                .contains("nothing answered at 192.168.8.10 on eth0")
        );
        assert_eq!(
            port_detail(&scanned(swept(), true), &subnets(), ""),
            "camera found here"
        );
    }

    #[test]
    fn a_router_on_a_port_already_set_for_the_camera_asks_for_the_cable_not_an_address() {
        let idle = Port {
            subnet: "192.168.144.0/24".into(),
            neighbors: vec![neighbor("192.168.1.3", "")],
            ..port()
        };
        let outcome = scan_outcome(vec![], vec![idle.clone(), uplink()], "");
        assert_eq!(todo_heading(&outcome.todo, &subnets()), "Check the cable");
        assert!(port_detail(&outcome.todo[0], &subnets(), "").contains("Plug the camera in here"));
        assert!(!fixer_open(&outcome, &subnets(), &idle));
        assert!(
            checklist_hint(std::slice::from_ref(&idle), &subnets())
                .contains("looks like a router or switch")
        );
        assert_eq!(
            subnet_options(&idle, &[idle.clone(), uplink()], &subnets())
                .iter()
                .map(|it| it.cidr.as_str())
                .collect::<Vec<_>>(),
            vec!["192.168.144.2/24", "192.168.8.2/24"]
        );
    }

    #[test]
    fn scan_results_lead_with_the_port_that_has_a_next_step() {
        let blank = Port {
            state: "no-address".into(),
            ..port()
        };
        let outcome = scan_outcome(vec![], vec![uplink(), blank.clone()], "");
        assert_eq!(ifaces(&outcome.todo), vec!["eth0"]);
        assert_eq!(ifaces(&outcome.rest), vec!["wlan0"]);
        assert_eq!(todo_heading(&outcome.todo, &subnets()), "Try this");
        assert_eq!(outcome.note, "No camera answered.");
        assert!(fixer_open(&outcome, &subnets(), &blank));
        assert_eq!(first_suggest(&blank), "");
        assert_eq!(
            scan_outcome(vec![], vec![], "").note,
            "No camera port on this device — type the camera's address below."
        );
    }

    #[test]
    fn a_port_with_nothing_left_to_do_stops_competing_once_a_new_camera_answers() {
        let outcome = scan_outcome(
            vec![cam()],
            vec![
                swept(),
                Port {
                    iface: "eth1".into(),
                    state: "no-address".into(),
                    ..port()
                },
            ],
            "",
        );
        assert!(outcome.todo.is_empty());
        assert_eq!(
            adopted_camera(&outcome, true, "").map(|it| it.url),
            Some("rtsp://192.168.8.10:554/stream1".into())
        );
    }

    #[test]
    fn only_one_fresh_camera_with_a_path_is_adopted_and_never_over_a_typed_address() {
        let adopted = |cams: Vec<Camera>, auto: bool, typed: &str| {
            adopted_camera(&scan_outcome(cams, vec![swept()], ""), auto, typed).map(|it| it.host)
        };
        assert_eq!(
            adopted(vec![cam()], true, "rtsp://192.168.144.25:8554/main.264"),
            None
        );
        assert_eq!(
            adopted(
                vec![Camera {
                    path_found: false,
                    ..cam()
                }],
                false,
                ""
            ),
            None
        );
        assert_eq!(
            adopted(
                vec![Camera {
                    used: "cam".into(),
                    ..cam()
                }],
                false,
                ""
            ),
            None
        );
        assert_eq!(
            adopted(
                vec![
                    cam(),
                    Camera {
                        host: "192.168.8.11".into(),
                        ..cam()
                    }
                ],
                false,
                ""
            ),
            None
        );
        assert_eq!(
            adopted(
                vec![
                    Camera {
                        used: "cam".into(),
                        ..cam()
                    },
                    Camera {
                        host: "192.168.144.25".into(),
                        ..cam()
                    }
                ],
                false,
                ""
            ),
            Some("192.168.144.25".into())
        );
    }

    #[test]
    fn the_subnet_chips_put_what_was_measured_first_and_never_a_subnet_another_port_holds() {
        let failed = Port {
            state: "failed".into(),
            reason: "address-unusable".into(),
            tried_address: "192.168.8.4/24".into(),
            neighbors: vec![neighbor("192.168.144.25", "192.168.144.2/24")],
            ..port()
        };
        assert_eq!(
            subnet_options(&failed, std::slice::from_ref(&failed), &subnets()),
            vec![
                Subnet {
                    cidr: "192.168.144.2/24".into(),
                    label: "found 192.168.144.25".into()
                },
                Subnet {
                    cidr: "192.168.8.4/24".into(),
                    label: "tried before".into()
                },
                Subnet {
                    cidr: "192.168.8.2/24".into(),
                    label: "LTE routers".into()
                },
            ]
        );
        let outcome = scan_outcome(vec![], vec![failed.clone()], "");
        assert!(fixer_open(&outcome, &subnets(), &failed));
        assert_eq!(first_suggest(&failed), "192.168.144.2/24");
        assert_eq!(
            fixer_go_label(&failed, &first_suggest(&failed)),
            "Set up eth0 and find the camera"
        );
        assert_eq!(fixer_go_label(&failed, ""), "Set up & scan again");
    }

    #[test]
    fn a_camera_make_picks_the_port_that_can_carry_its_subnet_and_says_when_another_holds_it() {
        assert_eq!(
            template_port(
                &[
                    uplink(),
                    Port {
                        state: "no-carrier".into(),
                        ..port()
                    },
                    Port {
                        iface: "eth1".into(),
                        state: "no-address".into(),
                        ..port()
                    }
                ],
                &siyi()
            )
            .map(|it| it.iface),
            Some("eth1".into())
        );
        assert_eq!(template_port(&[uplink()], &siyi()), None);
        let ready = Port {
            subnet: "192.168.144.0/24".into(),
            ..port()
        };
        let held = std::slice::from_ref(&ready);
        assert_eq!(
            template_ready_port(&siyi(), &[], held).map(|it| it.iface.as_str()),
            Some("eth0")
        );
        assert_eq!(
            template_ready_port(
                &siyi(),
                &[Camera {
                    host: "192.168.144.25".into(),
                    ..cam()
                }],
                held
            ),
            None
        );
        assert_eq!(
            template_sub(&siyi(), Some(&ready)),
            "The Ethernet port (eth0) is ready for a SIYI camera — nothing answered at 192.168.144.25. Check the cable."
        );
        assert_eq!(template_sub(&siyi(), None), siyi().hint);
        assert_eq!(template_action(false, Some(&ready)), "Scan again");
        assert_eq!(template_action(true, Some(&ready)), "Setting up…");
        assert!(
            template_holder_error(
                &uplink(),
                &Template {
                    label: "OpenIPC camera".into(),
                    cidr: "192.168.1.2/24".into(),
                    ..siyi()
                }
            )
            .contains("Wi-Fi (wlan0) already uses 192.168.1.x")
        );
    }

    #[test]
    fn a_sign_in_goes_into_the_url_encoded_and_leaves_with_the_camera() {
        assert_eq!(
            cam_url("rtsp://192.168.8.14:554/live", "admin", "p@ss word"),
            "rtsp://admin:p%40ss%20word@192.168.8.14:554/live"
        );
        assert_eq!(
            cam_url("rtsp://192.168.8.14:554/live", " admin ", ""),
            "rtsp://admin@192.168.8.14:554/live"
        );
        assert_eq!(
            cam_url("rtsp://192.168.8.14:554/live", "", ""),
            "rtsp://192.168.8.14:554/live"
        );
    }

    #[test]
    fn a_camera_on_a_working_port_skips_the_port_question() {
        let ports = [swept()];
        assert!(commits_now("rtsp://192.168.8.10:554/stream1", &ports));
        assert_eq!(
            next_label("rtsp://192.168.8.10:554/stream1", &ports, false),
            "Set up & verify"
        );
        assert_eq!(
            next_label("rtsp://192.168.8.10:554/stream1", &ports, true),
            "Add camera"
        );
        assert_eq!(
            next_label("rtsp://192.168.50.99:554/s", &ports, true),
            "Continue"
        );
        assert_eq!(next_label("rpiCamera", &[], true), "Add camera");
        assert_eq!(
            next_step("rtsp://192.168.8.10:554/stream1", None, &[], &ports, "").step,
            Step::Apply
        );
        assert_eq!(
            next_step("192.168.50.99:554/s", None, &[], &ports, ""),
            Next {
                step: Step::Port,
                rtsp: "rtsp://192.168.50.99:554/s".into(),
                name: None,
                dup_ack: String::new()
            }
        );
    }

    #[test]
    fn continue_refuses_what_cannot_work_and_says_where_it_belongs() {
        let others = [Other {
            name: "cam".into(),
            source: "rtsp://192.168.8.10:554/stream".into(),
        }];
        let refused = |rtsp: &str, name: Option<&str>, dup_ack: &str| {
            let next = next_step(rtsp, name, &others, &[], dup_ack);
            match next.step {
                Step::Refuse(refusal) => Some((refusal, next.dup_ack)),
                _ => None,
            }
        };
        assert!(
            refused("192.168.144.2/24", None, "")
                .unwrap()
                .0
                .text
                .contains("address for this device's port, not the camera")
        );
        assert_eq!(
            refused("rtsp://192.168.144.2/24", None, "")
                .unwrap()
                .0
                .field,
            Some("rtsp")
        );
        assert_eq!(
            refused("rtsp://192.168.8.11/s", Some(" "), "").unwrap().0,
            Refusal {
                text: "Give the camera a name.".into(),
                warn: false,
                field: Some("name")
            }
        );
        assert_eq!(
            refused("rtsp://192.168.8.11/s", Some("CAM"), "")
                .unwrap()
                .0
                .text,
            "A camera named “cam” already exists — pick another name."
        );
        assert!(
            refused("ftp://x", None, "")
                .unwrap()
                .0
                .text
                .contains("Enter the camera's address")
        );
        let (duplicate, ack) = refused("rtsp://192.168.8.10:554/stream", None, "").unwrap();
        assert!(duplicate.warn);
        assert_eq!(ack, "rtsp://192.168.8.10:554/stream");
        assert_eq!(
            refused(
                "rtsp://192.168.8.10:554/stream",
                None,
                "rtsp://192.168.8.10:554/stream"
            ),
            None
        );
        assert_eq!(
            next_step("rpiCamera", Some("picam"), &[], &[], ""),
            Next {
                step: Step::Apply,
                rtsp: "rpiCamera".into(),
                name: Some("picam".into()),
                dup_ack: String::new()
            }
        );
    }

    #[test]
    fn the_apply_note_says_whether_the_port_changes() {
        assert_eq!(
            apply_note("eth0", "192.168.8.2/24", "192.168.8.2/24"),
            "eth0 keeps 192.168.8.2/24 — nothing on that port changes."
        );
        assert_eq!(
            apply_note("eth0", "192.168.50.2/24", ""),
            "Sets eth0 to 192.168.50.2/24 — anything else on that port may need reconnecting."
        );
        assert_eq!(apply_note("", "192.168.50.2/24", ""), "");
    }

    struct Scripted {
        answers: RefCell<Vec<(String, Value)>>,
        asked: RefCell<Vec<String>>,
        paused: Cell<usize>,
    }

    fn scripted(answers: Vec<(&str, Value)>) -> Scripted {
        Scripted {
            answers: RefCell::new(
                answers
                    .into_iter()
                    .map(|(asked, answer)| (asked.to_string(), answer))
                    .collect(),
            ),
            asked: RefCell::new(Vec::new()),
            paused: Cell::new(0),
        }
    }

    impl Device for Scripted {
        async fn ask(&self, method: &str, path: &str, body: Option<Value>) -> Value {
            let request = format!("{method} {path}");
            self.asked.borrow_mut().push(match body {
                Some(body) => format!("{request} {body}"),
                None => request.clone(),
            });
            let at = self
                .answers
                .borrow()
                .iter()
                .position(|(asked, _)| *asked == request);
            at.map(|at| self.answers.borrow_mut().remove(at).1)
                .unwrap_or_else(
                    || json!({ "ok": false, "reason": "The device did not answer.", "field": "" }),
                )
        }

        async fn pause(&self, _wait: Duration) {
            self.paused.set(self.paused.get() + 1);
        }
    }

    fn answered(result: Value) -> Value {
        json!({ "ok": true, "result": result })
    }

    fn refused_with(body: Value) -> Value {
        json!({ "ok": false, "reason": "The device refused (422).", "field": text(&body, "field"), "result": body })
    }

    fn ports_answer(ports: Value) -> Value {
        answered(json!({ "ports": ports }))
    }

    fn wizard(is_new: bool) -> Wizard {
        Wizard {
            name: if is_new { String::new() } else { "cam".into() },
            is_new,
            subnets: subnets(),
            others: vec![Other {
                name: "siyi".into(),
                source: "rtsp://other".into(),
            }],
            templates: vec![siyi()],
            ..Wizard::default()
        }
    }

    fn shown() -> Value {
        view_of(&session().clone())
    }

    fn quietly<T>(job: impl Future<Output = T>) -> T {
        crate::ubxbase::block_on(job)
    }

    #[test]
    fn a_scan_fills_the_view_and_adopts_the_one_new_camera() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let id = begin(wizard(true));
        let device = scripted(vec![(
            "POST /api/streams/scan",
            answered(json!({
                "cameras": [{ "host": "192.168.8.10", "url": "rtsp://192.168.8.10:554/stream1", "pathFound": true }],
                "ports": [{ "interface": "eth0", "label": "Ethernet port (eth0)", "type": "ethernet", "state": "searched", "subnet": "192.168.8.0/24", "checked": 253 }],
            })),
        )]);
        quietly(scan(&device, id, true));
        let view = shown();
        assert_eq!(
            (
                view["session"].as_u64(),
                view["scans"].as_u64(),
                view["scanning"].as_bool()
            ),
            (Some(id), Some(1), Some(false))
        );
        assert_eq!(view["adopt"]["url"], "rtsp://192.168.8.10:554/stream1");
        assert_eq!(view["cams"][0]["note"], "ready to use");
        assert_eq!(view["camHeading"], "1 camera");
        assert_eq!(view["details"], "Scan details");
        assert_eq!(view["rest"][0]["detail"], "camera found here");
        assert_eq!(view["checklist"], "");
        assert_eq!(view["templates"][0]["action"], "Set up");
        assert!(take_changed());
        let failed = scripted(vec![]);
        quietly(scan(&failed, id, false));
        let view = shown();
        assert_eq!(view["note"], SCAN_FAILED_NOTE);
        assert_eq!(
            view["scans"], 1,
            "a scan that never reached the device is no new result"
        );
        assert_eq!(view["cams"], json!([]));
    }

    #[test]
    fn a_silent_scan_shows_the_checklist_instead_of_the_note() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let id = begin(wizard(true));
        let device = scripted(vec![(
            "POST /api/streams/scan",
            answered(json!({
                "cameras": [],
                "ports": [{ "interface": "eth0", "label": "Ethernet port (eth0)", "type": "ethernet", "state": "no-carrier" }],
            })),
        )]);
        quietly(scan(&device, id, true));
        let view = shown();
        assert_eq!(view["note"], "");
        assert_eq!(
            view["checklist"],
            "Nothing is plugged into the Ethernet port (eth0)."
        );
        assert_eq!(view["details"], "Details for a technician");
        assert_eq!(view["todoHeading"], "Try this");
        assert_eq!(view["todo"][0]["open"], true);
        assert_eq!(view["adopt"], Value::Null);
    }

    #[test]
    fn fixing_a_port_sets_its_address_waits_for_it_and_scans_again() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let id = begin(wizard(true));
        let device = scripted(vec![
            ("POST /api/setup/camera/port", answered(json!({}))),
            (
                "GET /api/streams/scan/ports",
                ports_answer(json!([{ "interface": "eth0", "state": "configuring" }])),
            ),
            (
                "GET /api/streams/scan/ports",
                ports_answer(
                    json!([{ "interface": "eth0", "state": "searched", "subnet": "192.168.144.0/24" }]),
                ),
            ),
            (
                "POST /api/streams/scan",
                answered(json!({ "cameras": [], "ports": [] })),
            ),
        ]);
        quietly(fix_port(
            &device,
            id,
            "eth0".into(),
            "192.168.144.2/24".into(),
        ));
        assert_eq!(
            device.asked.borrow().as_slice(),
            [
                r#"POST /api/setup/camera/port {"address":"192.168.144.2/24","interface":"eth0"}"#,
                "GET /api/streams/scan/ports",
                "GET /api/streams/scan/ports",
                "POST /api/streams/scan",
            ]
        );
        assert_eq!(device.paused.get(), 1);
        assert_eq!(shown()["scans"], 1);
        let refused = scripted(vec![(
            "POST /api/setup/camera/port",
            refused_with(json!({ "error": "eth0 is the uplink" })),
        )]);
        quietly(fix_port(&refused, id, "eth0".into(), "10.0.0.2/24".into()));
        assert_eq!(
            session().fixing.get("eth0"),
            Some(&(false, "eth0 is the uplink".to_string()))
        );
    }

    #[test]
    fn a_camera_make_moves_the_port_finds_the_camera_and_points_the_gimbal_at_it() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let id = begin(wizard(true));
        let device = scripted(vec![
            (
                "GET /api/streams/scan/ports",
                ports_answer(
                    json!([{ "interface": "eth0", "type": "ethernet", "state": "searched", "subnet": "192.168.8.0/24" }]),
                ),
            ),
            ("POST /api/setup/camera/port", answered(json!({}))),
            (
                "GET /api/streams/scan/ports",
                ports_answer(
                    json!([{ "interface": "eth0", "type": "ethernet", "state": "searched", "subnet": "192.168.144.0/24" }]),
                ),
            ),
            (
                "POST /api/streams/scan",
                answered(
                    json!({ "cameras": [{ "host": "192.168.144.25", "url": "rtsp://192.168.144.25:8554/main.264", "pathFound": true }], "ports": [] }),
                ),
            ),
            ("PUT /api/gimbal/config", refused_with(json!({}))),
        ]);
        quietly(set_up_template(&device, id, siyi()));
        let view = shown();
        assert_eq!(view["template"]["n"], 1);
        assert_eq!(view["template"]["refusal"], "");
        assert_eq!(
            view["template"]["camera"]["url"],
            "rtsp://192.168.144.25:8554/main.264"
        );
        assert_eq!(
            view["template"]["name"], "",
            "siyi is taken, so the typed name stays"
        );
        assert_eq!(view["templateBusy"], false);
        assert_eq!(
            view["notice"],
            json!({ "n": 1, "text": "Gimbal control was not switched to this camera — The device refused (422)." })
        );
        assert!(device.asked.borrow()[4].starts_with(
            r#"PUT /api/gimbal/config {"cameraAddr":"192.168.144.25:37260","vendor":"siyi"}"#
        ));
        let held = scripted(vec![(
            "GET /api/streams/scan/ports",
            ports_answer(json!([
                { "interface": "wlan0", "type": "wi-fi", "subnet": "192.168.144.0/24", "isUplink": true, "label": "Wi-Fi (wlan0)" },
                { "interface": "eth0", "type": "ethernet", "state": "no-address" },
            ])),
        )]);
        quietly(set_up_template(&held, id, siyi()));
        let view = shown();
        assert_eq!(view["template"]["n"], 2);
        assert!(
            view["template"]["refusal"]
                .as_str()
                .unwrap()
                .starts_with("The Wi-Fi (wlan0) already uses 192.168.144.x")
        );
        assert_eq!(view["template"]["camera"], Value::Null);
        quietly(set_up_template(&scripted(vec![]), id, siyi()));
        assert_eq!(
            shown()["template"]["refusal"],
            "This device has no Ethernet port to plug the SIYI camera into."
        );
    }

    #[test]
    fn applying_waits_for_the_picture_and_says_what_it_saw() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let id = begin(wizard(true));
        let setup = Setup {
            rtsp: "rtsp://192.168.8.10:554/stream1".into(),
            camera: "cam2".into(),
            transport: "auto".into(),
            ..Setup::default()
        };
        let status = "GET /api/setup/camera/status?source=rtsp%3A%2F%2F192.168.8.10%3A554%2Fstream1&name=cam2";
        let device = scripted(vec![
            ("POST /api/setup/camera", answered(json!({}))),
            (
                status,
                answered(json!({ "state": "starting", "reason": "connecting" })),
            ),
            (status, answered(json!({ "state": "live" }))),
        ]);
        quietly(apply(&device, id, 0, setup.clone()));
        assert!(device.asked.borrow()[0].contains(r#""new":true"#));
        assert_eq!(
            shown()["apply"],
            json!({ "n": 1, "state": "live", "text": VERIFYING, "field": "", "url": "", "camera": "cam2" })
        );
        assert_eq!(
            shown()["notice"],
            json!({ "n": 1, "text": "Camera cam2 added." })
        );
        let again = scripted(vec![
            ("POST /api/setup/camera", answered(json!({}))),
            (
                status,
                answered(
                    json!({ "state": "needs_auth", "reason": "The camera asked for a sign-in." }),
                ),
            ),
        ]);
        quietly(apply(&again, id, 0, setup.clone()));
        assert!(
            again.asked.borrow()[0].contains(r#""new":false"#),
            "a camera added once is reconfigured, not added twice"
        );
        let shown_apply = shown()["apply"].clone();
        assert_eq!(
            (
                shown_apply["n"].as_u64(),
                shown_apply["state"].as_str(),
                shown_apply["url"].as_str()
            ),
            (
                Some(2),
                Some("signIn"),
                Some("rtsp://192.168.8.10:554/stream1")
            )
        );
        let silent = scripted(vec![
            ("POST /api/setup/camera", answered(json!({}))),
            (
                status,
                answered(json!({ "state": "starting", "reason": "no packets yet" })),
            ),
        ]);
        quietly(apply(&silent, id, 0, setup.clone()));
        assert_eq!(silent.paused.get(), VERIFY_ATTEMPTS);
        assert_eq!(
            (
                shown()["apply"]["state"].as_str(),
                shown()["apply"]["text"].as_str()
            ),
            (Some("failed"), Some("no packets yet"))
        );
        let refused = scripted(vec![(
            "POST /api/setup/camera",
            refused_with(json!({ "error": "That name is taken", "field": "name" })),
        )]);
        quietly(apply(&refused, id, 0, setup.clone()));
        assert_eq!(
            (
                shown()["apply"]["text"].as_str(),
                shown()["apply"]["field"].as_str()
            ),
            (Some("That name is taken"), Some("name"))
        );
        quietly(apply(&scripted(vec![]), id, 0, setup.clone()));
        assert_eq!(shown()["apply"]["text"], "Network error — try again.");
        restart();
        let stale = scripted(vec![("POST /api/setup/camera", answered(json!({})))]);
        quietly(apply(&stale, id, 0, setup));
        assert_eq!(
            shown()["apply"]["state"],
            "idle",
            "a run from before Start over changes nothing"
        );
    }

    #[test]
    fn the_wizard_is_driven_through_its_actions() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let opened = run(
            WIZARD_OPEN,
            &json!([{ "name": "cam", "new": false, "others": [{ "name": "front", "source": "rtsp://192.168.8.10:554/stream" }] }, false]).to_string(),
        );
        let id = opened["result"]["session"].as_u64().unwrap();
        assert_eq!(session().id, id);
        assert_eq!(shown()["apply"]["state"], "idle");
        change(id, |held| {
            held.outcome = Some(scan_outcome(vec![], vec![swept()], ""))
        });
        assert_eq!(
            run(
                WIZARD_FORM,
                r#"["rtsp://192.168.8.10:554/stream1","eth0","192.168.8.2/24","192.168.8.2/24"]"#
            )["result"],
            json!({ "next": "Set up & verify", "skipsPort": true, "applyNote": "eth0 keeps 192.168.8.2/24 — nothing on that port changes." })
        );
        let next = run(WIZARD_NEXT, r#"["192.168.50.99/s","ignored",""]"#)["result"].clone();
        assert_eq!(
            next,
            json!({ "kind": "port", "rtsp": "rtsp://192.168.50.99/s", "name": null, "camera": "cam", "refusal": null, "dupAck": "", "iface": "", "ip": "192.168.50.2/24" })
        );
        let duplicate =
            run(WIZARD_NEXT, r#"["rtsp://192.168.8.10:554/stream","",""]"#)["result"].clone();
        assert_eq!(
            (
                duplicate["kind"].as_str(),
                duplicate["refusal"]["warn"].as_bool(),
                duplicate["dupAck"].as_str()
            ),
            (
                Some("refuse"),
                Some(true),
                Some("rtsp://192.168.8.10:554/stream")
            )
        );
        assert_eq!(
            run(WIZARD_SIGN_IN, r#"["rtsp://10.0.0.5/live","admin","pw"]"#)["result"],
            "rtsp://admin:pw@10.0.0.5/live"
        );
        assert_eq!(run(WIZARD_TEMPLATE, r#"["nope"]"#)["ok"], false);
        assert_eq!(run("deviceWizard.nope", "[]")["ok"], false);
        assert!(owns(WIZARD_APPLY) && !owns("device.call"));
    }
}
