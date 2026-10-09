use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::cardimage::{self, CANCELLED_BY_USER, Release};
use crate::cardwrite::{self, BlockDevice, Stage};
use crate::flasherform::{self, Form, Remembered};
use crate::platformdisk::{self, Card, Disk, Opening};

pub const DEPS: &[&str] = &[];
pub const FLASHER_OPEN: &str = "flasher.open";
pub const FLASHER_CHANNEL: &str = "flasher.channel";
pub const FLASHER_RELEASES: &str = "flasher.releases";
pub const FLASHER_SELECT: &str = "flasher.select";
pub const FLASHER_FORM: &str = "flasher.form";
pub const FLASHER_DISKS: &str = "flasher.disks";
pub const FLASHER_CHOOSE_DISK: &str = "flasher.chooseDisk";
pub const FLASHER_START: &str = "flasher.start";
pub const FLASHER_CANCEL: &str = "flasher.cancel";
pub const FLASHER_AGAIN: &str = "flasher.again";
pub const FLASHER_DOWNLOAD: &str = "flasher.download";
const FLASHER_COMMANDS: &[&str] = &[FLASHER_OPEN, FLASHER_CHANNEL, FLASHER_RELEASES, FLASHER_SELECT, FLASHER_FORM, FLASHER_DISKS, FLASHER_CHOOSE_DISK, FLASHER_START, FLASHER_CANCEL, FLASHER_AGAIN, FLASHER_DOWNLOAD];
const SETTINGS_KEY: &str = "Flasher/settings";
const FOLDER: &str = "Flasher";
const DEFAULT_CHANNEL: &str = "stable";
const SPEED_WINDOW: Duration = Duration::from_secs(1);
const PERMISSION_POLL: Duration = Duration::from_millis(500);
const PERMISSION_WAIT: Duration = Duration::from_secs(120);
const DOWNLOAD_POLL: Duration = Duration::from_millis(200);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Releases {
    #[default]
    Idle,
    Loading,
    Ready(Vec<Release>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Download {
    #[default]
    Idle,
    Running { version: String, done: u64, total: u64 },
    Ready { version: String, path: PathBuf, image_len: u64 },
    Failed { version: String, error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum CardState {
    #[default]
    None,
    Opening(String),
    Waiting(String),
    Ready { id: String, label: String, capacity: u64 },
    Failed { id: String, error: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Phase {
    #[default]
    Idle,
    Downloading,
    Preparing,
    Writing,
    Verifying,
    Customizing,
    Done,
    Failed,
    Cancelled,
}

impl Phase {
    fn token(self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Downloading => "downloading",
            Phase::Preparing => "preparing",
            Phase::Writing => "writing",
            Phase::Verifying => "verifying",
            Phase::Customizing => "customizing",
            Phase::Done => "done",
            Phase::Failed => "failed",
            Phase::Cancelled => "cancelled",
        }
    }

    fn busy(self) -> bool {
        matches!(self, Phase::Downloading | Phase::Preparing | Phase::Writing | Phase::Verifying | Phase::Customizing)
    }

    fn finished(self) -> bool {
        matches!(self, Phase::Done | Phase::Failed | Phase::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Job {
    phase: Phase,
    done: u64,
    total: u64,
    speed: u64,
    error: Option<String>,
    hostname: String,
}

#[derive(Debug, Clone, Default)]
struct State {
    loaded: bool,
    channel: String,
    releases: Releases,
    selected: Option<String>,
    form: Form,
    remembered: Remembered,
    disks: Vec<Disk>,
    last_disk: Option<String>,
    card: CardState,
    download: Download,
    job: Job,
    hostable: bool,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));
static CARD: Mutex<Option<Card>> = Mutex::new(None);
static JOB_CANCEL: AtomicBool = AtomicBool::new(false);
static DOWNLOAD_TOKEN: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

fn state() -> MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn card_slot() -> MutexGuard<'static, Option<Card>> {
    CARD.lock().unwrap_or_else(PoisonError::into_inner)
}

fn ok() -> Value {
    json!({ "ok": true })
}

fn refused(reason: &str) -> Value {
    json!({ "ok": false, "reason": reason })
}

fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    let _ = std::thread::Builder::new().name(name.into()).spawn(move || {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
    });
}

fn remembered() -> Remembered {
    crate::settingsstore::stored_text(SETTINGS_KEY).and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

fn remember(remembered: &Remembered) {
    if let Ok(text) = serde_json::to_string(remembered) {
        crate::settingsstore::written(SETTINGS_KEY, &text);
    }
}

fn folder() -> PathBuf {
    crate::settingsstore::folder().unwrap_or_else(std::env::temp_dir).join(FOLDER)
}

pub fn owns(path: &str) -> bool {
    FLASHER_COMMANDS.contains(&path)
}

pub fn run(path: &str, args: &str) -> Value {
    let given: Vec<Value> = serde_json::from_str(args).unwrap_or_default();
    let text = |at: usize| given.get(at).and_then(Value::as_str).unwrap_or_default().to_string();
    let busy = state().job.phase.busy();
    match path {
        FLASHER_CANCEL => cancel(),
        FLASHER_OPEN => open(),
        _ if busy => refused("A card is being written"),
        FLASHER_CHANNEL => channel(&text(0)),
        FLASHER_RELEASES => {
            load_releases();
            ok()
        }
        FLASHER_SELECT => select(&text(0)),
        FLASHER_DOWNLOAD => download(),
        FLASHER_FORM => form(given.first().cloned().unwrap_or(Value::Null)),
        FLASHER_DISKS => {
            refresh_disks();
            ok()
        }
        FLASHER_CHOOSE_DISK => choose_disk(&text(0)),
        FLASHER_START => start(),
        FLASHER_AGAIN => again(),
        _ => refused("Unknown flasher command"),
    }
}

fn open() -> Value {
    let reload = {
        let mut s = state();
        if !s.loaded {
            let remembered = remembered();
            s.form = flasherform::form_from(&remembered);
            s.channel = Some(remembered.channel.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHANNEL.into());
            s.remembered = remembered;
            s.loaded = true;
        }
        s.hostable = platformdisk::installed();
        matches!(s.releases, Releases::Idle | Releases::Failed(_))
    };
    if reload {
        load_releases();
    }
    refresh_disks();
    ok()
}

fn channel(name: &str) -> Value {
    match cardimage::releases_url(name) {
        None => refused("Unknown channel"),
        Some(_) => {
            {
                let mut s = state();
                s.channel = name.to_string();
                s.selected = None;
                s.remembered.channel = name.to_string();
                remember(&s.remembered);
            }
            load_releases();
            ok()
        }
    }
}

fn load_releases() {
    let channel = {
        let mut s = state();
        s.releases = Releases::Loading;
        Some(s.channel.clone()).filter(|c| !c.is_empty()).unwrap_or_else(|| DEFAULT_CHANNEL.into())
    };
    spawn("flasher-releases", move || {
        let fetched = cardimage::fetch_releases(&channel);
        let mut s = state();
        if s.channel == channel || s.channel.is_empty() {
            s.selected = s.selected.clone().filter(|version| fetched.as_ref().is_ok_and(|list| list.iter().any(|r| &r.version == version))).or_else(|| fetched.as_ref().ok().and_then(|list| recommended(list)).map(|r| r.version.clone()));
            s.releases = fetched.map_or_else(Releases::Failed, Releases::Ready);
        }
    });
}

fn selected_release(s: &State) -> Option<Release> {
    match &s.releases {
        Releases::Ready(list) => s.selected.as_ref().and_then(|version| list.iter().find(|r| &r.version == version)).cloned(),
        _ => None,
    }
}

fn select(version: &str) -> Value {
    let mut s = state();
    let known = matches!(&s.releases, Releases::Ready(list) if list.iter().any(|r| r.version == version));
    match known {
        false => refused("That version is not in the release list"),
        true => {
            s.selected = Some(version.to_string());
            ok()
        }
    }
}

fn download() -> Value {
    let release = selected_release(&state());
    match release {
        None => refused("Choose an Aircast OS version"),
        Some(release) => {
            prefetch(release);
            ok()
        }
    }
}

fn prefetch(release: Release) {
    let wanted = match &state().download {
        Download::Running { version, .. } | Download::Ready { version, .. } => version != &release.version,
        _ => true,
    };
    if wanted {
        let token = Arc::new(AtomicBool::new(false));
        if let Some(previous) = DOWNLOAD_TOKEN.lock().unwrap_or_else(PoisonError::into_inner).replace(token.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
        spawn("flasher-download", move || {
            let _ = fetch_release(&release, &token);
        });
    }
}

fn fetch_release(release: &Release, cancel: &AtomicBool) -> Result<(PathBuf, u64), String> {
    let version = release.version.clone();
    let mine = |download: &Download| matches!(download, Download::Running { version: v, .. } if *v == version);
    state().download = Download::Running { version: version.clone(), done: 0, total: release.image.size };
    let fetched = cardimage::download(&release.image, &folder(), &|| cancel.load(Ordering::Relaxed), &mut |done, total| {
        let mut s = state();
        if mine(&s.download) {
            s.download = Download::Running { version: version.clone(), done, total };
        }
    })
    .and_then(|path| cardimage::uncompressed_len(&path).map(|image_len| (path, image_len)));
    let mut s = state();
    if mine(&s.download) {
        s.download = match &fetched {
            Ok((path, image_len)) => Download::Ready { version: version.clone(), path: path.clone(), image_len: *image_len },
            Err(error) => Download::Failed { version: version.clone(), error: error.clone() },
        };
    }
    fetched
}

fn form(patch: Value) -> Value {
    let mut s = state();
    s.form = merged(&s.form, &patch);
    ok()
}

fn merged(form: &Form, patch: &Value) -> Form {
    let current = serde_json::to_value(form).unwrap_or(Value::Null);
    let combined: serde_json::Map<String, Value> = current.as_object().into_iter().flatten().chain(patch.as_object().into_iter().flatten()).map(|(k, v)| (k.clone(), v.clone())).collect();
    serde_json::from_value(Value::Object(combined)).unwrap_or_else(|_| form.clone())
}

fn refresh_disks() {
    let disks = platformdisk::disks();
    let mut s = state();
    let held = match &s.card {
        CardState::Opening(id) | CardState::Waiting(id) => Some(id.clone()),
        CardState::Ready { id, .. } | CardState::Failed { id, .. } => Some(id.clone()),
        CardState::None => None,
    };
    if held.is_some_and(|id| !disks.iter().any(|d| d.id == id)) && !s.job.phase.busy() {
        s.card = CardState::None;
        card_slot().take();
    }
    s.disks = disks;
}

fn choose_disk(id: &str) -> Value {
    let known = state().disks.iter().any(|d| d.id == id) || platformdisk::disks().iter().any(|d| d.id == id);
    match known {
        false => refused("That card reader is not connected"),
        true => {
            card_slot().take();
            {
                let mut s = state();
                s.card = CardState::Opening(id.to_string());
                s.last_disk = Some(id.to_string());
            }
            let id = id.to_string();
            spawn("flasher-card", move || open_card(&id));
            ok()
        }
    }
}

fn open_card(id: &str) {
    let deadline = Instant::now() + PERMISSION_WAIT;
    let still_mine = || matches!(&state().card, CardState::Opening(held) | CardState::Waiting(held) if held == id);
    loop {
        if !still_mine() {
            return;
        }
        match platformdisk::open(id) {
            Opening::Ready(card) => {
                let mut s = state();
                if matches!(&s.card, CardState::Opening(held) | CardState::Waiting(held) if held == id) {
                    s.card = CardState::Ready { id: id.to_string(), label: card.label.clone(), capacity: card.capacity };
                    *card_slot() = Some(card);
                }
                return;
            }
            Opening::Failed(error) => {
                let mut s = state();
                if matches!(&s.card, CardState::Opening(held) | CardState::Waiting(held) if held == id) {
                    s.card = CardState::Failed { id: id.to_string(), error };
                }
                return;
            }
            Opening::PermissionPending if Instant::now() >= deadline => {
                state().card = CardState::Failed { id: id.to_string(), error: "Access to the card reader was not allowed".into() };
                return;
            }
            Opening::PermissionPending => {
                state().card = CardState::Waiting(id.to_string());
                std::thread::sleep(PERMISSION_POLL);
            }
        }
    }
}

fn required_bytes(s: &State) -> u64 {
    match (&s.download, selected_release(s)) {
        (Download::Ready { version, image_len, .. }, Some(release)) if *version == release.version => *image_len,
        (_, Some(release)) => release.image.uncompressed_size.unwrap_or(cardimage::MIN_CARD_BYTES),
        (_, None) => cardimage::MIN_CARD_BYTES,
    }
}

fn blocked(s: &State) -> Option<String> {
    let problem = flasherform::problems(&s.form).first().map(|(_, message)| message.to_string());
    let card = match &s.card {
        CardState::None => Some("Connect a USB card reader with the card in it, then choose it".to_string()),
        CardState::Opening(_) => Some("Checking the card reader…".to_string()),
        CardState::Waiting(_) => Some("Allow access to the card reader".to_string()),
        CardState::Failed { error, .. } => Some(error.clone()),
        CardState::Ready { capacity, .. } if *capacity < required_bytes(s) => Some(format!("The card holds {}; the image needs {}", size_text(*capacity), size_text(required_bytes(s)))),
        CardState::Ready { .. } => None,
    };
    [
        (!s.hostable).then(|| "This device cannot write cards".to_string()),
        s.job.phase.busy().then(|| "A card is being written".to_string()),
        selected_release(s).is_none().then(|| "Choose an Aircast OS version".to_string()),
        problem,
        card,
    ]
    .into_iter()
    .flatten()
    .next()
}

fn start() -> Value {
    let (release, form, reason) = {
        let s = state();
        (selected_release(&s), s.form.clone(), blocked(&s))
    };
    match (reason, release) {
        (Some(reason), _) => refused(&reason),
        (None, None) => refused("Choose an Aircast OS version"),
        (None, Some(release)) => {
            JOB_CANCEL.store(false, Ordering::Relaxed);
            state().job = Job { phase: Phase::Downloading, hostname: form.hostname.trim().to_string(), ..Job::default() };
            spawn("flasher-job", move || run_job(&release, &form));
            ok()
        }
    }
}

fn cancel() -> Value {
    match state().job.phase.busy() {
        true => {
            JOB_CANCEL.store(true, Ordering::Relaxed);
            ok()
        }
        false => refused("Nothing is being written"),
    }
}

fn again() -> Value {
    let reopen = {
        let mut s = state();
        match s.job.phase.finished() {
            false => None,
            true => {
                if s.job.phase == Phase::Done {
                    s.form.hostname = flasherform::next_free_hostname(&s.job.hostname, &s.remembered.flashed_hostnames);
                }
                s.job = Job::default();
                s.last_disk.clone().filter(|id| s.disks.iter().any(|d| &d.id == id))
            }
        }
    };
    if let Some(id) = reopen {
        choose_disk(&id);
    }
    ok()
}

fn set_phase(phase: Phase) {
    let mut s = state();
    s.job = Job { phase, done: 0, total: 0, speed: 0, ..s.job.clone() };
}

fn ensure_download(release: &Release) -> Result<(PathBuf, u64), String> {
    loop {
        if JOB_CANCEL.load(Ordering::Relaxed) {
            return Err(CANCELLED_BY_USER.to_string());
        }
        let current = state().download.clone();
        match current {
            Download::Ready { version, path, image_len } if version == release.version && path.is_file() => return Ok((path, image_len)),
            Download::Running { version, done, total } if version == release.version => {
                {
                    let mut s = state();
                    s.job = Job { done, total, ..s.job.clone() };
                }
                std::thread::sleep(DOWNLOAD_POLL);
            }
            _ => return fetch_release(release, &JOB_CANCEL),
        }
    }
}

fn run_job(release: &Release, form: &Form) {
    let outcome = job_steps(release, form);
    card_slot().take();
    let mut s = state();
    s.card = CardState::None;
    s.job = match outcome {
        Ok(()) => {
            s.remembered = flasherform::remembered_after_flash(&s.remembered, form);
            remember(&s.remembered);
            Job { phase: Phase::Done, error: None, ..s.job.clone() }
        }
        Err(_) if JOB_CANCEL.load(Ordering::Relaxed) => Job { phase: Phase::Cancelled, error: None, ..s.job.clone() },
        Err(error) => Job { phase: Phase::Failed, error: Some(error), ..s.job.clone() },
    };
}

fn job_steps(release: &Release, form: &Form) -> Result<(), String> {
    let (path, image_len) = ensure_download(release)?;
    set_phase(Phase::Preparing);
    let mut card = card_slot().take().ok_or_else(|| "The card reader was disconnected. Choose it again.".to_string())?;
    match card.capacity >= image_len {
        false => Err(format!("The card holds {}; the image needs {}", size_text(card.capacity), size_text(image_len))),
        true => {
            let mut speed = Speed::default();
            write_card(card.device(), &path, image_len, &flasherform::provision(form), &JOB_CANCEL, &mut |phase, done, total| {
                let bps = speed.sample(phase, done, Instant::now());
                let mut s = state();
                s.job = Job { phase, done, total, speed: bps, ..s.job.clone() };
            })
        }
    }
}

pub fn write_card(device: &mut (dyn BlockDevice + Send), image: &Path, image_len: u64, provision: &crate::cardprovision::ProvisionConfig, cancel: &AtomicBool, progress: &mut dyn FnMut(Phase, u64, u64)) -> Result<(), String> {
    let mut reader = Cancellable { inner: cardimage::open_image(image)?, cancel };
    let mut guarded = CancellableDevice { inner: device, cancel };
    cardwrite::flash(&mut guarded, &mut reader, &cardwrite::Params { image_len, verify: true }, provision, &mut |stage, done, total| progress(phase_of(stage), done, total)).map_err(|e| match cancel.load(Ordering::Relaxed) {
        true => CANCELLED_BY_USER.to_string(),
        false => e,
    })
}

fn phase_of(stage: Stage) -> Phase {
    match stage {
        Stage::Write => Phase::Writing,
        Stage::Verify => Phase::Verifying,
        Stage::Customize => Phase::Customizing,
    }
}

struct Cancellable<'a, R: Read> {
    inner: R,
    cancel: &'a AtomicBool,
}

impl<R: Read> Read for Cancellable<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.cancel.load(Ordering::Relaxed) {
            true => Err(io::Error::other(CANCELLED_BY_USER)),
            false => self.inner.read(buf),
        }
    }
}

struct CancellableDevice<'a> {
    inner: &'a mut (dyn BlockDevice + Send),
    cancel: &'a AtomicBool,
}

impl CancellableDevice<'_> {
    fn check(&self) -> io::Result<()> {
        match self.cancel.load(Ordering::Relaxed) {
            true => Err(io::Error::other(CANCELLED_BY_USER)),
            false => Ok(()),
        }
    }
}

impl Read for CancellableDevice<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.check()?;
        self.inner.read(buf)
    }
}

