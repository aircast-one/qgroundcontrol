use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tauri::webview::PageLoadEvent;

const SILENT: Duration = Duration::from_secs(10);
const EMPTY: Duration = Duration::from_secs(5);
const CHECK_EVERY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Health {
    Fine,
    Silent,
    Empty,
}

#[derive(Clone, Copy, Debug)]
pub struct Watch {
    started: Option<Instant>,
    heard: Option<Instant>,
    empty_since: Option<Instant>,
    hidden: bool,
    reported: Health,
    reported_at: Option<Instant>,
    terminations: u32,
}

impl Watch {
    pub const fn new() -> Self {
        Watch {
            started: None,
            heard: None,
            empty_since: None,
            hidden: false,
            reported: Health::Fine,
            reported_at: None,
            terminations: 0,
        }
    }

    pub fn started(self, now: Instant) -> Self {
        Watch {
            started: Some(now),
            heard: None,
            empty_since: None,
            reported: Health::Fine,
            reported_at: None,
            ..self
        }
    }

    pub fn beat(self, rendered: bool, hidden: bool, now: Instant) -> Self {
        Watch {
            heard: Some(now),
            empty_since: (!rendered).then(|| self.empty_since.unwrap_or(now)),
            hidden,
            ..self
        }
    }

    pub fn terminated(self) -> Self {
        Watch {
            terminations: self.terminations + 1,
            ..self
        }
    }

    fn health(&self, now: Instant) -> Health {
        let Some(started) = self.started else {
            return Health::Fine;
        };
        let silent = now.duration_since(self.heard.unwrap_or(started)) >= SILENT;
        let empty = self
            .empty_since
            .is_some_and(|since| now.duration_since(since) >= EMPTY);
        match (silent, empty) {
            (true, _) => Health::Silent,
            (false, true) => Health::Empty,
            (false, false) => Health::Fine,
        }
    }

    pub fn checked(self, now: Instant) -> (Self, Option<String>) {
        let health = self.health(now);
        if health == self.reported {
            return (self, None);
        }
        let seconds = |since: Option<Instant>| {
            since.map_or(0.0, |since| now.duration_since(since).as_secs_f64())
        };
        let page_s = seconds(self.started);
        let line = match health {
            Health::Silent => format!(
                "event=page_silent silent_s={:.1} page_s={page_s:.1} hidden={}",
                seconds(self.heard.or(self.started)),
                self.hidden
            ),
            Health::Empty => format!(
                "event=page_empty empty_s={:.1} page_s={page_s:.1} hidden={}",
                seconds(self.empty_since),
                self.hidden
            ),
            Health::Fine => format!(
                "event=page_recovered was={} lasted_s={:.1} page_s={page_s:.1}",
                self.reported.event(),
                seconds(self.reported_at)
            ),
        };
        (
            Watch {
                reported: health,
                reported_at: Some(now),
                ..self
            },
            Some(line),
        )
    }

    pub fn termination_line(&self, now: Instant) -> String {
        let seconds = |since: Option<Instant>| {
            since.map_or(-1.0, |since| now.duration_since(since).as_secs_f64())
        };
        format!(
            "event=web_content_terminated count={} page_s={:.1} last_beat_s={:.1} rendered={} hidden={}",
            self.terminations,
            seconds(self.started),
            seconds(self.heard),
            self.empty_since.is_none(),
            self.hidden
        )
    }
}

impl Health {
    fn event(self) -> &'static str {
        match self {
            Health::Fine => "page_fine",
            Health::Silent => "page_silent",
            Health::Empty => "page_empty",
        }
    }
}

static WATCH: Mutex<Watch> = Mutex::new(Watch::new());

fn watch() -> MutexGuard<'static, Watch> {
    WATCH.lock().unwrap_or_else(PoisonError::into_inner)
}

fn update(change: impl FnOnce(Watch) -> Watch) {
    let mut held = watch();
    *held = change(*held);
}

pub fn loading(event: PageLoadEvent, url: &url::Url) {
    let phase = match event {
        PageLoadEvent::Started => {
            update(|watch| watch.started(Instant::now()));
            "started"
        }
        PageLoadEvent::Finished => "finished",
    };
    log::info!(target: "desktop.page", "event=page_load phase={phase} url={url}");
}

