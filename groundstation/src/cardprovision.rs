use serde::{Deserialize, Serialize};

const TS_STATE_PATH: &str = "/var/lib/aircastd/tailscale.json";
const TS_KEY_PATH: &str = "/var/lib/aircastd/tailscale.authkey";
const KEY_STAGE_PATH: &str = "/root/.aircast-authorized-key";
const INSTALL_KEY_PATH: &str = "/root/aircast-install-key.sh";
const RECOVERY_MARKER: &str = "/var/log/aircast-key-install-failed";
const ACCESS_DROPIN: &str = "/etc/ssh/sshd_config.d/10-aircast-access.conf";
const PSK_ITERATIONS: u32 = 4096;
const SCRUB_USERDATA_ITEM: &str = "  - [ sh, -c, \"rm -f /boot/firmware/user-data /var/lib/cloud/instance/user-data.txt /var/lib/cloud/instances/*/user-data.txt 2>/dev/null; true\" ]\n";
const DISABLE_SSH_ITEM: &str = "  - [ sh, -c, \"systemctl disable --now ssh.socket ssh.service 2>/dev/null; true\" ]\n";
pub const FIRSTRUN_CMDLINE: &str = " systemd.run=/boot/firstrun.sh systemd.run_success_action=reboot systemd.unit=kernel-command-line.target";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WifiConfig {
    pub ssid: String,
    pub password: String,
    pub country: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum InitFormat {
    #[default]
    CloudInit,
    FirstRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleConfig {
    pub control_server: String,
    pub auth_key: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SshMode {
    KeyOnly,
    Password,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccessConfig {
    pub ssh: SshMode,
    #[serde(default)]
    pub authorized_key: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

impl AccessConfig {
    pub fn is_effective(&self) -> bool {
        match self.ssh {
            SshMode::Disabled => true,
            SshMode::KeyOnly => non_blank(&self.authorized_key),
            SshMode::Password => non_blank(&self.password),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProvisionConfig {
    pub hostname: Option<String>,
    pub wifi: Option<WifiConfig>,
    pub tailscale: Option<TailscaleConfig>,
    #[serde(default)]
    pub access: Option<AccessConfig>,
    pub init_format: InitFormat,
}

impl ProvisionConfig {
    pub fn is_empty(&self) -> bool {
        self.hostname.is_none() && self.wifi.is_none() && self.tailscale.is_none() && !self.access.as_ref().is_some_and(AccessConfig::is_effective)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionFile {
    pub name: String,
    pub contents: String,
    pub executable: bool,
}

impl ProvisionFile {
    fn plain(name: &str, contents: String) -> Self {
        Self { name: name.into(), contents, executable: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisionPlan {
    Noop,
    CloudInit { files: Vec<ProvisionFile> },
    FirstRun { script: ProvisionFile, cmdline_append: String },
}

#[derive(Default)]
struct CloudAccess {
    top: String,
    write_files: Vec<String>,
    runcmd: Vec<String>,
    wrote_secret: bool,
}

pub fn wpa_psk(ssid: &str, password: &str) -> String {
    crate::signingkeys::to_hex(&pbkdf2::pbkdf2_hmac_array::<sha1::Sha1, 32>(password.as_bytes(), ssid.as_bytes(), PSK_ITERATIONS))
}

pub fn plan(config: &ProvisionConfig) -> ProvisionPlan {
    match (config.is_empty(), config.init_format) {
        (true, _) => ProvisionPlan::Noop,
        (false, InitFormat::CloudInit) => ProvisionPlan::CloudInit { files: cloud_init_files(config) },
        (false, InitFormat::FirstRun) => ProvisionPlan::FirstRun {
            script: ProvisionFile { name: "firstrun.sh".into(), contents: firstrun_script(config), executable: true },
            cmdline_append: FIRSTRUN_CMDLINE.to_string(),
        },
    }
}

fn non_blank(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|s| !s.trim().is_empty())
}

fn sanitize_line(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

fn sanitized(value: &Option<String>) -> Option<String> {
    Some(sanitize_line(value.as_deref().unwrap_or_default()).trim().to_string()).filter(|s| !s.is_empty())
}

fn provisioned_tailscale(ts: &TailscaleConfig) -> Option<(String, String)> {
    Some((sanitize_line(&ts.control_server), sanitize_line(&ts.auth_key))).filter(|(_, key)| !key.is_empty())
}

fn json_string(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            other => other.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn shell_single_quote(s: &str) -> String {
    s.replace('\'', "'\\''")
}

fn tailscale_state_json(control_server: &str) -> String {
    format!("{{\"loginServer\":{}}}", json_string(control_server))
}

fn section(name: &str, entries: &[String]) -> String {
    match entries.is_empty() {
        true => String::new(),
        false => format!("{name}:\n{}", entries.concat()),
    }
}

fn cloud_init_files(config: &ProvisionConfig) -> Vec<ProvisionFile> {
    cloud_init_user_data(config)
        .map(|user_data| ProvisionFile::plain("user-data", user_data))
        .into_iter()
        .chain(config.wifi.as_ref().map(|wifi| ProvisionFile::plain("network-config", network_config_v2(wifi))))
        .chain(std::iter::once(ProvisionFile::plain("meta-data", String::new())))
        .collect()
}

fn cloud_init_user_data(config: &ProvisionConfig) -> Option<String> {
    let host = config.hostname.as_ref().map(|host| format!("hostname: {host}\npreserve_hostname: false\nmanage_etc_hosts: localhost\n")).unwrap_or_default();
    let access = config.access.as_ref().filter(|a| a.is_effective()).map(access_cloud_init).unwrap_or_default();
    let tailscale = config.tailscale.as_ref().and_then(provisioned_tailscale).map(|(control, key)| tailscale_write_files_entry(&control, &key));
    let scrub = access.wrote_secret || tailscale.is_some();
    let top = format!("{host}{}", access.top);
    let write_files: Vec<String> = access.write_files.into_iter().chain(tailscale).collect();
    let runcmd: Vec<String> = access.runcmd.into_iter().chain(scrub.then(|| SCRUB_USERDATA_ITEM.to_string())).collect();
    match top.is_empty() && write_files.is_empty() && runcmd.is_empty() {
        true => None,
        false => Some(format!("#cloud-config\n{top}{}{}", section("write_files", &write_files), section("runcmd", &runcmd))),
    }
}

fn access_cloud_init(access: &AccessConfig) -> CloudAccess {
    match access.ssh {
        SshMode::KeyOnly => sanitized(&access.authorized_key)
            .map(|key| CloudAccess { top: "ssh_pwauth: false\n".into(), write_files: vec![install_key_write_files(&key)], runcmd: vec![install_key_runcmd()], wrote_secret: false })
            .unwrap_or_default(),
        SshMode::Password => sanitized(&access.password)
            .map(|password| CloudAccess {
                top: format!("ssh_pwauth: true\nchpasswd:\n  expire: false\n  users:\n    - {{name: pi, password: {}, type: text}}\n", json_string(&password)),
                wrote_secret: true,
                ..CloudAccess::default()
            })
            .unwrap_or_default(),
        SshMode::Disabled => CloudAccess { runcmd: vec![DISABLE_SSH_ITEM.to_string()], ..CloudAccess::default() },
    }
}

fn install_key_body() -> String {
    format!(
        "key={KEY_STAGE_PATH}\n\
         user=$(getent passwd 1000 | cut -d: -f1)\n\
         [ -n \"$user\" ] || user=pi\n\
         group=$(getent passwd \"$user\" | cut -d: -f4)\n\
         [ -n \"$group\" ] || group=\"$user\"\n\
         home=$(getent passwd \"$user\" | cut -d: -f6)\n\
         [ -n \"$home\" ] || home=/home/$user\n\
         landed=no\n\
         if [ -s \"$key\" ]; then\n\
         \x20 install -d -m 700 -o \"$user\" -g \"$group\" \"$home/.ssh\" \\\n\
         \x20   && cat \"$key\" >>\"$home/.ssh/authorized_keys\" \\\n\
         \x20   && chmod 600 \"$home/.ssh/authorized_keys\" \\\n\
         \x20   && chown \"$user\":\"$group\" \"$home/.ssh/authorized_keys\"\n\
         \x20 grep -qxF \"$(cat \"$key\")\" \"$home/.ssh/authorized_keys\" 2>/dev/null && landed=yes\n\
         fi\n\
         rm -f \"$key\"\n\
         mkdir -p /etc/ssh/sshd_config.d\n\
         if [ \"$landed\" = yes ]; then\n\
         \x20 want=no\n\
         \x20 rm -f {RECOVERY_MARKER}\n\
         else\n\
         \x20 want=yes\n\
         \x20 echo \"aircast: could not install the operator key for $user; passwords left on so the device stays reachable\" >{RECOVERY_MARKER}\n\
         fi\n\
         printf 'PasswordAuthentication %s\\n' \"$want\" >{ACCESS_DROPIN}\n\
         sed -i \"s/^#\\?PasswordAuthentication.*/PasswordAuthentication $want/\" /etc/ssh/sshd_config 2>/dev/null || true\n\
         systemctl try-restart ssh 2>/dev/null || systemctl try-restart sshd 2>/dev/null || true\n"
    )
}

fn install_key_script() -> String {
    format!("#!/bin/sh\n{}", install_key_body())
}

fn install_key_write_files(key: &str) -> String {
    let script: String = install_key_script().lines().map(|line| format!("      {line}\n")).collect();
    format!("  - path: {KEY_STAGE_PATH}\n    permissions: '0600'\n    content: |\n      {key}\n\x20 - path: {INSTALL_KEY_PATH}\n    permissions: '0700'\n    content: |\n{script}")
}

fn install_key_runcmd() -> String {
    format!("  - [ sh, {INSTALL_KEY_PATH} ]\n")
}

fn tailscale_write_files_entry(control_server: &str, auth_key: &str) -> String {
    let json = tailscale_state_json(control_server);
    format!("  - path: {TS_STATE_PATH}\n    permissions: '0644'\n    content: |\n      {json}\n  - path: {TS_KEY_PATH}\n    permissions: '0600'\n    content: |\n      {auth_key}\n")
}

fn network_config_v2(wifi: &WifiConfig) -> String {
    let key = wpa_psk(&wifi.ssid, &wifi.password);
    format!(
        "version: 2\n\
         wifis:\n  \
         wlan0:\n    \
         dhcp4: true\n    \
         optional: true\n    \
         regulatory-domain: \"{country}\"\n    \
         access-points:\n      \
         {ssid}:\n        \
         password: \"{key}\"\n",
        country = wifi.country,
        ssid = serde_json::Value::from(wifi.ssid.as_str()),
    )
}

fn firstrun_hostname(host: &str) -> String {
    format!(
        "CURRENT_HOSTNAME=$(cat /etc/hostname | tr -d \" \\t\\n\\r\")\n\
         if [ -f /usr/lib/raspberrypi-sys-mods/imager_custom ]; then\n\
         \x20  /usr/lib/raspberrypi-sys-mods/imager_custom set_hostname {host}\n\
         else\n\
         \x20  echo {host} >/etc/hostname\n\
         \x20  sed -i \"s/127.0.1.1.*$CURRENT_HOSTNAME/127.0.1.1\\t{host}/g\" /etc/hosts\n\
         fi\n\n"
    )
}

fn firstrun_wifi(wifi: &WifiConfig) -> String {
    let key = wpa_psk(&wifi.ssid, &wifi.password);
    format!(
        "cat >/etc/wpa_supplicant/wpa_supplicant.conf <<'WPAEOF'\n\
         ctrl_interface=DIR=/var/run/wpa_supplicant GROUP=netdev\n\
         update_config=1\n\
         country={country}\n\
         network={{\n\
         \x20  ssid={ssid}\n\
         \x20  psk={key}\n\
         }}\n\
         WPAEOF\n\
         rfkill unblock wifi\n\
         for f in /var/lib/systemd/rfkill/*:wlan ; do echo 0 >\"$f\" 2>/dev/null || true ; done\n\n",
        country = wifi.country,
        ssid = wifi.ssid.bytes().map(|b| format!("{b:02x}")).collect::<String>(),
    )
}

fn firstrun_tailscale(control: &str, key: &str) -> String {
    let json = tailscale_state_json(control);
    format!(
        "mkdir -p /var/lib/aircastd\n\
         cat >{TS_STATE_PATH} <<'AIRCASTTSEOF'\n\
         {json}\n\
         AIRCASTTSEOF\n\
         cat >{TS_KEY_PATH} <<'AIRCASTKEYEOF'\n\
         {key}\n\
         AIRCASTKEYEOF\n\
         chmod 600 {TS_KEY_PATH}\n\n",
    )
}

fn firstrun_script(config: &ProvisionConfig) -> String {
    [
        "#!/bin/bash\n\nset +e\n\n".to_string(),
        config.hostname.as_deref().map(firstrun_hostname).unwrap_or_default(),
        config.wifi.as_ref().map(firstrun_wifi).unwrap_or_default(),
        config.tailscale.as_ref().and_then(provisioned_tailscale).map(|(control, key)| firstrun_tailscale(&control, &key)).unwrap_or_default(),
        config.access.as_ref().filter(|a| a.is_effective()).map(firstrun_access).unwrap_or_default(),
        "rm -f /boot/firstrun.sh\nsed -i 's| systemd.run=.*||g' /boot/cmdline.txt 2>/dev/null || true\nexit 0\n".to_string(),
    ]
    .concat()
}

fn firstrun_access(access: &AccessConfig) -> String {
    match access.ssh {
        SshMode::KeyOnly => sanitized(&access.authorized_key)
            .map(|key| format!("printf '%s\\n' '{key}' >{KEY_STAGE_PATH}\nchmod 600 {KEY_STAGE_PATH}\n{body}systemctl enable ssh\n\n", key = shell_single_quote(&key), body = install_key_body()))
            .unwrap_or_default(),
        SshMode::Password => sanitized(&access.password)
            .map(|password| {
                format!(
                    "printf 'pi:%s\\n' '{pw}' | chpasswd\nsed -i 's/^#\\?PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config\nsystemctl enable ssh\n\n",
                    pw = shell_single_quote(&password)
                )
            })
            .unwrap_or_default(),
        SshMode::Disabled => "systemctl disable --now ssh.socket ssh.service 2>/dev/null || true\n\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IEEE_PSK: &str = "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e";

    fn cfg(init: InitFormat, with_wifi: bool, host: Option<&str>) -> ProvisionConfig {
        ProvisionConfig {
            hostname: host.map(String::from),
            wifi: with_wifi.then(|| WifiConfig { ssid: "IEEE".into(), password: "password".into(), country: "US".into() }),
            tailscale: None,
            access: None,
            init_format: init,
        }
    }

    fn ts_cfg(init: InitFormat, host: Option<&str>) -> ProvisionConfig {
        ProvisionConfig {
            hostname: host.map(String::from),
            wifi: None,
            tailscale: Some(TailscaleConfig { control_server: "https://headscale.example.com".into(), auth_key: "hskey-auth-abc123".into() }),
            access: None,
            init_format: init,
        }
    }

    fn access_cfg(init: InitFormat, access: AccessConfig) -> ProvisionConfig {
        ProvisionConfig { hostname: None, wifi: None, tailscale: None, access: Some(access), init_format: init }
    }

    fn access(ssh: SshMode, key: Option<&str>, pw: Option<&str>) -> AccessConfig {
        AccessConfig { ssh, authorized_key: key.map(String::from), password: pw.map(String::from) }
    }

    fn cloud_user_data(config: &ProvisionConfig) -> String {
        let ProvisionPlan::CloudInit { files } = plan(config) else { panic!("expected CloudInit plan") };
        files.into_iter().find(|f| f.name == "user-data").expect("user-data file").contents
    }

    fn firstrun(config: &ProvisionConfig) -> (ProvisionFile, String) {
        let ProvisionPlan::FirstRun { script, cmdline_append } = plan(config) else { panic!("expected FirstRun plan") };
        (script, cmdline_append)
    }

    #[test]
    fn wpa_psk_matches_the_ieee_80211i_test_vector() {
        assert_eq!(wpa_psk("IEEE", "password"), IEEE_PSK);
    }

    #[test]
    fn empty_config_is_noop() {
        assert_eq!(plan(&cfg(InitFormat::CloudInit, false, None)), ProvisionPlan::Noop);
    }

    #[test]
    fn cloud_init_writes_psk_not_plaintext() {
        let ProvisionPlan::CloudInit { files } = plan(&cfg(InitFormat::CloudInit, true, Some("aircast"))) else { panic!("expected CloudInit plan") };
        let net = files.iter().find(|f| f.name == "network-config").unwrap();
        assert!(net.contents.contains(IEEE_PSK));
        assert!(!net.contents.contains("password: \"password\""));
        assert!(files.iter().any(|f| f.name == "user-data" && f.contents.contains("hostname: aircast")));
        assert!(files.iter().any(|f| f.name == "meta-data"));
    }

    #[test]
    fn cloud_init_renames_the_host_in_etc_hosts_too() {
        assert!(cloud_user_data(&cfg(InitFormat::CloudInit, true, Some("aircast"))).contains("manage_etc_hosts: localhost"));
    }

    #[test]
    fn firstrun_writes_script_and_cmdline() {
        let (script, cmdline_append) = firstrun(&cfg(InitFormat::FirstRun, true, Some("aircast")));
        assert_eq!(script.name, "firstrun.sh");
        assert!(script.executable);
        assert!(script.contents.contains("set_hostname aircast"));
        assert!(script.contents.contains(&format!("psk={IEEE_PSK}")));
        assert!(cmdline_append.contains("systemd.run=/boot/firstrun.sh"));
    }

    #[test]
    fn an_ssid_with_quotes_or_backslashes_survives_both_wifi_configs() {
        let wifi = WifiConfig { ssid: "Ole\"s \\ net".into(), password: "password".into(), country: "US".into() };
        assert!(network_config_v2(&wifi).contains(r#""Ole\"s \\ net":"#), "a YAML double-quoted key escapes like JSON");
        assert!(firstrun_wifi(&wifi).contains("ssid=4f6c652273205c206e6574\n"), "wpa_supplicant takes an unquoted hex SSID");
    }

    #[test]
    fn tailscale_only_config_is_not_noop() {
        assert!(!ts_cfg(InitFormat::CloudInit, None).is_empty());
        assert!(!matches!(plan(&ts_cfg(InitFormat::CloudInit, None)), ProvisionPlan::Noop));
    }

    #[test]
    fn cloud_init_provisions_tailscale_store() {
        let user_data = cloud_user_data(&ts_cfg(InitFormat::CloudInit, None));
        assert!(user_data.contains("write_files:"));
        assert!(user_data.contains("path: /var/lib/aircastd/tailscale.json"));
        assert!(user_data.contains("{\"loginServer\":\"https://headscale.example.com\"}"));
        assert!(user_data.contains("path: /var/lib/aircastd/tailscale.authkey"));
        assert!(user_data.contains("permissions: '0600'"));
        assert!(user_data.contains("hskey-auth-abc123"));
        assert!(user_data.contains("runcmd:"));
        assert!(user_data.contains("/var/lib/cloud/instance/user-data.txt"));
    }

    #[test]
    fn firstrun_provisions_tailscale_store() {
        let (script, _) = firstrun(&ts_cfg(InitFormat::FirstRun, None));
        assert!(script.contents.contains("cat >/var/lib/aircastd/tailscale.json"));
        assert!(script.contents.contains("{\"loginServer\":\"https://headscale.example.com\"}"));
        assert!(script.contents.contains("cat >/var/lib/aircastd/tailscale.authkey"));
        assert!(script.contents.contains("hskey-auth-abc123"));
        assert!(script.contents.contains("chmod 600 /var/lib/aircastd/tailscale.authkey"));
    }

    #[test]
    fn json_string_escapes_quotes_and_backslashes() {
        assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn access_key_only_with_no_key_is_noop() {
        let config = access_cfg(InitFormat::CloudInit, access(SshMode::KeyOnly, None, None));
        assert!(config.is_empty());
        assert_eq!(plan(&config), ProvisionPlan::Noop);
    }

    #[test]
    fn access_password_with_no_password_is_noop() {
        assert!(access_cfg(InitFormat::CloudInit, access(SshMode::Password, None, None)).is_empty());
    }

    #[test]
    fn access_disabled_is_always_effective() {
        let config = access_cfg(InitFormat::CloudInit, access(SshMode::Disabled, None, None));
        assert!(!config.is_empty());
        let user_data = cloud_user_data(&config);
        assert!(user_data.contains("runcmd:"));
        assert!(user_data.contains("systemctl disable --now ssh.socket ssh.service"));
    }

    #[test]
    fn access_key_only_disables_password_auth_and_adds_key() {
        let user_data = cloud_user_data(&access_cfg(InitFormat::CloudInit, access(SshMode::KeyOnly, Some("ssh-ed25519 AAAAkey operator@base"), None)));
        assert!(user_data.contains("ssh_pwauth: false"));
        assert!(user_data.contains(&format!("  - path: {KEY_STAGE_PATH}")));
        assert!(user_data.contains("      ssh-ed25519 AAAAkey operator@base\n"));
        assert!(user_data.contains(&format!("  - [ sh, {INSTALL_KEY_PATH} ]")));
        assert!(!user_data.contains("rm -f /boot/firmware/user-data"));
    }

    #[test]
    fn key_only_never_leaves_the_key_to_cloud_inits_default_user() {
        let user_data = cloud_user_data(&access_cfg(InitFormat::CloudInit, access(SshMode::KeyOnly, Some("ssh-ed25519 AAAAkey operator@base"), None)));
        assert!(!user_data.contains("ssh_authorized_keys:"));
        assert!(user_data.contains("  - [ sh, /root/aircast-install-key.sh ]"));
    }

    #[test]
    fn install_key_script_resolves_the_user_instead_of_assuming_one() {
        let script = install_key_script();
        assert!(script.contains("user=$(getent passwd 1000 | cut -d: -f1)"));
        assert!(script.contains("home=$(getent passwd \"$user\" | cut -d: -f6)"));
        assert!(script.contains("group=$(getent passwd \"$user\" | cut -d: -f4)"));
        assert!(script.contains("install -d -m 700 -o \"$user\" -g \"$group\" \"$home/.ssh\""));
        assert!(!script.contains("-g \"$user\""), "a group is not always named after its user");
    }

    #[test]
    fn install_key_script_puts_passwords_back_when_the_key_does_not_land() {
        let script = install_key_script();
        assert!(!script.contains("|| exit 0"), "a missing staged key is a failed install, not a reason to leave passwords off");
        let (landed, failed) = script.rsplit_once("else").expect("the script branches on whether the key landed");
        assert!(landed.contains("want=no"));
        assert!(failed.contains("want=yes"));
        assert!(failed.contains(RECOVERY_MARKER));
    }

    #[test]
    fn key_only_user_data_is_valid_cloud_config() {
        let user_data = cloud_user_data(&access_cfg(InitFormat::CloudInit, access(SshMode::KeyOnly, Some("ssh-ed25519 AAAAkey operator@base"), None)));
        let doc: serde_norway::Value = serde_norway::from_str(&user_data).expect("valid YAML");
        assert_eq!(doc["ssh_pwauth"].as_bool(), Some(false));
        assert!(doc.get("ssh_authorized_keys").is_none());
        let files = doc["write_files"].as_sequence().expect("write_files is a list");
        let staged = files.iter().find(|f| f["path"].as_str() == Some(KEY_STAGE_PATH)).expect("the key is staged");
        assert_eq!(staged["content"].as_str(), Some("ssh-ed25519 AAAAkey operator@base\n"));
        assert_eq!(staged["permissions"].as_str(), Some("0600"));
        let script = files.iter().find(|f| f["path"].as_str() == Some(INSTALL_KEY_PATH)).expect("the script is written");
        assert!(script["content"].as_str().unwrap().starts_with("#!/bin/sh\n"));
        let run = doc["runcmd"].as_sequence().expect("runcmd is a list");
        let argv: Vec<&str> = run[0].as_sequence().expect("runcmd item is the argv list form").iter().map(|v| v.as_str().expect("argv entries are strings")).collect();
        assert_eq!(argv, vec!["sh", INSTALL_KEY_PATH]);
    }

    #[test]
    fn recovery_beats_cloud_inits_own_sshd_drop_in() {
        assert!(ACCESS_DROPIN.starts_with("/etc/ssh/sshd_config.d/"));
        let name = ACCESS_DROPIN.rsplit('/').next().unwrap();
        assert!(name < "50-cloud-init.conf", "{name} must sort before it");
        assert!(install_key_script().contains(ACCESS_DROPIN));
    }

    #[test]
    fn install_key_script_checks_the_key_itself_not_just_a_non_empty_file() {
        assert!(install_key_script().contains("grep -qxF \"$(cat \"$key\")\" \"$home/.ssh/authorized_keys\""));
    }

    #[test]
    fn both_init_formats_run_the_same_install_body() {
        let key_only = access(SshMode::KeyOnly, Some("ssh-ed25519 AAAAkey operator@base"), None);
        let cloud = cloud_user_data(&access_cfg(InitFormat::CloudInit, key_only.clone()));
        let (script, _) = firstrun(&access_cfg(InitFormat::FirstRun, key_only));
        install_key_body().lines().filter(|l| !l.trim().is_empty()).for_each(|line| {
            assert!(script.contents.contains(line), "firstrun is missing: {line}");
            assert!(cloud.contains(line.trim()), "cloud-init is missing: {line}");
        });
    }

    #[test]
    fn access_password_sets_pi_password_and_scrubs_userdata() {
        let user_data = cloud_user_data(&access_cfg(InitFormat::CloudInit, access(SshMode::Password, None, Some("s3cret-pass"))));
        assert!(user_data.contains("ssh_pwauth: true"));
        assert!(user_data.contains("chpasswd:"));
        assert!(user_data.contains("{name: pi, password: \"s3cret-pass\", type: text}"));
        assert!(user_data.contains("rm -f /boot/firmware/user-data"));
    }

    #[test]
    fn access_value_newlines_are_stripped() {
        let user_data = cloud_user_data(&access_cfg(InitFormat::CloudInit, access(SshMode::KeyOnly, Some("ssh-ed25519 KEY\nssh_pwauth: true"), None)));
        assert!(user_data.contains("      ssh-ed25519 KEYssh_pwauth: true\n"));
        assert!(!user_data.lines().any(|l| l == "ssh_pwauth: true"));
    }

    #[test]
    fn disabled_ssh_and_tailscale_share_one_runcmd() {
        let config = ProvisionConfig { access: Some(access(SshMode::Disabled, None, None)), ..ts_cfg(InitFormat::CloudInit, None) };
        let user_data = cloud_user_data(&config);
        assert_eq!(user_data.matches("runcmd:").count(), 1);
        assert!(user_data.contains("systemctl disable --now ssh.socket ssh.service"));
        assert!(user_data.contains("rm -f /boot/firmware/user-data"));
    }

    #[test]
    fn firstrun_password_sets_pi_password() {
        let (script, _) = firstrun(&access_cfg(InitFormat::FirstRun, access(SshMode::Password, None, Some("s3cret"))));
        assert!(script.contents.contains("printf 'pi:%s\\n' 's3cret' | chpasswd"));
        assert!(script.contents.contains("PasswordAuthentication yes"));
    }

    #[test]
    fn firstrun_key_only_escapes_single_quotes() {
        let (script, _) = firstrun(&access_cfg(InitFormat::FirstRun, access(SshMode::KeyOnly, Some("key-with-'quote"), None)));
        assert!(script.contents.contains(r"key-with-'\''quote"));
        assert!(script.contents.contains("PasswordAuthentication %s"));
        assert!(!script.contents.contains("-o pi -g pi"));
    }

    #[test]
    fn empty_auth_key_is_not_provisioned() {
        let config = ProvisionConfig { tailscale: Some(TailscaleConfig { control_server: "https://headscale.example.com".into(), auth_key: "".into() }), ..ts_cfg(InitFormat::CloudInit, None) };
        let ProvisionPlan::CloudInit { files } = plan(&config) else { panic!("expected CloudInit plan") };
        assert!(files.iter().all(|f| !f.contents.contains("write_files")));
    }

    #[test]
    fn tailscale_values_are_stripped_to_a_single_line() {
        let config = ProvisionConfig { tailscale: Some(TailscaleConfig { control_server: "https://h\nevil: true".into(), auth_key: "hskey\nmore".into() }), ..ts_cfg(InitFormat::CloudInit, None) };
        let user_data = cloud_user_data(&config);
        assert!(user_data.contains("https://hevil: true"));
        assert!(user_data.contains("hskeymore"));
        assert!(!user_data.contains("\nevil: true"));
    }
}
