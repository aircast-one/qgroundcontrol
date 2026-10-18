use std::net::IpAddr;
use std::sync::{Mutex, PoisonError};

use url::Url;

pub const NEEDS_DEVICE: &str =
    "Open QGroundControl from your Aircast device to use this in a browser.";
pub const PAIR: &str = "device.pair";
pub const PROXY_TARGET: &str = "Aircast-Target";

static BASE: Mutex<Option<Url>> = Mutex::new(None);

pub fn pair(base: &str) -> Result<(), String> {
    let base =
        Url::parse(base).map_err(|error| format!("{base} is not a device address: {error}"))?;
    if !matches!(base.scheme(), "http" | "https") || base.cannot_be_a_base() {
        return Err(format!("{base} is not a device address"));
    }
    *BASE.lock().unwrap_or_else(PoisonError::into_inner) = Some(base.join("./").unwrap_or(base));
    #[cfg(target_family = "wasm")]
    crate::offlinemaps::browser::restore_from_device();
    Ok(())
}

pub fn unpair() {
    *BASE.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

pub fn paired() -> bool {
    base().is_some()
}

fn base() -> Option<Url> {
    BASE.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

pub fn foreign(url: &str) -> bool {
    let (Some(base), Ok(url)) = (base(), Url::parse(url)) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https") && url.origin() != base.origin()
}

pub fn address(host: &str, port: u16) -> String {
    match host.trim_matches(['[', ']']).parse::<IpAddr>() {
        Ok(IpAddr::V6(ip)) => format!("[{ip}]:{port}"),
        _ => format!("{host}:{port}"),
    }
}

fn socket(query: &[(&str, String)]) -> Option<String> {
    let mut url = base()?.join("socket").ok()?;
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme).ok()?;
    url.query_pairs_mut().extend_pairs(query);
    Some(url.into())
}

pub fn tcp(host: &str, port: u16) -> Option<String> {
    socket(&[("tcp", address(host, port))])
}

pub fn tls(host: &str, port: u16, self_signed: bool) -> Option<String> {
    let accept = self_signed.then(|| ("selfsigned", "1".to_string()));
    socket(
        &std::iter::once(("tls", address(host, port)))
            .chain(accept)
            .collect::<Vec<_>>(),
    )
}

pub fn udp(listen: u16, to: &[(String, u16)]) -> Option<String> {
    let targets = to.iter().map(|(host, port)| ("to", address(host, *port)));
    socket(
        &std::iter::once(("udp", listen.to_string()))
            .chain(targets)
            .collect::<Vec<_>>(),
    )
}

pub fn files(path: &str) -> Option<String> {
    let mut url = base()?.join("files/").ok()?;
    url.path_segments_mut()
        .ok()?
        .pop_if_empty()
        .extend(path.split('/').filter(|segment| !segment.is_empty()));
    Some(url.into())
}

pub fn proxy() -> Option<String> {
    base()?.join("proxy").ok().map(Into::into)
}

pub fn firmware() -> Option<String> {
    base()?.join("/api/telemetry/firmware").ok().map(Into::into)
}

pub fn pair_command(args: &str) -> serde_json::Value {
    let base = serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|args| {
            args.get(0)
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();
    match pair(&base) {
        Ok(()) => serde_json::json!({ "ok": true }),
        Err(reason) => serde_json::json!({ "ok": false, "reason": reason }),
    }
}

#[cfg(test)]
pub(crate) static TEST_PAIRING: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paired_device_page_names_every_endpoint_on_its_own_origin() {
        let _pairing = TEST_PAIRING.lock().unwrap_or_else(PoisonError::into_inner);
        pair("http://falcon.local/fly/index.html").unwrap();
        assert_eq!(
            tcp("10.0.0.2", 30003).as_deref(),
            Some("ws://falcon.local/fly/socket?tcp=10.0.0.2%3A30003")
        );
        assert_eq!(
            tls("caster.example", 443, false).as_deref(),
            Some("ws://falcon.local/fly/socket?tls=caster.example%3A443")
        );
        assert_eq!(
            udp(
                14550,
                &[("fe80::1".into(), 14555), ("192.168.1.5".into(), 14550)]
            )
            .as_deref(),
            Some(
                "ws://falcon.local/fly/socket?udp=14550&to=%5Bfe80%3A%3A1%5D%3A14555&to=192.168.1.5%3A14550"
            )
        );
        assert_eq!(
            files("/Aircast/Logs/a b.ulg").as_deref(),
            Some("http://falcon.local/fly/files/Aircast/Logs/a%20b.ulg")
        );
        assert_eq!(proxy().as_deref(), Some("http://falcon.local/fly/proxy"));
        assert_eq!(
            firmware().as_deref(),
            Some("http://falcon.local/api/telemetry/firmware")
        );
        pair("https://falcon.tail.ts.net/fly/").unwrap();
        assert_eq!(
            tcp("h", 1).as_deref(),
            Some("wss://falcon.tail.ts.net/fly/socket?tcp=h%3A1"),
            "an https page talks to its relay over wss"
        );
        unpair();
        assert!(
            tcp("h", 1).is_none() && files("x").is_none(),
            "without a device there is no relay or store"
        );
    }

    #[test]
    fn only_a_web_address_pairs() {
        let _pairing = TEST_PAIRING.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(
            crate::actions::owns(PAIR),
            "the router hands device.pair to the pairing command, not to the Qt backend"
        );
        assert!(pair("file:///fly/").is_err() && pair("not a url").is_err());
        assert_eq!(pair_command(r#"["mailto:x@y"]"#)["ok"], false);
        assert!(!paired());
    }
}
