use std::collections::BTreeSet;

use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cardprovision::{AccessConfig, InitFormat, ProvisionConfig, SshMode, TailscaleConfig, WifiConfig};

const MAX_ADVANCE: usize = 50;
const MAX_GENERATED: usize = 999;
const MIN_DEVICE_PASSWORD: usize = 8;
const DEFAULT_COUNTRY: &str = "US";
const IMAGE_DEFAULT_LOGIN: &str = "Image default (pi / raspberry)";
pub const FLASHED_HISTORY_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Form {
    pub hostname: String,
    pub ssid: String,
    pub wifi_password: String,
    pub no_wifi: bool,
    pub country: String,
    pub ssh_mode: String,
    pub authorized_key: String,
    pub device_password: String,
    pub auth_key: String,
    pub control_server: String,
    pub secured_ssid: String,
}

impl Default for Form {
    fn default() -> Self {
        Self {
            hostname: String::new(),
            ssid: String::new(),
            wifi_password: String::new(),
            no_wifi: false,
            country: String::new(),
            ssh_mode: "key-only".into(),
            authorized_key: String::new(),
            device_password: String::new(),
            auth_key: String::new(),
            control_server: String::new(),
            secured_ssid: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Remembered {
    pub ssid: String,
    pub wifi_password: String,
    pub no_wifi: bool,
    pub country: String,
    pub authorized_key: String,
    pub control_server: String,
    pub channel: String,
    pub flashed_hostnames: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyIdentity {
    pub algorithm: String,
    pub comment: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummaryRow {
    pub label: &'static str,
    pub value: String,
    pub warn: bool,
}

fn pattern(source: &str) -> Regex {
    Regex::new(source).expect("a fixed pattern compiles")
}

fn lowered(names: &[String]) -> BTreeSet<String> {
    names.iter().map(|name| name.trim().to_lowercase()).collect()
}

fn numbered(n: usize) -> String {
    format!("aircast-{n:02}")
}

pub fn default_hostname(taken: &[String]) -> String {
    let used = lowered(taken);
    (1..=MAX_GENERATED).map(numbered).find(|candidate| !used.contains(candidate)).unwrap_or_else(|| numbered(1))
}

pub fn next_hostname(current: &str) -> String {
    let name = current.trim();
    match (name.is_empty(), pattern(r"^(.*?)(\d+)$").captures(name)) {
        (true, _) => default_hostname(&[]),
        (false, None) => format!("{name}-02"),
        (false, Some(found)) => {
            let digits = &found[2];
            let incremented = (digits.parse::<u128>().unwrap_or(0) + 1).to_string();
            format!("{}{incremented:0>width$}", &found[1], width = digits.len())
        }
    }
}

pub fn next_free_hostname(current: &str, taken: &[String]) -> String {
    let used = lowered(taken);
    (0..MAX_ADVANCE).fold(next_hostname(current), |name, _| match used.contains(&name.to_lowercase()) {
        true => next_hostname(&name),
        false => name,
    })
}

pub fn valid_hostname(value: &str) -> bool {
    pattern(r"^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$").is_match(value.trim())
}

pub fn valid_control_server(value: &str) -> bool {
    let v = value.trim();
    v.is_empty() || pattern(r"^https?://.+").is_match(v)
}

pub fn invalid_ssh_key(value: &str) -> bool {
    let v = value.trim();
    !v.is_empty() && !pattern(r"(?i)^[a-z][a-z0-9@.-]*\s+[A-Za-z0-9+/]+=*(\s.*)?$").is_match(v)
}

pub fn invalid_wifi_password(value: &str) -> bool {
    !value.is_empty() && !(8..=63).contains(&value.chars().count()) && !pattern(r"(?i)^[0-9a-f]{64}$").is_match(value)
}

pub fn weak_device_password(value: &str) -> bool {
    !value.is_empty() && value.chars().count() < MIN_DEVICE_PASSWORD
}

pub fn needs_ssid(form: &Form) -> bool {
    !form.no_wifi && form.ssid.trim().is_empty()
}

pub fn identify_key(key: &str) -> Option<KeyIdentity> {
    let parts: Vec<&str> = key.split_whitespace().collect();
    let decoded = base64::engine::general_purpose::STANDARD.decode(parts.get(1)?).ok().filter(|blob| !blob.is_empty())?;
    let fingerprint = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(&decoded));
    Some(KeyIdentity { algorithm: parts[0].to_string(), comment: parts[2..].join(" "), fingerprint: format!("SHA256:{}", fingerprint.trim_end_matches('=')) })
}

fn ssh_mode(form: &Form) -> SshMode {
    match form.ssh_mode.as_str() {
        "password" => SshMode::Password,
        "disabled" => SshMode::Disabled,
        _ => SshMode::KeyOnly,
    }
}

pub fn provision(form: &Form) -> ProvisionConfig {
    let blank = |value: &str| value.trim().is_empty();
    ProvisionConfig {
        hostname: Some(form.hostname.trim().to_string()).filter(|h| !h.is_empty()),
        wifi: (!form.no_wifi && !blank(&form.ssid)).then(|| WifiConfig {
            ssid: form.ssid.trim().to_string(),
            password: form.wifi_password.clone(),
            country: Some(form.country.trim().to_uppercase()).filter(|c| c.len() == 2).unwrap_or_else(|| DEFAULT_COUNTRY.to_string()),
        }),
        tailscale: (!blank(&form.auth_key) && valid_control_server(&form.control_server)).then(|| TailscaleConfig { control_server: form.control_server.trim().to_string(), auth_key: form.auth_key.trim().to_string() }),
        access: Some(AccessConfig {
            ssh: ssh_mode(form),
            authorized_key: Some(form.authorized_key.trim().to_string()).filter(|k| !k.is_empty()),
            password: Some(form.device_password.clone()).filter(|p| !p.is_empty()),
        }),
        init_format: InitFormat::CloudInit,
    }
}

pub fn problems(form: &Form) -> Vec<(&'static str, String)> {
    let ssid = form.ssid.trim();
    let mode = ssh_mode(form);
    [
        (!form.hostname.trim().is_empty() && !valid_hostname(&form.hostname)).then(|| ("hostname", "Use lowercase letters, digits and dashes, up to 63 characters.".to_string())),
        needs_ssid(form).then(|| ("ssid", "Enter the WiFi network name, or choose no WiFi.".to_string())),
        (!form.no_wifi && invalid_wifi_password(&form.wifi_password)).then(|| ("wifiPassword", "A WiFi password is 8 to 63 characters, or a 64-digit hex key.".to_string())),
        (!form.no_wifi && form.wifi_password.is_empty() && !ssid.is_empty() && ssid == form.secured_ssid.trim()).then(|| ("wifiPassword", format!("{ssid} needs its password. This phone uses one to join it."))),
        (mode == SshMode::KeyOnly && form.authorized_key.trim().is_empty()).then(|| ("authorizedKey", "Paste your public key, or sign in with a password instead.".to_string())),
        (mode == SshMode::KeyOnly && invalid_ssh_key(&form.authorized_key)).then(|| ("authorizedKey", "That does not look like an SSH public key.".to_string())),
        (mode == SshMode::Password && form.device_password.is_empty()).then(|| ("devicePassword", "Set a password for pi, or turn SSH off under More options.".to_string())),
        (mode == SshMode::Password && weak_device_password(&form.device_password)).then(|| ("devicePassword", "Use at least 8 characters.".to_string())),
        (!valid_control_server(&form.control_server)).then(|| ("controlServer", "A control server is a URL starting with https://".to_string())),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn remote_access(form: &Form) -> (String, bool) {
    let server = form.control_server.trim();
    match (form.auth_key.trim().is_empty(), server.is_empty(), valid_control_server(server)) {
        (true, _, _) => ("Off".into(), false),
        (false, true, _) => ("Tailscale".into(), false),
        (false, false, false) => ("Off — control server isn't a URL".into(), true),
        (false, false, true) => (format!("Headscale · {}", pattern(r"^https?://").replace(server, "")), false),
    }
}

fn device_access(form: &Form) -> String {
    let key = form.authorized_key.trim();
    match ssh_mode(form) {
        SshMode::Disabled => "SSH disabled".into(),
        SshMode::KeyOnly if key.is_empty() => IMAGE_DEFAULT_LOGIN.into(),
        SshMode::KeyOnly => key.split_whitespace().nth(2).map(|comment| format!("SSH key · {comment}")).unwrap_or_else(|| "SSH key · pasted".into()),
        SshMode::Password if form.device_password.is_empty() => IMAGE_DEFAULT_LOGIN.into(),
        SshMode::Password => "Password for pi".into(),
    }
}

pub fn summary(image: &str, form: &Form) -> Vec<SummaryRow> {
    let hostname = form.hostname.trim();
    let ssid = form.ssid.trim();
    let (remote, remote_warn) = remote_access(form);
    let access = device_access(form);
    vec![
        SummaryRow { label: "Image", value: image.to_string(), warn: false },
        SummaryRow { label: "Hostname", value: if hostname.is_empty() { "Image default".into() } else { format!("{hostname}.local") }, warn: false },
        match (form.no_wifi, ssid.is_empty()) {
            (true, _) => SummaryRow { label: "WiFi", value: "Ethernet or cellular only".into(), warn: false },
            (false, true) => SummaryRow { label: "WiFi", value: "Not configured".into(), warn: true },
            (false, false) if form.wifi_password.is_empty() => SummaryRow { label: "WiFi", value: format!("{ssid} · no password"), warn: true },
            (false, false) => SummaryRow { label: "WiFi", value: format!("{ssid} · password set"), warn: false },
        },
        SummaryRow { label: "Remote access", value: remote, warn: remote_warn },
        SummaryRow { label: "Device access", warn: access == IMAGE_DEFAULT_LOGIN, value: access },
    ]
}

pub fn remembered_after_flash(remembered: &Remembered, form: &Form) -> Remembered {
    let written = form.hostname.trim().to_string();
    let flashed: Vec<String> = match written.is_empty() || remembered.flashed_hostnames.contains(&written) {
        true => remembered.flashed_hostnames.clone(),
        false => remembered.flashed_hostnames.iter().cloned().chain(std::iter::once(written)).collect(),
    };
    let skip = flashed.len().saturating_sub(FLASHED_HISTORY_LIMIT);
    Remembered {
        ssid: form.ssid.trim().to_string(),
        wifi_password: form.wifi_password.clone(),
        no_wifi: form.no_wifi,
        country: form.country.clone(),
        authorized_key: form.authorized_key.trim().to_string(),
        control_server: form.control_server.trim().to_string(),
        flashed_hostnames: flashed.into_iter().skip(skip).collect(),
        ..remembered.clone()
    }
}

pub fn form_from(remembered: &Remembered) -> Form {
    Form {
        hostname: default_hostname(&remembered.flashed_hostnames),
        ssid: remembered.ssid.clone(),
        wifi_password: remembered.wifi_password.clone(),
        no_wifi: remembered.no_wifi,
        country: remembered.country.clone(),
        authorized_key: remembered.authorized_key.clone(),
        control_server: remembered.control_server.clone(),
        ..Form::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJ3z5r1sZ0K7DxHhqTn7lVXJHkiqrxKZ+i0Ck0KFqkkn operator@base";

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn base() -> Form {
        Form { hostname: "falcon-01".into(), ssid: "field-net".into(), ..Form::default() }
    }

    fn complete() -> Form {
        Form { wifi_password: "hunter22".into(), ssh_mode: "password".into(), device_password: "s3cret-pass".into(), ..base() }
    }

    fn row(form: &Form, label: &str) -> SummaryRow {
        summary("Aircast OS v0.3.0", form).into_iter().find(|r| r.label == label).expect("row")
    }

    #[test]
    fn a_generated_default_counts_up_like_any_other_numbered_name() {
        assert_eq!(next_hostname("aircast-01"), "aircast-02");
        assert_eq!(next_hostname("aircast-09"), "aircast-10");
    }

    #[test]
    fn a_numbered_name_counts_up_keeping_its_padding() {
        assert_eq!(next_hostname("falcon-01"), "falcon-02");
        assert_eq!(next_hostname("falcon-09"), "falcon-10");
        assert_eq!(next_hostname("falcon-99"), "falcon-100");
        assert_eq!(next_hostname("falcon9"), "falcon10");
    }

    #[test]
    fn an_unnumbered_name_gets_a_number_rather_than_reusing_the_same_one() {
        assert_eq!(next_hostname("falcon"), "falcon-02");
        assert_eq!(next_hostname("  falcon  "), "falcon-02");
    }

    #[test]
    fn an_empty_name_falls_back_to_a_generated_default() {
        assert_eq!(next_hostname(""), "aircast-01");
    }

    #[test]
    fn flashing_a_batch_never_repeats_a_hostname() {
        let batch: Vec<String> = std::iter::successors(Some("falcon-01".to_string()), |name| Some(next_hostname(name))).take(21).collect();
        assert_eq!(lowered(&batch).len(), batch.len());
    }

    #[test]
    fn counting_up_skips_names_already_taken_keeping_the_operators_stem() {
        assert_eq!(next_free_hostname("falcon-01", &[]), "falcon-02");
        assert_eq!(next_free_hostname("falcon-01", &names(&["falcon-02"])), "falcon-03");
        assert_eq!(next_free_hostname("falcon-01", &names(&["falcon-02", "FALCON-03"])), "falcon-04");
        assert_eq!(next_free_hostname("aircast-01", &names(&["aircast-02"])), "aircast-03");
    }

    #[test]
    fn the_default_hostname_is_the_lowest_number_not_already_in_use() {
        assert_eq!(default_hostname(&[]), "aircast-01");
        assert_eq!(default_hostname(&names(&["aircast-01"])), "aircast-02");
        assert_eq!(default_hostname(&names(&["aircast-01", "aircast-02"])), "aircast-03");
        assert_eq!(default_hostname(&names(&["AIRCAST-01", "aircast-03"])), "aircast-02");
        assert_eq!(default_hostname(&names(&["falcon", "owel-3"])), "aircast-01");
    }

    #[test]
    fn hostnames_follow_dns_label_rules() {
        assert!(valid_hostname("aircast-01"));
        assert!(valid_hostname(" falcon "));
        assert!(!valid_hostname("Falcon"));
        assert!(!valid_hostname("-falcon"));
        assert!(!valid_hostname("falcon-"));
        assert!(!valid_hostname(&"a".repeat(64)));
    }

    #[test]
    fn an_open_network_needs_no_password() {
        assert!(!invalid_wifi_password(""));
    }

    #[test]
    fn rejects_passphrases_wpa_cannot_hold() {
        assert!(invalid_wifi_password("short"));
        assert!(invalid_wifi_password(&"z".repeat(64)));
    }

    #[test]
    fn accepts_a_real_passphrase_and_a_raw_64_hex_key() {
        assert!(!invalid_wifi_password("hunter22"));
        assert!(!invalid_wifi_password(&"a".repeat(63)));
        assert!(!invalid_wifi_password(&"0123456789abcdef".repeat(4)));
    }

    #[test]
    fn ssh_keys_and_device_passwords_are_checked_only_when_given() {
        assert!(!invalid_ssh_key(""));
        assert!(!invalid_ssh_key(KEY));
        assert!(invalid_ssh_key("hello"));
        assert!(!weak_device_password(""));
        assert!(weak_device_password("short"));
        assert!(!weak_device_password("long enough"));
    }

    #[test]
    fn reports_the_hostname_as_the_address_the_device_answers_on() {
        assert_eq!(row(&base(), "Hostname").value, "falcon-01.local");
        assert_eq!(row(&Form { hostname: "  ".into(), ..base() }, "Hostname").value, "Image default");
    }

    #[test]
    fn says_when_wifi_is_not_configured_and_flags_it() {
        assert_eq!(row(&complete(), "WiFi"), SummaryRow { label: "WiFi", value: "field-net · password set".into(), warn: false });
        assert_eq!(row(&Form { ssid: "".into(), ..base() }, "WiFi"), SummaryRow { label: "WiFi", value: "Not configured".into(), warn: true });
    }

    #[test]
    fn a_network_with_no_password_is_called_out_before_writing() {
        assert_eq!(row(&base(), "WiFi"), SummaryRow { label: "WiFi", value: "field-net · no password".into(), warn: true });
    }

    #[test]
    fn a_secured_network_the_phone_is_on_needs_its_password() {
        let fields = |form: Form| problems(&form).into_iter().map(|(field, _)| field).collect::<Vec<_>>();
        assert_eq!(fields(Form { secured_ssid: "field-net".into(), ..complete() }), Vec::<&str>::new());
        let missing = Form { secured_ssid: "field-net".into(), wifi_password: "".into(), ..complete() };
        assert_eq!(problems(&missing), vec![("wifiPassword", "field-net needs its password. This phone uses one to join it.".to_string())]);
        assert_eq!(fields(Form { secured_ssid: "other-net".into(), wifi_password: "".into(), ..complete() }), Vec::<&str>::new(), "an open network the phone is not on stays allowed");
    }

    #[test]
    fn the_image_default_login_cannot_survive_a_flash() {
        let fields = |form: Form| problems(&form).into_iter().map(|(field, _)| field).collect::<Vec<_>>();
        assert_eq!(fields(Form { ssh_mode: "password".into(), device_password: "".into(), ..complete() }), vec!["devicePassword"]);
        assert_eq!(fields(Form { ssh_mode: "key-only".into(), authorized_key: "".into(), ..complete() }), vec!["authorizedKey"]);
        assert_eq!(fields(Form { ssh_mode: "disabled".into(), device_password: "".into(), ..complete() }), Vec::<&str>::new());
    }

    #[test]
    fn skipping_wifi_on_purpose_is_stated_not_flagged() {
        assert_eq!(row(&Form { ssid: "".into(), no_wifi: true, ..base() }, "WiFi"), SummaryRow { label: "WiFi", value: "Ethernet or cellular only".into(), warn: false });
    }

    #[test]
    fn the_surviving_image_default_login_is_flagged() {
        assert!(row(&base(), "Device access").warn);
        assert!(!row(&Form { authorized_key: "ssh-rsa AAAAB3Nz".into(), ..base() }, "Device access").warn);
    }

    #[test]
    fn remote_access_is_off_without_a_key_whatever_the_control_server_says() {
        assert_eq!(row(&base(), "Remote access").value, "Off");
        assert_eq!(row(&Form { control_server: "https://hs.example.com".into(), ..base() }, "Remote access").value, "Off");
    }

    #[test]
    fn distinguishes_tailscale_from_a_self_hosted_control_server() {
        let with_key = Form { auth_key: "tskey-auth-abc".into(), ..base() };
        assert_eq!(row(&with_key, "Remote access").value, "Tailscale");
        assert_eq!(row(&Form { control_server: "https://hs.example.com".into(), ..with_key }, "Remote access").value, "Headscale · hs.example.com");
    }

    #[test]
    fn a_control_server_that_isnt_a_url_means_remote_access_is_skipped() {
        let bad = Form { auth_key: "tskey-auth-abc".into(), control_server: "headscale".into(), ..base() };
        assert_eq!(row(&bad, "Remote access"), SummaryRow { label: "Remote access", value: "Off — control server isn't a URL".into(), warn: true });
        assert_eq!(provision(&bad).tailscale, None);
    }

    #[test]
    fn an_empty_key_or_password_means_the_image_default_login_survives() {
        assert_eq!(row(&base(), "Device access").value, IMAGE_DEFAULT_LOGIN);
        assert_eq!(row(&Form { ssh_mode: "password".into(), ..base() }, "Device access").value, IMAGE_DEFAULT_LOGIN);
    }

    #[test]
    fn names_the_key_that_will_be_written() {
        assert_eq!(row(&Form { authorized_key: "ssh-ed25519 AAAAC3Nz pavliha@mac".into(), ..base() }, "Device access").value, "SSH key · pavliha@mac");
        assert_eq!(row(&Form { authorized_key: "ssh-rsa AAAAB3Nz".into(), ..base() }, "Device access").value, "SSH key · pasted");
    }

    #[test]
    fn reports_a_set_password_and_a_disabled_ssh_server() {
        assert_eq!(row(&Form { ssh_mode: "password".into(), device_password: "s3cret!".into(), ..base() }, "Device access").value, "Password for pi");
        assert_eq!(row(&Form { ssh_mode: "disabled".into(), ..base() }, "Device access").value, "SSH disabled");
    }

    #[test]
    fn identifies_a_public_key_the_way_ssh_keygen_does() {
        assert_eq!(identify_key(KEY), Some(KeyIdentity { algorithm: "ssh-ed25519".into(), comment: "operator@base".into(), fingerprint: "SHA256:6ToLTRFTRhCceqBA4tBy84TESMsJViBkYltFaKXXP20".into() }));
        assert_eq!(identify_key(KEY.trim_end_matches(" operator@base")).map(|k| k.comment), Some(String::new()));
    }

    #[test]
    fn rubbish_is_not_a_key() {
        assert_eq!(identify_key("hello"), None);
        assert_eq!(identify_key("ssh-ed25519 not-base64!!"), None);
        assert_eq!(identify_key(""), None);
    }

    #[test]
    fn the_form_becomes_a_provision_config_without_blank_parts() {
        let config = provision(&Form { country: "ge".into(), wifi_password: "hunter22".into(), authorized_key: format!("  {KEY}  "), ..base() });
        assert_eq!(config.hostname.as_deref(), Some("falcon-01"));
        assert_eq!(config.wifi, Some(WifiConfig { ssid: "field-net".into(), password: "hunter22".into(), country: "GE".into() }));
        assert_eq!(config.tailscale, None);
        assert_eq!(config.access.as_ref().and_then(|a| a.authorized_key.clone()).as_deref(), Some(KEY));
        assert_eq!(config.init_format, InitFormat::CloudInit);
        assert_eq!(provision(&Form { no_wifi: true, ..base() }).wifi, None);
        assert_eq!(provision(&Form { country: "".into(), ..base() }).wifi.map(|w| w.country).as_deref(), Some("US"));
    }

    #[test]
    fn problems_name_the_field_that_blocks_writing() {
        assert_eq!(problems(&complete()), vec![]);
        let fields = |form: Form| problems(&form).into_iter().map(|(field, _)| field).collect::<Vec<_>>();
        assert_eq!(fields(Form { hostname: "Bad Name".into(), ..complete() }), vec!["hostname"]);
        assert_eq!(fields(Form { ssid: "".into(), ..complete() }), vec!["ssid"]);
        assert_eq!(fields(Form { ssid: "".into(), no_wifi: true, ..complete() }), Vec::<&str>::new());
        assert_eq!(fields(Form { wifi_password: "short".into(), ..complete() }), vec!["wifiPassword"]);
        assert_eq!(fields(Form { ssh_mode: "key-only".into(), authorized_key: "hello".into(), ..complete() }), vec!["authorizedKey"]);
        assert_eq!(fields(Form { device_password: "short".into(), ..complete() }), vec!["devicePassword"]);
    }

    #[test]
    fn a_flash_remembers_the_network_and_the_name_it_used() {
        let after = remembered_after_flash(&Remembered { channel: "stable".into(), flashed_hostnames: names(&["falcon-01"]), ..Remembered::default() }, &Form { hostname: "falcon-02".into(), ..base() });
        assert_eq!(after.flashed_hostnames, names(&["falcon-01", "falcon-02"]));
        assert_eq!(after.ssid, "field-net");
        assert_eq!(after.channel, "stable");
        assert_eq!(remembered_after_flash(&after, &Form { hostname: "falcon-02".into(), ..base() }).flashed_hostnames.len(), 2);
        assert_eq!(form_from(&after).hostname, "aircast-01");
    }

    #[test]
    fn the_flashed_history_keeps_only_the_newest_names() {
        let full = Remembered { flashed_hostnames: (0..FLASHED_HISTORY_LIMIT).map(|i| format!("old-{i}")).collect(), ..Remembered::default() };
        let after = remembered_after_flash(&full, &Form { hostname: "new-1".into(), ..base() });
        assert_eq!(after.flashed_hostnames.len(), FLASHED_HISTORY_LIMIT);
        assert_eq!(after.flashed_hostnames.last().map(String::as_str), Some("new-1"));
        assert_eq!(after.flashed_hostnames.first().map(String::as_str), Some("old-1"));
    }
}
