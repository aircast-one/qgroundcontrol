use std::ops::RangeInclusive;

use serde_json::{Value, json};

pub const TELEMETRY_ENDPOINTS: &str = "deviceTelemetry.endpoints";
pub const ICE_SERVERS: &str = "deviceNetwork.iceServers";

const UNKNOWN_SCHEME_HINT: &str =
    "Set outside this page — saved exactly as it stands unless you change the type.";
const UNSAVED: &str = "It is not saved until it has one.";
const FALLBACK_SCHEME: &str = "udps";
const FALLBACK_ENDPOINT_PORT: &str = "14550";
const FALLBACK_HOST_LABEL: &str = "Address";
const ENDPOINT_PORTS: RangeInclusive<u32> = 1..=65535;

#[derive(Clone, Debug, PartialEq)]
struct EndpointRow {
    scheme: String,
    host: String,
    port: String,
}

fn text<'a>(json: &'a Value, key: &str) -> &'a str {
    json.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn row(scheme: &str, host: &str, port: &str) -> EndpointRow {
    EndpointRow {
        scheme: scheme.to_string(),
        host: host.to_string(),
        port: port.to_string(),
    }
}

fn scheme_of<'a>(schemes: &'a [Value], name: &str) -> Option<&'a Value> {
    schemes.iter().find(|it| text(it, "value") == name)
}

fn default_host<'a>(schemes: &'a [Value], name: &str) -> &'a str {
    scheme_of(schemes, name).map_or("", |it| text(it, "host"))
}

fn default_port<'a>(schemes: &'a [Value], name: &str) -> &'a str {
    scheme_of(schemes, name).map_or(FALLBACK_ENDPOINT_PORT, |it| text(it, "port"))
}

fn new_row(schemes: &[Value], name: &str) -> EndpointRow {
    row(
        name,
        default_host(schemes, name),
        default_port(schemes, name),
    )
}

fn blank_row(schemes: &[Value]) -> EndpointRow {
    new_row(
        schemes,
        schemes
            .first()
            .map_or(FALLBACK_SCHEME, |it| text(it, "value")),
    )
}

fn parse_spec(spec: &str) -> Option<EndpointRow> {
    let (scheme, address) = spec.split_once(':')?;
    let (host, port) = address.rsplit_once(':').unwrap_or((address, ""));
    Some(row(scheme, host, port))
}

fn parse_specs(text: &str) -> Vec<EndpointRow> {
    text.split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(parse_spec)
        .filter(|row| !row.scheme.is_empty())
        .collect()
}

fn retarget(schemes: &[Value], typed: &EndpointRow) -> EndpointRow {
    let host = typed.host.trim();
    let untouched = host.is_empty()
        || schemes.iter().any(|it| {
            let default = text(it, "host");
            !default.is_empty() && default == host
        });
    EndpointRow {
        scheme: typed.scheme.clone(),
        host: match untouched {
            true => default_host(schemes, &typed.scheme).to_string(),
            false => typed.host.clone(),
        },
        port: match typed.port.trim().is_empty() {
            true => default_port(schemes, &typed.scheme).to_string(),
            false => typed.port.clone(),
        },
    }
}

fn port_in_range(port: &str) -> bool {
    port.chars().all(|c| c.is_ascii_digit())
        && port
            .parse::<u32>()
            .is_ok_and(|port| ENDPOINT_PORTS.contains(&port))
}

fn endpoint_problem(schemes: &[Value], typed: &EndpointRow) -> Option<String> {
    let (host, port) = (typed.host.trim(), typed.port.trim());
    if host.is_empty() && port.is_empty() {
        None
    } else if !port_in_range(port) {
        Some(format!(
            "Needs a port between {} and {}.",
            ENDPOINT_PORTS.start(),
            ENDPOINT_PORTS.end()
        ))
    } else if host.is_empty() && default_host(schemes, &typed.scheme).is_empty() {
        Some("Needs an address.".to_string())
    } else {
        None
    }
}