#[tauri::command]
pub fn page_alive(rendered: bool, hidden: bool) {
    update(|watch| watch.beat(rendered, hidden, Instant::now()));
}

pub fn keep_watch() {
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(CHECK_EVERY);
            let mut held = watch();
            let (next, line) = held.checked(Instant::now());
            *held = next;
            drop(held);
            match (line, next.reported) {
                (Some(line), Health::Fine) => log::info!(target: "desktop.page", "{line}"),
                (Some(line), _) => log::error!(target: "desktop.page", "{line}"),
                (None, _) => {}
            }
        }
    });
}

#[cfg(target_os = "macos")]
pub fn terminated<R: tauri::Runtime>(webview: &tauri::Webview<R>) {
    let mut held = watch();
    *held = held.terminated();
    let line = held.termination_line(Instant::now());
    drop(held);
    log::error!(target: "desktop.page", "{line}");
    let webview = webview.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = webview.reload() {
            log::error!(target: "desktop.page", "event=web_content_not_reloaded reason=\"{error}\"");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(watch: Watch, at: &[Instant]) -> (Watch, Vec<String>) {
        at.iter().fold((watch, Vec::new()), |(watch, seen), now| {
            let (next, line) = watch.checked(*now);
            (next, seen.into_iter().chain(line).collect())
        })
    }

    #[test]
    fn a_page_that_stops_answering_is_logged_once_and_again_when_it_answers() {
        let start = Instant::now();
        let at = |seconds: u64| start + Duration::from_secs(seconds);
        let alive = Watch::new()
            .started(start)
            .beat(true, false, at(1))
            .beat(true, false, at(3));
        let (quiet, seen) = lines(alive, &[at(5), at(12), at(13), at(14)]);
        assert_eq!(
            seen,
            ["event=page_silent silent_s=10.0 page_s=13.0 hidden=false"],
            "one line when it goes quiet, none while it stays quiet"
        );
        let (_, seen) = lines(quiet.beat(true, false, at(20)), &[at(21)]);
        assert_eq!(
            seen,
            ["event=page_recovered was=page_silent lasted_s=8.0 page_s=21.0"]
        );
    }

    #[test]
    fn a_page_that_never_runs_its_script_counts_as_silent_from_its_load() {
        let start = Instant::now();
        let (_, seen) = lines(
            Watch::new().started(start),
            &[start + Duration::from_secs(9), start + SILENT],
        );
        assert_eq!(
            seen,
            ["event=page_silent silent_s=10.0 page_s=10.0 hidden=false"]
        );
    }

    #[test]
    fn a_page_that_answers_but_draws_nothing_is_logged_as_empty() {
        let start = Instant::now();
        let at = |seconds: u64| start + Duration::from_secs(seconds);
        let blank = Watch::new()
            .started(start)
            .beat(true, false, at(1))
            .beat(false, false, at(2))
            .beat(false, false, at(4))
            .beat(false, false, at(6));
        let (blank, seen) = lines(blank, &[at(6), at(7)]);
        assert_eq!(
            seen,
            ["event=page_empty empty_s=5.0 page_s=7.0 hidden=false"],
            "empty counts from the first beat that drew nothing"
        );
        let (_, seen) = lines(blank.beat(true, false, at(8)), &[at(8)]);
        assert_eq!(
            seen,
            ["event=page_recovered was=page_empty lasted_s=1.0 page_s=8.0"]
        );
    }

    #[test]
    fn a_reload_starts_the_watch_over_and_before_any_page_nothing_is_reported() {
        let start = Instant::now();
        assert_eq!(lines(Watch::new(), &[start + SILENT * 3]).1.len(), 0);
        let (quiet, _) = lines(Watch::new().started(start), &[start + SILENT]);
        let reloaded = quiet.started(start + SILENT);
        assert_eq!(
            lines(reloaded, &[start + SILENT + Duration::from_secs(9)])
                .1
                .len(),
            0
        );
    }

    #[test]
    fn a_terminated_page_says_how_long_it_lived_and_what_it_last_showed() {
        let start = Instant::now();
        let watch = Watch::new()
            .started(start)
            .beat(true, true, start + Duration::from_secs(60))
            .terminated();
        assert_eq!(
            watch.termination_line(start + Duration::from_secs(61)),
            "event=web_content_terminated count=1 page_s=61.0 last_beat_s=1.0 rendered=true hidden=true"
        );
    }
}
