use std::collections::BTreeSet;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Drone {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub addresses: Vec<String>,
}

pub fn drones_in(body: &Value) -> Vec<Drone> {
    body.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|device| {
            let text = |key: &str| {
                device
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
            };
            let addresses: Vec<String> = device
                .get("addresses")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            Some(Drone {
                id: text("id")?.to_string(),
                name: text("name").or(text("id"))?.to_string(),
                domain: text("domain")?.to_ascii_lowercase(),
                addresses,
            })
            .filter(|drone| !drone.addresses.is_empty())
        })
        .collect()
}

fn named(domain: &str, host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == domain || host.ends_with(&format!(".{domain}"))
}

pub fn answers_as(drone: &Drone, sent: Result<(u16, Vec<u8>), String>) -> bool {
    sent.ok()
        .filter(|(status, _)| (200..300).contains(status))
        .and_then(|(_, bytes)| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|answer| {
            ["origin", "fallback"]
                .iter()
                .filter_map(|key| answer.get(key)?.as_str())
                .filter_map(|origin| url::Url::parse(origin).ok())
                .any(|origin| {
                    origin
                        .host_str()
                        .is_some_and(|host| named(&drone.domain, host))
                })
        })
}

fn set_up_already(drone: &Drone, paired: &str) -> bool {
    let host = crate::devicesetup::bare_host(paired);
    drone.addresses.iter().any(|address| address == host) || named(&drone.domain, host)
}

pub fn wanted(drones: Vec<Drone>, paired: &str, offered: &BTreeSet<String>) -> Vec<Drone> {
    drones
        .into_iter()
        .filter(|drone| !offered.contains(&drone.id) && !set_up_already(drone, paired))
        .collect()
}

pub fn found(
    drones: &[Drone],
    probe: impl Fn(&str) -> Result<(u16, Vec<u8>), String>,
) -> Option<(Drone, String)> {
    drones.iter().find_map(|drone| {
        drone
            .addresses
            .iter()
            .find(|address| answers_as(drone, probe(address)))
            .map(|address| (drone.clone(), address.clone()))
    })
}