fn serialize_endpoints(schemes: &[Value], rows: &[EndpointRow]) -> String {
    rows.iter()
        .filter(|typed| {
            endpoint_problem(schemes, typed).is_none()
                && !(typed.host.trim().is_empty() && typed.port.trim().is_empty())
        })
        .map(|typed| {
            let host = match typed.host.trim() {
                "" => default_host(schemes, &typed.scheme),
                host => host,
            };
            format!("{}:{host}:{}", typed.scheme, typed.port.trim())
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn row_json(typed: &EndpointRow) -> Value {
    json!({ "scheme": typed.scheme, "host": typed.host, "port": typed.port })
}

fn option(schemes: &[Value], typed: &EndpointRow, value: &str, label: &str) -> Value {
    let picked = EndpointRow {
        scheme: value.to_string(),
        ..typed.clone()
    };
    json!({ "value": value, "label": label, "row": row_json(&retarget(schemes, &picked)) })
}

fn row_view(schemes: &[Value], typed: &EndpointRow) -> Value {
    let scheme = scheme_of(schemes, &typed.scheme);
    let unknown = (scheme.is_none() && !typed.scheme.is_empty())
        .then(|| option(schemes, typed, &typed.scheme, &typed.scheme));
    let options: Vec<Value> = schemes
        .iter()
        .map(|it| option(schemes, typed, text(it, "value"), text(it, "label")))
        .chain(unknown)
        .collect();
    json!({
        "scheme": typed.scheme,
        "host": typed.host,
        "port": typed.port,
        "hostLabel": scheme.map_or(FALLBACK_HOST_LABEL, |it| text(it, "hostLabel")),
        "placeholder": scheme.map_or("", |it| text(it, "placeholder")),
        "note": scheme.map_or(UNKNOWN_SCHEME_HINT, |it| text(it, "hint")),
        "problem": endpoint_problem(schemes, typed).map(|problem| format!("{problem} {UNSAVED}")).unwrap_or_default(),
        "options": options,
    })
}

fn typed_rows(schemes: &[Value], endpoints: &Value) -> Option<Vec<EndpointRow>> {
    match endpoints {
        Value::String(spec) => Some(match parse_specs(spec) {
            rows if rows.is_empty() => vec![blank_row(schemes)],
            rows => rows,
        }),
        Value::Array(rows) => Some(
            rows.iter()
                .map(|it| row(text(it, "scheme"), text(it, "host"), text(it, "port")))
                .collect(),
        ),
        _ => None,
    }
}

fn endpoint_editor(schemes: &[Value], rows: &[EndpointRow]) -> Value {
    json!({
        "rows": rows.iter().map(|typed| row_view(schemes, typed)).collect::<Vec<Value>>(),
        "blank": row_view(schemes, &blank_row(schemes)),
        "endpoints": serialize_endpoints(schemes, rows),
    })
}

fn ice_line(server: &Value) -> Option<String> {
    let (url, username, password) = (
        text(server, "url").trim(),
        text(server, "username").trim(),
        text(server, "password").trim(),
    );
    match (url, username, password) {
        ("", _, _) => None,
        (url, "", "") => Some(url.to_string()),
        (url, username, password) => Some(format!("{url} | {username} | {password}")),
    }
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

pub fn owns(path: &str) -> bool {
    [TELEMETRY_ENDPOINTS, ICE_SERVERS].contains(&path)
}

pub fn run(path: &str, args: &str) -> Value {
    let args = serde_json::from_str::<Vec<Value>>(args).unwrap_or_default();
    match path {
        TELEMETRY_ENDPOINTS => {
            let schemes = args.first().and_then(Value::as_array);
            match schemes.zip(args.get(1)).and_then(|(schemes, endpoints)| {
                typed_rows(schemes, endpoints).map(|rows| endpoint_editor(schemes, &rows))
            }) {
                Some(editor) => json!({ "ok": true, "result": editor }),
                None => refused(
                    "deviceTelemetry.endpoints takes the device's schemes and the endpoints, as stored or as rows of scheme, host and port.",
                ),
            }
        }
        ICE_SERVERS => match args.first().and_then(Value::as_array) {
            Some(servers) => {
                json!({ "ok": true, "result": servers.iter().filter_map(ice_line).collect::<Vec<String>>().join("\n") })
            }
            None => refused(
                "deviceNetwork.iceServers takes the relay servers, each a url with an optional username and password.",
            ),
        },
        _ => refused(&format!("{path} is not a device form")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheme(value: &str, host: &str, port: &str) -> Value {
        json!({ "value": value, "label": value, "host": host, "port": port, "hostLabel": "Address", "placeholder": "", "hint": format!("{value} hint") })
    }

    fn schemes() -> Vec<Value> {
        vec![
            scheme("udps", "0.0.0.0", "14550"),
            scheme("udpc", "", "14550"),
            scheme("udpb", "255.255.255.255", "14550"),
            scheme("tcps", "0.0.0.0", "5760"),
            scheme("tcpc", "", "5760"),
        ]
    }

    fn rows(typed: &[(&str, &str, &str)]) -> Vec<EndpointRow> {
        typed
            .iter()
            .map(|(scheme, host, port)| row(scheme, host, port))
            .collect()
    }

    fn problem(typed: (&str, &str, &str)) -> String {
        endpoint_problem(&schemes(), &row(typed.0, typed.1, typed.2)).unwrap_or_default()
    }

    fn serialized(typed: &[(&str, &str, &str)]) -> String {
        serialize_endpoints(&schemes(), &rows(typed))
    }

    #[test]
    fn stored_endpoints_come_back_as_rows_a_form_can_edit() {
        assert_eq!(
            parse_specs("udps:0.0.0.0:14550\n\n tcpc:gcs.local:5760 "),
            rows(&[("udps", "0.0.0.0", "14550"), ("tcpc", "gcs.local", "5760")])
        );
        assert_eq!(
            parse_specs("nonsense\n:host:1"),
            vec![],
            "a line with no scheme is not a row"
        );
    }

    #[test]
    fn an_ipv6_address_keeps_its_brackets_and_loses_only_the_port() {
        assert_eq!(
            parse_specs("udps:[::]:14550"),
            rows(&[("udps", "[::]", "14550")])
        );
    }

    #[test]
    fn rows_serialize_back_to_the_specs_the_router_parses() {
        assert_eq!(
            serialized(&[
                ("udps", "0.0.0.0", "14550"),
                ("udpc", " 10.0.0.5 ", " 14551 ")
            ]),
            "udps:0.0.0.0:14550\nudpc:10.0.0.5:14551"
        );
    }

    #[test]
    fn a_row_with_no_port_is_dropped_and_an_empty_host_falls_back_to_the_default() {
        assert_eq!(
            serialized(&[("udpc", "10.0.0.5", ""), ("udps", "", "14550")]),
            "udps:0.0.0.0:14550"
        );
    }

    #[test]
    fn a_new_row_is_ready_to_save_as_it_stands() {
        assert_eq!(
            serialize_endpoints(&schemes(), &[blank_row(&schemes())]),
            "udps:0.0.0.0:14550"
        );
        assert_eq!(
            serialize_endpoints(&schemes(), &[new_row(&schemes(), "tcps")]),
            "tcps:0.0.0.0:5760"
        );
        assert_eq!(blank_row(&[]), row("udps", "", "14550"));
    }

    #[test]
    fn retargeting_keeps_a_typed_address_and_replaces_an_untouched_default() {
        let retargeted =
            |typed: (&str, &str, &str)| retarget(&schemes(), &row(typed.0, typed.1, typed.2));
        assert_eq!(
            retargeted(("tcpc", "10.0.0.5", "14550")),
            row("tcpc", "10.0.0.5", "14550")
        );
        assert_eq!(
            retargeted(("udpb", "0.0.0.0", "14550")),
            row("udpb", "255.255.255.255", "14550")
        );
        assert_eq!(retargeted(("tcps", "", "")), row("tcps", "0.0.0.0", "5760"));
    }

    #[test]
    fn a_half_written_address_is_named_in_the_row_not_sent_as_a_broken_spec() {
        assert_eq!(problem(("udpc", "", "14550")), "Needs an address.");
        assert_eq!(
            problem(("udpc", "10.0.0.5", "")),
            "Needs a port between 1 and 65535."
        );
        assert_eq!(problem(("udps", "", "14550")), "");
        assert_eq!(problem(("udpc", "", "")), "");
        assert_eq!(
            serialized(&[("udpc", "", "14550"), ("udps", "", "14550")]),
            "udps:0.0.0.0:14550"
        );
    }

    #[test]
    fn a_port_that_is_not_a_port_is_named_in_the_row() {
        assert_eq!(
            problem(("udps", "0.0.0.0", "ssh")),
            "Needs a port between 1 and 65535."
        );
        assert_eq!(
            problem(("udps", "0.0.0.0", "70000")),
            "Needs a port between 1 and 65535."
        );
        assert_eq!(
            problem(("udps", "0.0.0.0", "+80")),
            "Needs a port between 1 and 65535."
        );
        assert_eq!(problem(("udps", "0.0.0.0", "14550")), "");
        assert_eq!(serialized(&[("udps", "", "ssh")]), "");
    }

    #[test]
    fn a_scheme_the_device_does_not_offer_is_shown_as_itself_and_saved_untouched() {
        let serial = row("serial", "/dev/ttyUSB0", "57600");
        let shown = row_view(&schemes(), &serial);
        let options = shown["options"].as_array().unwrap();
        assert_eq!(options.len(), 6);
        assert_eq!(
            (&options[5]["value"], &options[5]["label"]),
            (&json!("serial"), &json!("serial"))
        );
        assert_eq!(
            row_view(&schemes(), &row("udps", "", ""))["options"]
                .as_array()
                .map(Vec::len),
            Some(5)
        );
        assert_eq!(
            serialize_endpoints(&schemes(), &[serial]),
            "serial:/dev/ttyUSB0:57600"
        );
        assert_eq!(shown["note"], UNKNOWN_SCHEME_HINT);
        assert_eq!(shown["hostLabel"], "Address");
        assert_eq!(problem(("serial", "", "57600")), "Needs an address.");
    }

    #[test]
    fn the_editor_starts_from_the_stored_spec_and_answers_each_edit_with_its_rows_and_spec() {
        let started = run(
            TELEMETRY_ENDPOINTS,
            &json!([schemes(), "udps:0.0.0.0:14550"]).to_string(),
        );
        assert_eq!(started["ok"], true);
        let editor = &started["result"];
        assert_eq!(editor["endpoints"], "udps:0.0.0.0:14550");
        assert_eq!(
            editor["blank"], editor["rows"][0],
            "a new row is drawn like the stored default it equals"
        );
        let first = &editor["rows"][0];
        assert_eq!(
            (
                &first["scheme"],
                &first["host"],
                &first["port"],
                &first["note"],
                &first["problem"]
            ),
            (
                &json!("udps"),
                &json!("0.0.0.0"),
                &json!("14550"),
                &json!("udps hint"),
                &json!("")
            )
        );
        assert_eq!(
            first["options"][2],
            json!({ "value": "udpb", "label": "udpb", "row": { "scheme": "udpb", "host": "255.255.255.255", "port": "14550" } }),
            "picking a type carries the row it becomes"
        );
        assert_eq!(
            run(TELEMETRY_ENDPOINTS, &json!([schemes(), ""]).to_string())["result"]["rows"]
                .as_array()
                .map(Vec::len),
            Some(1),
            "nothing stored still offers one row"
        );
        let edited = run(
            TELEMETRY_ENDPOINTS,
            &json!([schemes(), [{ "scheme": "udpc", "host": "", "port": "14550" }, { "scheme": "tcps", "host": "", "port": "5760" }]]).to_string(),
        );
        assert_eq!(edited["result"]["endpoints"], "tcps:0.0.0.0:5760");
        assert_eq!(
            edited["result"]["rows"][0]["problem"],
            "Needs an address. It is not saved until it has one."
        );
        assert_eq!(
            run(TELEMETRY_ENDPOINTS, &json!([schemes(), []]).to_string())["result"]["endpoints"],
            ""
        );
        assert_eq!(run(TELEMETRY_ENDPOINTS, "[]")["ok"], false);
    }

    #[test]
    fn relay_servers_serialize_to_the_lines_the_device_parses_credentials_only_where_given() {
        let servers = json!([[
            { "url": " turn:turn.example.com:3478 ", "username": "drone", "password": "" },
            { "url": "stun:stun.l.google.com:19302", "username": "", "password": "" },
            { "url": "  ", "username": "orphan", "password": "x" },
        ]]);
        assert_eq!(
            run(ICE_SERVERS, &servers.to_string()),
            json!({ "ok": true, "result": "turn:turn.example.com:3478 | drone | \nstun:stun.l.google.com:19302" })
        );
        assert_eq!(
            run(ICE_SERVERS, "[[]]"),
            json!({ "ok": true, "result": "" })
        );
        assert_eq!(run(ICE_SERVERS, "[]")["ok"], false);
    }
}