impl Write for CancellableDevice<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.check()?;
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for CancellableDevice<'_> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl BlockDevice for CancellableDevice<'_> {
    fn sync(&mut self) -> io::Result<()> {
        self.inner.sync()
    }
}

#[derive(Default)]
struct Speed {
    phase: Option<Phase>,
    since: Option<Instant>,
    from: u64,
    bps: u64,
}

impl Speed {
    fn sample(&mut self, phase: Phase, done: u64, now: Instant) -> u64 {
        match (self.phase == Some(phase) && done >= self.from, self.since) {
            (true, Some(since)) if now.duration_since(since) >= SPEED_WINDOW => {
                self.bps = ((done - self.from) as f64 / now.duration_since(since).as_secs_f64()) as u64;
                self.since = Some(now);
                self.from = done;
                self.bps
            }
            (true, Some(_)) => self.bps,
            _ => {
                *self = Speed { phase: Some(phase), since: Some(now), from: done, bps: 0 };
                0
            }
        }
    }
}

pub fn size_text(bytes: u64) -> String {
    match bytes {
        b if b >= 1_000_000_000 => format!("{:.1} GB", b as f64 / 1e9),
        b if b >= 1_000_000 => format!("{:.0} MB", b as f64 / 1e6),
        b => format!("{} KB", b.div_ceil(1000)),
    }
}