#[cfg(not(target_family = "wasm"))]
mod probing {
    use std::collections::BTreeSet;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Mutex, MutexGuard, PoisonError};
    use std::time::Duration;

    use super::Drone;

    const TLS_ORIGIN: &str = "/api/tls";
    const EVERY_MS: u64 = 15_000;
    const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

    static NEXT_MS: AtomicU64 = AtomicU64::new(0);
    static PROBING: AtomicBool = AtomicBool::new(false);
    static OFFERED: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

    fn offered() -> MutexGuard<'static, BTreeSet<String>> {
        OFFERED.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn offer(drone: &Drone, address: &str) {
        if crate::hub::lock().active_id().is_none()
            && crate::deeplinksetup::offer_found(address, &drone.name)
        {
            offered().insert(drone.id.clone());
        }
    }

    pub fn tick(now_ms: u64) {
        if now_ms < NEXT_MS.load(Ordering::Relaxed) || PROBING.load(Ordering::Relaxed) {
            return;
        }
        NEXT_MS.store(now_ms + EVERY_MS, Ordering::Relaxed);
        if crate::hub::lock().active_id().is_some() {
            return;
        }
        let drones = super::wanted(
            crate::account::reachable_drones(),
            &crate::deviceviews::paired_host(),
            &offered(),
        );
        if drones.is_empty() {
            return;
        }
        PROBING.store(true, Ordering::Relaxed);
        let started = crate::background::spawn("groundstation-device-find", move || {
            let probe = |address: &str| {
                crate::http::send(
                    "GET",
                    &format!("http://{address}{TLS_ORIGIN}"),
                    &[("Accept", "application/json")],
                    None,
                    PROBE_TIMEOUT,
                )
            };
            if let Some((drone, address)) = super::found(&drones, probe) {
                offer(&drone, &address);
            }
            PROBING.store(false, Ordering::Relaxed);
        });
        if started.is_err() {
            PROBING.store(false, Ordering::Relaxed);
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub use probing::tick;

#[cfg(target_family = "wasm")]
pub fn tick(_now_ms: u64) {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn falcon() -> Drone {
        Drone {
            id: "d-1".into(),
            name: "falcon".into(),
            domain: "falcon-gb3v.d.aircast.one".into(),
            addresses: vec!["192.168.1.100".into(), "100.64.0.15".into()],
        }
    }

    #[test]
    fn only_a_drone_that_reported_where_it_answers_can_be_found() {
        let body = json!([
            { "id": "d-1", "name": "falcon", "domain": "Falcon-GB3V.d.aircast.one", "addresses": ["192.168.1.100", "100.64.0.15"] },
            { "id": "d-2", "name": "", "domain": "spare-k3q7.d.aircast.one", "addresses": ["100.64.0.9"] },
            { "id": "d-3", "name": "never linked" },
            { "id": "d-4", "name": "no report yet", "domain": "quiet-aaaa.d.aircast.one" },
            { "id": "", "domain": "x.d.aircast.one", "addresses": ["10.0.0.1"] }
        ]);
        let drones = drones_in(&body);
        assert_eq!(drones[0], falcon(), "the domain is matched without case");
        assert_eq!(
            drones.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["falcon", "d-2"],
            "a drone with no name is called by its id"
        );
        assert!(drones_in(&json!({ "error": "x" })).is_empty());
    }

    #[test]
    fn a_drone_is_found_only_where_it_answers_with_its_own_name() {
        let drone = falcon();
        let tls = |origin: &str| Ok((200, json!({ "origin": origin }).to_string().into_bytes()));
        assert!(answers_as(
            &drone,
            tls("https://100-64-0-15.falcon-gb3v.d.aircast.one")
        ));
        assert!(answers_as(&drone, tls("https://falcon-gb3v.d.aircast.one")));
        assert!(answers_as(
            &drone,
            Ok((
                200,
                br#"{"origin":"","fallback":"https://falcon-gb3v.d.aircast.one:8443"}"#.to_vec()
            ))
        ));
        assert!(
            !answers_as(
                &drone,
                tls("https://192-168-1-100.other-k3q7.d.aircast.one")
            ),
            "another Aircast device at the same address is not this drone"
        );
        assert!(!answers_as(
            &drone,
            tls("https://evilfalcon-gb3v.d.aircast.one")
        ));
        assert!(
            !answers_as(&drone, tls("")),
            "a device with no certificate proves nothing"
        );
        assert!(!answers_as(&drone, Ok((404, Vec::new()))));
        assert!(!answers_as(&drone, Err("timed out".into())));
    }

    #[test]
    fn the_first_address_that_answers_is_offered_in_the_order_the_drone_reported() {
        let drones = [falcon()];
        let asked = std::cell::RefCell::new(Vec::new());
        let only_tailnet = |address: &str| {
            asked.borrow_mut().push(address.to_string());
            match address {
                "100.64.0.15" => Ok((
                    200,
                    br#"{"origin":"https://100-64-0-15.falcon-gb3v.d.aircast.one"}"#.to_vec(),
                )),
                _ => Err("no route".to_string()),
            }
        };
        assert_eq!(
            found(&drones, only_tailnet),
            Some((falcon(), "100.64.0.15".to_string()))
        );
        assert_eq!(*asked.borrow(), ["192.168.1.100", "100.64.0.15"]);
        assert_eq!(found(&drones, |_| Err("offline".into())), None);
    }

    #[test]
    fn a_drone_already_set_up_or_already_offered_is_left_alone() {
        let none = BTreeSet::new();
        assert_eq!(wanted(vec![falcon()], "", &none), [falcon()]);
        for paired in [
            "100.64.0.15",
            "192.168.1.100:8080",
            "falcon-gb3v.d.aircast.one",
            "100-64-0-15.falcon-gb3v.d.aircast.one",
        ] {
            assert!(wanted(vec![falcon()], paired, &none).is_empty(), "{paired}");
        }
        assert_eq!(wanted(vec![falcon()], "10.0.0.7", &none), [falcon()]);
        assert!(wanted(vec![falcon()], "", &BTreeSet::from(["d-1".to_string()])).is_empty());
    }
}