fn percent(done: u64, total: u64) -> f64 {
    match total {
        0 => 0.0,
        t => (done as f64 / t as f64 * 100.0).min(100.0),
    }
}

fn recommended(list: &[Release]) -> Option<&Release> {
    list.iter().find(|r| !r.prerelease).or(list.first())
}

fn release_json(release: &Release, recommended: bool) -> Value {
    let label = format!("Aircast OS {}{}", release.version, if release.prerelease { " (beta)" } else { "" });
    json!({
        "recommended": recommended,
        "version": release.version,
        "label": label,
        "prerelease": release.prerelease,
        "published": release.created_at.split('T').next().unwrap_or_default(),
        "size": release.image.size,
        "sizeText": size_text(release.image.size),
    })
}

fn render(s: &State) -> Value {
    let (releases_state, items, releases_error) = match &s.releases {
        Releases::Idle => ("idle", vec![], Value::Null),
        Releases::Loading => ("loading", vec![], Value::Null),
        Releases::Ready(list) => ("ready", list.iter().map(|r| release_json(r, recommended(list).is_some_and(|best| best.version == r.version))).collect(), Value::Null),
        Releases::Failed(e) => ("failed", vec![], json!(e)),
    };
    let download = match &s.download {
        Download::Idle => json!({ "state": "idle" }),
        Download::Running { version, done, total } => json!({ "state": "running", "version": version, "done": done, "total": total, "percent": percent(*done, *total) }),
        Download::Ready { version, image_len, .. } => json!({ "state": "ready", "version": version, "imageSize": image_len, "imageSizeText": size_text(*image_len) }),
        Download::Failed { version, error } => json!({ "state": "failed", "version": version, "error": error }),
    };
    let card = match &s.card {
        CardState::None => json!({ "state": "none" }),
        CardState::Opening(id) => json!({ "state": "opening", "id": id }),
        CardState::Waiting(id) => json!({ "state": "waiting", "id": id }),
        CardState::Ready { id, label, capacity } => json!({ "state": "ready", "id": id, "label": label, "capacity": capacity, "capacityText": size_text(*capacity) }),
        CardState::Failed { id, error } => json!({ "state": "failed", "id": id, "error": error }),
    };
    let image = selected_release(s).map(|r| release_json(&r, false)["label"].as_str().unwrap_or_default().to_string()).unwrap_or_default();
    let reason = blocked(s);
    json!({
        "available": s.hostable,
        "channel": s.channel,
        "channels": cardimage::RELEASE_CHANNELS.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        "releases": { "state": releases_state, "items": items, "error": releases_error },
        "selected": s.selected,
        "form": s.form,
        "problems": flasherform::problems(&s.form).into_iter().map(|(field, message)| (field.to_string(), json!(message))).collect::<serde_json::Map<String, Value>>(),
        "keyIdentity": flasherform::identify_key(&s.form.authorized_key),
        "summary": flasherform::summary(&image, &s.form),
        "disks": s.disks.iter().map(|d| json!({ "id": d.id, "name": d.name })).collect::<Vec<_>>(),
        "card": card,
        "download": download,
        "job": {
            "phase": s.job.phase.token(),
            "busy": s.job.phase.busy(),
            "cancellable": s.job.phase.busy(),
            "done": s.job.done,
            "total": s.job.total,
            "percent": percent(s.job.done, s.job.total),
            "speed": s.job.speed,
            "speedText": if s.job.speed > 0 { format!("{}/s", size_text(s.job.speed)) } else { String::new() },
            "error": s.job.error,
            "hostname": s.job.hostname,
        },
        "canStart": reason.is_none(),
        "blocked": reason,
    })
}

pub fn view(_backend: &dyn crate::router::Backend, _args: &[String]) -> Value {
    let snapshot = state().clone();
    render(&snapshot)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::cardimage::Image;
    use crate::cardwrite::tests::{MemDevice, build_image, cloud_init_config, read_fat_file};

    fn release(version: &str, prerelease: bool) -> Release {
        Release {
            version: version.into(),
            prerelease,
            created_at: "2026-08-22T15:56:38Z".into(),
            image: Image { filename: format!("aircast-{version}.img.xz"), extension: "img.xz".into(), size: 576_627_168, uncompressed_size: None, download_url: "https://example.invalid/a".into(), checksum_url: "https://example.invalid/a.sha256".into() },
        }
    }

    fn ready_state() -> State {
        State {
            hostable: true,
            channel: "stable".into(),
            releases: Releases::Ready(vec![release("v0.3.5", false)]),
            selected: Some("v0.3.5".into()),
            form: Form { hostname: "falcon-01".into(), ssid: "field-net".into(), ..Form::default() },
            card: CardState::Ready { id: "/dev/bus/usb/001/004".into(), label: "Generic SD".into(), capacity: 16_000_000_000 },
            ..State::default()
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("flasher-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_complete_setup_can_start() {
        let view = render(&ready_state());
        assert_eq!(view["canStart"], true);
        assert_eq!(view["blocked"], Value::Null);
        assert_eq!(view["releases"]["items"][0]["label"], "Aircast OS v0.3.5");
        assert_eq!(view["releases"]["items"][0]["recommended"], true);
        assert_eq!(view["summary"][1]["value"], "falcon-01.local");
        assert_eq!(view["card"]["capacityText"], "16.0 GB");
    }

    #[test]
    fn the_first_thing_missing_is_what_blocks_starting() {
        let reason = |s: State| render(&s)["blocked"].as_str().map(str::to_string);
        assert_eq!(reason(State { hostable: false, ..ready_state() }).as_deref(), Some("This device cannot write cards"));
        assert_eq!(reason(State { selected: None, ..ready_state() }).as_deref(), Some("Choose an Aircast OS version"));
        assert_eq!(reason(State { form: Form { ssid: "".into(), ..ready_state().form }, ..ready_state() }).as_deref(), Some("Enter the WiFi network name, or choose no WiFi."));
        assert_eq!(reason(State { card: CardState::None, ..ready_state() }).as_deref(), Some("Connect a USB card reader with the card in it, then choose it"));
        assert_eq!(reason(State { card: CardState::Waiting("x".into()), ..ready_state() }).as_deref(), Some("Allow access to the card reader"));
        assert_eq!(reason(State { job: Job { phase: Phase::Writing, ..Job::default() }, ..ready_state() }).as_deref(), Some("A card is being written"));
    }

    #[test]
    fn a_card_smaller_than_the_image_is_refused_with_both_sizes() {
        let small = State { card: CardState::Ready { id: "x".into(), label: "".into(), capacity: 2_000_000_000 }, ..ready_state() };
        assert_eq!(render(&small)["blocked"], "The card holds 2.0 GB; the image needs 3.5 GB");
        let measured = State { download: Download::Ready { version: "v0.3.5".into(), path: PathBuf::from("/x"), image_len: 1_500_000_000 }, ..small };
        assert_eq!(render(&measured)["canStart"], true, "a measured image smaller than the card is allowed");
    }

    #[test]
    fn job_progress_is_shown_as_a_percentage_with_speed() {
        let view = render(&State { job: Job { phase: Phase::Writing, done: 250, total: 1000, speed: 12_300_000, ..Job::default() }, ..ready_state() });
        assert_eq!(view["job"]["phase"], "writing");
        assert_eq!(view["job"]["busy"], true);
        assert_eq!(view["job"]["percent"], 25.0);
        assert_eq!(view["job"]["speedText"], "12 MB/s");
    }

    #[test]
    fn a_form_patch_changes_only_the_fields_it_names() {
        let form = Form { hostname: "falcon-01".into(), ssid: "field-net".into(), ..Form::default() };
        let patched = merged(&form, &json!({ "ssid": "other", "noWifi": true }));
        assert_eq!(patched.ssid, "other");
        assert!(patched.no_wifi);
        assert_eq!(patched.hostname, "falcon-01");
        assert_eq!(merged(&form, &json!({ "noWifi": "not a bool" })), form, "a bad patch leaves the form as it was");
        assert_eq!(merged(&form, &Value::Null), form);
    }

    #[test]
    fn speed_is_measured_over_a_window_and_restarts_with_each_phase() {
        let t0 = Instant::now();
        let mut speed = Speed::default();
        assert_eq!(speed.sample(Phase::Writing, 0, t0), 0);
        assert_eq!(speed.sample(Phase::Writing, 5_000_000, t0 + Duration::from_millis(500)), 0);
        assert_eq!(speed.sample(Phase::Writing, 10_000_000, t0 + Duration::from_secs(1)), 10_000_000);
        assert_eq!(speed.sample(Phase::Writing, 12_000_000, t0 + Duration::from_millis(1500)), 10_000_000);
        assert_eq!(speed.sample(Phase::Verifying, 0, t0 + Duration::from_secs(2)), 0);
    }

    #[test]
    fn the_newest_stable_release_is_recommended_over_a_newer_beta() {
        let view = render(&State { releases: Releases::Ready(vec![release("v0.4.0-beta.1", true), release("v0.3.5", false), release("v0.3.4", false)]), ..ready_state() });
        let flags: Vec<(String, bool)> = view["releases"]["items"].as_array().unwrap().iter().map(|r| (r["version"].as_str().unwrap().to_string(), r["recommended"].as_bool().unwrap())).collect();
        assert_eq!(flags, vec![("v0.4.0-beta.1".to_string(), false), ("v0.3.5".to_string(), true), ("v0.3.4".to_string(), false)]);
        let betas_only = render(&State { releases: Releases::Ready(vec![release("v0.4.0-beta.2", true), release("v0.4.0-beta.1", true)]), selected: None, ..ready_state() });
        assert_eq!(betas_only["releases"]["items"][0]["recommended"], true, "with no stable release the newest one is recommended");
    }

    #[test]
    fn starting_the_download_returns_instead_of_waiting_on_its_own_lock() {
        *state() = State { releases: Releases::Ready(vec![release("v9.9.9", false)]), selected: Some("v9.9.9".into()), ..State::default() };
        assert_eq!(run(FLASHER_DOWNLOAD, "[]"), ok());
        assert!(matches!(state().download, Download::Running { .. } | Download::Failed { .. } | Download::Idle));
        if let Some(token) = DOWNLOAD_TOKEN.lock().unwrap_or_else(PoisonError::into_inner).take() {
            token.store(true, Ordering::Relaxed);
        }
        *state() = State::default();
    }

    #[test]
    fn sizes_read_in_decimal_units() {
        assert_eq!(size_text(576_627_168), "577 MB");
        assert_eq!(size_text(3_500_000_000), "3.5 GB");
        assert_eq!(size_text(999), "1 KB");
    }

    #[test]
    fn a_compressed_image_is_written_verified_and_provisioned() {
        let dir = scratch("write");
        let image = build_image();
        let mut compressed = Vec::new();
        lzma_rs::xz_compress(&mut Cursor::new(image.clone()), &mut compressed).unwrap();
        let path = dir.join("aircast.img.xz");
        std::fs::write(&path, compressed).unwrap();
        let mut device = MemDevice::zeroed(image.len() + cardwrite::BLOCK);
        let mut phases: Vec<Phase> = Vec::new();
        write_card(&mut device, &path, cardimage::uncompressed_len(&path).unwrap(), &cloud_init_config(), &AtomicBool::new(false), &mut |phase, _, _| {
            if phases.last() != Some(&phase) {
                phases.push(phase);
            }
        })
        .expect("write");
        assert_eq!(phases, vec![Phase::Writing, Phase::Verifying, Phase::Customizing]);
        assert!(read_fat_file(&device.cur.into_inner(), "user-data").unwrap().contains("hostname: aircast"));
    }

    #[test]
    fn cancelling_stops_the_write_and_says_so() {
        let dir = scratch("cancel");
        let image = build_image();
        let path = dir.join("aircast.img");
        std::fs::write(&path, &image).unwrap();
        let mut device = MemDevice::zeroed(image.len());
        let error = write_card(&mut device, &path, image.len() as u64, &cloud_init_config(), &AtomicBool::new(true), &mut |_, _, _| {}).unwrap_err();
        assert_eq!(error, CANCELLED_BY_USER);
    }
}
