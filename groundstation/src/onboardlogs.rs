use std::io::{Seek, SeekFrom, Write};

use serde_json::{Value, json};

pub const LOG_DATA_LEN: u32 = 90;
const TABLE_BINS: u32 = 2048;
const CHUNK_SIZE: u32 = TABLE_BINS * LOG_DATA_LEN;
const LIST_TIMEOUT_MS: u64 = 5000;
const TIMEOUT_MS: u64 = 500;
const GUI_RATE_MS: u64 = 500;
const SIZE_UPDATE_BYTES: u64 = 102_400;

pub const MAVLINK_LOG_ROOT: &str = "@MAV_LOG";
pub const PX4_LOG_ROOT: &str = "/fs/microsd/log";
pub const APM_LOG_ROOT: &str = "/APM/LOGS";
pub const FTP_TRANSPORT: &str = "ftp";
const MESSAGES_TRANSPORT: &str = "messages";

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    RequestList { start: u16, end: u16 },
    RequestData { id: u16, offset: u32, count: u32 },
    RequestEnd,
    Erase,
    FtpList(String),
    FtpDownload { path: String, local: std::path::PathBuf },
    FtpDelete(String),
    FtpCancel,
}

#[derive(Debug, Default)]
struct Ftp {
    root: String,
    fallback_root: Option<&'static str>,
    tried_fallback: bool,
    listing_root: bool,
    dirs: Vec<String>,
    next_id: u16,
    downloads: Vec<u16>,
    deletes: Vec<u16>,
    deleting: bool,
    erasing: Option<u16>,
    current: Option<u16>,
    had_error: bool,
    progress_bytes: u64,
    progress_from_ms: u64,
    rate_avg: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: u16,
    pub size: u32,
    pub time_utc: Option<u32>,
    pub received: bool,
    pub selected: bool,
    pub status: String,
    pub ftp_path: Option<String>,
}

impl Entry {
    fn pending(id: u16) -> Entry {
        Entry { id, size: 0, time_utc: None, received: false, selected: false, status: "Pending".to_string(), ftp_path: None }
    }

    fn status_id(&self) -> String {
        match self.status.as_str() {
            text if text.ends_with("/s)") => "downloading".to_string(),
            text => text.to_lowercase(),
        }
    }
}

#[derive(Debug)]
struct Download {
    id: u16,
    size: u32,
    file: std::fs::File,
    path: std::path::PathBuf,
    chunk: u32,
    table: Vec<bool>,
    written: u64,
    rate_bytes: u64,
    rate_avg: f64,
    last_status_written: u64,
    elapsed_from_ms: u64,
}

impl Download {
    fn bins(&self) -> usize {
        chunk_bins(self.size, self.chunk)
    }

    fn chunks(&self) -> u32 {
        self.size.div_ceil(CHUNK_SIZE)
    }

    fn chunk_complete(&self) -> bool {
        self.table.iter().all(|bit| *bit)
    }

    fn log_complete(&self) -> bool {
        self.chunk_complete() && self.chunk + 1 == self.chunks()
    }

    fn advance(&mut self) {
        self.chunk += 1;
        self.table = vec![false; self.bins()];
    }
}

fn chunk_bins(size: u32, chunk: u32) -> usize {
    let left = f64::from(size) - f64::from(chunk) * f64::from(CHUNK_SIZE);
    ((left / f64::from(LOG_DATA_LEN)).ceil().max(0.0) as u32).min(TABLE_BINS) as usize
}

pub fn big_size_text(size: u64) -> String {
    const KB: f64 = 1024.0;
    let bytes = size as f64;
    match size {
        s if (s as f64) < KB => format!("{s}B"),
        s if (s as f64) < KB * KB => format!("{:.1}KB", bytes / KB),
        s if (s as f64) < KB * KB * KB => format!("{:.1}MB", bytes / (KB * KB)),
        s if (s as f64) < KB * KB * KB * KB => format!("{:.1}GB", bytes / (KB * KB * KB)),
        _ => format!("{:.1}TB", bytes / (KB * KB * KB * KB)),
    }
}

fn local_time(seconds: u32) -> Option<chrono::DateTime<chrono::Local>> {
    chrono::DateTime::from_timestamp(i64::from(seconds), 0).map(|utc| utc.with_timezone(&chrono::Local))
}

#[derive(Debug, Default)]
pub struct OnboardLogs {
    pub entries: Vec<Entry>,
    pub listing: bool,
    pub downloading: bool,
    pub sort_ascending: bool,
    appended: bool,
    retries: u32,
    offset: u16,
    due: Option<u64>,
    folder: std::path::PathBuf,
    extension: String,
    download: Option<Download>,
    pub ftp_capable: bool,
    ftp_disabled: bool,
    use_ftp: bool,
    ftp: Ftp,
}

pub fn ftp_entry(entry: &str, subdir: Option<&str>) -> Option<(String, u32, Option<u32>)> {
    let info = entry.strip_prefix('F')?;
    let mut fields = info.split('\t');
    let name = fields.next()?.to_string();
    let size = fields.next()?.parse::<u32>().ok()?;
    let lower = name.to_lowercase();
    if !(lower.ends_with(".ulg") || lower.ends_with(".bin")) {
        return None;
    }
    let mtime = fields.next().and_then(|m| m.parse::<i64>().ok()).filter(|m| *m > 0).and_then(|m| u32::try_from(m).ok());
    let from_dir = || {
        let date = chrono::NaiveDate::parse_from_str(subdir?, "%Y-%m-%d").ok()?;
        let stem = name.rsplit_once('.').map_or(name.as_str(), |(stem, _)| stem);
        let time = chrono::NaiveTime::parse_from_str(stem, "%H_%M_%S").unwrap_or_default();
        u32::try_from(date.and_time(time).and_utc().timestamp()).ok()
    };
    Some((name.clone(), size, mtime.or_else(from_dir)))
}

impl OnboardLogs {
    pub fn busy(&self) -> bool {
        self.listing || self.downloading
    }

    pub fn selected_count(&self) -> usize {
        self.entries.iter().filter(|e| e.received && e.selected).count()
    }

    pub fn all_selected(&self) -> bool {
        let selectable = self.entries.iter().filter(|e| e.received).count();
        selectable > 0 && self.entries.iter().filter(|e| e.received && e.selected).count() == selectable
    }

    fn request_list(&mut self, start: u16, end: u16, now_ms: u64) -> Vec<Out> {
        self.listing = true;
        self.due = Some(now_ms + LIST_TIMEOUT_MS);
        vec![Out::RequestList { start, end }]
    }

    pub fn refresh(&mut self, now_ms: u64) -> Vec<Out> {
        if self.busy() {
            return Vec::new();
        }
        self.entries.clear();
        match self.ftp_capable && !self.ftp_disabled {
            true => {
                self.use_ftp = true;
                self.listing = true;
                self.ftp = Ftp { root: MAVLINK_LOG_ROOT.to_string(), fallback_root: self.ftp.fallback_root, listing_root: true, ..Ftp::default() };
                vec![Out::FtpList(self.ftp.root.clone())]
            }
            false => {
                self.use_ftp = false;
                self.request_list(0, 0xffff, now_ms)
            }
        }
    }

    pub fn set_ftp_fallback_root(&mut self, root: Option<&'static str>) {
        self.ftp.fallback_root = root;
    }

    pub fn transport(&self) -> &'static str {
        if self.use_ftp { FTP_TRANSPORT } else { MESSAGES_TRANSPORT }
    }

    fn fall_back_to_messages(&mut self, now_ms: u64) -> Vec<Out> {
        self.ftp_disabled = true;
        self.use_ftp = false;
        self.entries.clear();
        self.request_list(0, 0xffff, now_ms)
    }

    pub fn on_ftp_listed(&mut self, listed: Result<Vec<String>, String>, time_unsupported: bool, now_ms: u64) -> Vec<Out> {
        if !self.listing || !self.use_ftp {
            return Vec::new();
        }
        let entries = match listed {
            Ok(entries) => entries,
            Err(_) if self.ftp.listing_root && !self.ftp.tried_fallback && self.ftp.fallback_root.is_some() => {
                self.ftp.tried_fallback = true;
                self.ftp.root = self.ftp.fallback_root.unwrap_or_default().to_string();
                return vec![Out::FtpList(self.ftp.root.clone())];
            }
            Err(_) => return self.fall_back_to_messages(now_ms),
        };
        match self.ftp.listing_root {
            true => {
                self.add_ftp_entries(&entries, None);
                let mut dirs: Vec<String> = entries.iter().filter_map(|e| e.strip_prefix('D')).filter(|d| !d.is_empty()).map(str::to_string).collect();
                dirs.sort();
                self.ftp.dirs = dirs;
                self.ftp.listing_root = false;
            }
            false => {
                let current = self.ftp.dirs.first().cloned();
                self.add_ftp_entries(&entries, current.as_deref());
                if !self.ftp.dirs.is_empty() {
                    self.ftp.dirs.remove(0);
                }
            }
        }
        self.next_listing(time_unsupported, now_ms)
    }

    pub fn on_ftp_list_refused(&mut self, time_unsupported: bool, now_ms: u64) -> Vec<Out> {
        if !self.listing || !self.use_ftp {
            return Vec::new();
        }
        if self.ftp.listing_root {
            return self.fall_back_to_messages(now_ms);
        }
        if !self.ftp.dirs.is_empty() {
            self.ftp.dirs.remove(0);
        }
        self.next_listing(time_unsupported, now_ms)
    }

    fn next_listing(&mut self, time_unsupported: bool, now_ms: u64) -> Vec<Out> {
        match self.ftp.dirs.first() {
            Some(dir) => vec![Out::FtpList(format!("{}/{dir}", self.ftp.root))],
            None if time_unsupported => self.fall_back_to_messages(now_ms),
            None => {
                self.finish_listing();
                Vec::new()
            }
        }
    }

    fn add_ftp_entries(&mut self, listed: &[String], subdir: Option<&str>) {
        let found: Vec<Entry> = listed
            .iter()
            .filter_map(|entry| ftp_entry(entry, subdir))
            .enumerate()
            .map(|(at, (name, size, time_utc))| Entry {
                id: self.ftp.next_id + at as u16,
                size,
                time_utc,
                received: true,
                selected: false,
                status: "Available".to_string(),
                ftp_path: Some(match subdir {
                    Some(dir) => format!("{}/{dir}/{name}", self.ftp.root),
                    None => format!("{}/{name}", self.ftp.root),
                }),
            })
            .collect();
        self.ftp.next_id += found.len() as u16;
        self.appended = self.appended || !found.is_empty();
        self.entries.extend(found);
    }

    fn sort(&mut self) {
        let ascending = self.sort_ascending;
        let by_id = |a: &Entry, b: &Entry| if ascending { a.id.cmp(&b.id) } else { b.id.cmp(&a.id) };
        let timed = |e: &Entry| e.received && e.time_utc.is_some_and(|t| t > 0);
        self.entries.sort_by(|a, b| match (timed(a), timed(b)) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            (true, true) if a.time_utc != b.time_utc => if ascending { a.time_utc.cmp(&b.time_utc) } else { b.time_utc.cmp(&a.time_utc) },
            _ => by_id(a, b),
        });
    }

    fn finish_listing(&mut self) {
        self.due = None;
        if self.listing {
            self.listing = false;
            self.sort();
        }
    }

    pub fn on_entry(&mut self, apm: bool, time_utc: u32, size: u32, id: u16, num_logs: u16, now_ms: u64) {
        if !self.listing || self.use_ftp {
            return;
        }
        if self.entries.is_empty() && num_logs > 0 {
            self.offset = u16::from(apm);
            self.entries = (0..num_logs).map(Entry::pending).collect();
            self.appended = true;
        }
        match num_logs > 0 {
            true if size > 0 || !apm => {
                let index = id.wrapping_sub(self.offset);
                if let Some(entry) = self.entries.iter_mut().find(|e| e.id == index) {
                    *entry = Entry { size, time_utc: Some(time_utc), received: true, status: "Available".to_string(), ..entry.clone() };
                }
            }
            true => {}
            false => self.finish_listing(),
        }
        self.retries = 0;
        match self.entries.iter().all(|e| e.received) {
            true => self.finish_listing(),
            false => self.due = Some(now_ms + TIMEOUT_MS),
        }
    }

    fn find_missing_entries(&mut self, now_ms: u64) -> Vec<Out> {
        let missing: Vec<usize> = self.entries.iter().enumerate().filter(|(_, e)| !e.received).map(|(i, _)| i).collect();
        let Some(start) = missing.first().copied() else {
            self.finish_listing();
            return Vec::new();
        };
        let end = missing.iter().enumerate().take_while(|(k, i)| **i == start + k).last().map_or(start, |(_, i)| *i);
        self.retries += 1;
        if self.retries > 3 {
            self.entries.iter_mut().filter(|e| !e.received).for_each(|e| e.status = "Error".to_string());
            self.finish_listing();
            return Vec::new();
        }
        let first = start as u16 + self.offset;
        self.request_list(first, end as u16 + self.offset, now_ms)
    }

    pub fn on_timeout(&mut self, now_ms: u64) -> Vec<Out> {
        if !self.due.is_some_and(|due| now_ms >= due) {
            return Vec::new();
        }
        self.due = None;
        match (self.listing, self.downloading) {
            (true, _) => self.find_missing_entries(now_ms),
            (false, true) => self.find_missing_data(now_ms),
            _ => Vec::new(),
        }
    }

    fn set_status(&mut self, id: u16, status: &str) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.status = status.to_string();
        }
    }

    pub fn select(&mut self, index: usize, on: bool) -> bool {
        self.entries.get_mut(index).map(|e| e.selected = on).is_some()
    }

    pub fn select_all(&mut self, on: bool) {
        self.entries.iter_mut().filter(|e| e.received).for_each(|e| e.selected = on);
    }

    pub fn set_sort_ascending(&mut self, ascending: bool) {
        if self.sort_ascending != ascending {
            self.sort_ascending = ascending;
            self.sort();
        }
    }

    fn reset_selection(&mut self, canceled: bool) {
        self.entries.iter_mut().filter(|e| e.selected).for_each(|e| {
            if canceled {
                e.status = "Canceled".to_string();
            }
            e.selected = false;
        });
    }

    pub fn download(&mut self, folder: &std::path::Path, extension: &str, now_ms: u64) -> Vec<Out> {
        if self.use_ftp {
            return self.ftp_download(folder, now_ms);
        }
        self.finish_listing();
        self.download = None;
        if folder.as_os_str().is_empty() {
            return Vec::new();
        }
        self.folder = folder.to_path_buf();
        self.extension = extension.to_string();
        if let Some(id) = self.entries.iter().find(|e| e.selected).map(|e| e.id) {
            self.set_status(id, "Waiting");
        }
        self.downloading = true;
        self.next_download(now_ms)
    }

    fn file_name(entry: &Entry, extension: &str) -> String {
        let when = entry.time_utc.and_then(local_time).filter(|t| chrono::Datelike::year(t) >= 2010).map_or_else(|| "UnknownDate".to_string(), |t| t.format("%Y-%-m-%-d-%H-%M-%S").to_string());
        format!("log_{}_{when}{extension}", entry.id)
    }

    fn open(&self, name: &str) -> Option<(std::fs::File, std::path::PathBuf)> {
        let (stem, extension) = name.rsplit_once('.').unwrap_or((name, ""));
        let path = (0..)
            .map(|n| match n {
                0 => self.folder.join(name),
                n => self.folder.join(format!("{stem}_{n}.{extension}")),
            })
            .find(|path| !path.exists())?;
        std::fs::File::create(&path).ok().map(|file| (file, path))
    }

    fn next_download(&mut self, now_ms: u64) -> Vec<Out> {
        self.due = None;
        self.download = None;
        let Some(entry) = self.entries.iter_mut().find(|e| e.selected) else {
            self.reset_selection(false);
            self.downloading = false;
            return Vec::new();
        };
        entry.selected = false;
        let entry = entry.clone();
        let opened = self.open(&Self::file_name(&entry, &self.extension)).and_then(|(file, path)| match file.set_len(u64::from(entry.size)) {
            Ok(()) => Some((file, path)),
            Err(_) => {
                let _ = std::fs::remove_file(&path);
                None
            }
        });
        let Some((file, path)) = opened else {
            self.set_status(entry.id, "Error");
            self.reset_selection(false);
            self.downloading = false;
            return Vec::new();
        };
        let download = Download { id: entry.id, size: entry.size, file, path, chunk: 0, table: vec![false; chunk_bins(entry.size, 0)], written: 0, rate_bytes: 0, rate_avg: 0.0, last_status_written: 0, elapsed_from_ms: now_ms };
        let count = download.table.len() as u32 * LOG_DATA_LEN;
        self.download = Some(download);
        self.due = Some(now_ms + TIMEOUT_MS);
        vec![Out::RequestData { id: entry.id + self.offset, offset: 0, count }]
    }

    fn update_rate(&mut self, now_ms: u64) {
        let Some(download) = self.download.as_mut() else { return };
        let elapsed = now_ms.saturating_sub(download.elapsed_from_ms);
        let time_met = elapsed >= GUI_RATE_MS;
        if !time_met && download.written.saturating_sub(download.last_status_written) < SIZE_UPDATE_BYTES {
            return;
        }
        if time_met {
            let rate = download.rate_bytes as f64 / (elapsed as f64 / 1000.0);
            download.rate_avg = download.rate_avg * 0.95 + rate * 0.05;
            download.rate_bytes = 0;
            download.elapsed_from_ms = now_ms;
        }
        let status = format!("{} ({}/s)", big_size_text(download.written), big_size_text(download.rate_avg as u64));
        download.last_status_written = download.written;
        let id = download.id;
        self.set_status(id, &status);
    }

    pub fn on_data(&mut self, offset: u32, id: u16, data: &[u8], now_ms: u64) -> Vec<Out> {
        if !self.downloading {
            return Vec::new();
        }
        let index = id.wrapping_sub(self.offset);
        let Some(download) = self.download.as_mut().filter(|d| d.id == index) else { return Vec::new() };
        if offset % LOG_DATA_LEN != 0 {
            return Vec::new();
        }
        if offset > download.size {
            self.set_status(index, "Error");
            return Vec::new();
        }
        let chunk = offset / CHUNK_SIZE;
        if chunk != download.chunk {
            return Vec::new();
        }
        let bin = ((offset - chunk * CHUNK_SIZE) / LOG_DATA_LEN) as usize;
        if let Some(slot) = download.table.get_mut(bin) {
            *slot = true;
        }
        if download.file.seek(SeekFrom::Start(u64::from(offset))).is_err() {
            return Vec::new();
        }
        if download.file.write_all(data).is_err() {
            self.set_status(index, "Error");
            return Vec::new();
        }
        download.written += data.len() as u64;
        download.rate_bytes += data.len() as u64;
        self.update_rate(now_ms);
        self.retries = 0;
        self.due = Some(now_ms + TIMEOUT_MS);
        let Some(download) = self.download.as_mut() else { return Vec::new() };
        match () {
            _ if download.log_complete() => {
                self.set_status(index, "Downloaded");
                self.next_download(now_ms)
            }
            _ if download.chunk_complete() => {
                download.advance();
                let request = Out::RequestData { id: index + self.offset, offset: download.chunk * CHUNK_SIZE, count: download.table.len() as u32 * LOG_DATA_LEN };
                vec![request]
            }
            _ if bin + 1 < download.table.len() && download.table[bin + 1] => self.find_missing_data(now_ms),
            _ => Vec::new(),
        }
    }

    fn find_missing_data(&mut self, now_ms: u64) -> Vec<Out> {
        let complete = self.download.as_ref().is_some_and(Download::log_complete);
        if complete {
            return self.next_download(now_ms);
        }
        let Some(download) = self.download.as_mut() else { return Vec::new() };
        if download.chunk_complete() {
            download.advance();
        }
        self.retries += 1;
        self.update_rate(now_ms);
        let Some(download) = self.download.as_ref() else { return Vec::new() };
        let start = download.table.iter().position(|bit| !*bit).unwrap_or(download.table.len());
        let end = download.table.iter().skip(start).position(|bit| *bit).map_or(download.table.len(), |run| start + run);
        let request = Out::RequestData { id: download.id + self.offset, offset: download.chunk * CHUNK_SIZE + start as u32 * LOG_DATA_LEN, count: (end - start) as u32 * LOG_DATA_LEN };
        self.due = Some(now_ms + TIMEOUT_MS);
        vec![request]
    }

    fn ftp_download(&mut self, folder: &std::path::Path, now_ms: u64) -> Vec<Out> {
        if folder.as_os_str().is_empty() {
            return Vec::new();
        }
        self.folder = folder.to_path_buf();
        self.ftp.had_error = false;
        self.ftp.downloads = self.entries.iter().filter(|e| e.selected && e.ftp_path.is_some()).map(|e| e.id).collect();
        let queued = self.ftp.downloads.clone();
        queued.iter().for_each(|id| self.set_status(*id, "Waiting"));
        if queued.is_empty() {
            return Vec::new();
        }
        self.downloading = true;
        self.next_ftp_download(now_ms)
    }

    fn local_name(&self, remote: &str, id: u16) -> std::path::PathBuf {
        let name = remote.rsplit('/').next().filter(|n| !n.is_empty()).map_or_else(|| format!("log_{id}.ulg"), str::to_string);
        let (stem, extension) = name.rsplit_once('.').map_or((name.as_str(), String::new()), |(s, e)| (s, format!(".{e}")));
        (0..)
            .map(|n| match n {
                0 => self.folder.join(&name),
                n => self.folder.join(format!("{stem}_{n}{extension}")),
            })
            .find(|path| !path.exists())
            .unwrap_or_else(|| self.folder.join(&name))
    }

    fn next_ftp_download(&mut self, now_ms: u64) -> Vec<Out> {
        self.ftp.current = None;
        if self.ftp.downloads.is_empty() {
            if self.ftp.had_error {
                self.ftp_disabled = true;
            }
            self.downloading = false;
            return Vec::new();
        }
        let id = self.ftp.downloads.remove(0);
        let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else { return self.next_ftp_download(now_ms) };
        entry.selected = false;
        entry.status = "Downloading".to_string();
        let path = entry.ftp_path.clone().unwrap_or_default();
        let local = self.local_name(&path, id);
        self.ftp.current = Some(id);
        (self.ftp.progress_bytes, self.ftp.progress_from_ms, self.ftp.rate_avg) = (0, now_ms, 0.0);
        vec![Out::FtpDownload { path, local }]
    }

    pub fn on_ftp_downloaded(&mut self, result: Result<(), String>, now_ms: u64) -> Vec<Out> {
        let Some(id) = self.ftp.current.filter(|_| self.downloading && !self.ftp.deleting) else { return Vec::new() };
        match result {
            Ok(()) => self.set_status(id, "Downloaded"),
            Err(_) => {
                self.set_status(id, "Error");
                self.ftp.had_error = true;
            }
        }
        self.next_ftp_download(now_ms)
    }

    pub fn on_ftp_progress(&mut self, fraction: f64, now_ms: u64) {
        let Some(id) = self.ftp.current.filter(|_| !self.ftp.deleting) else { return };
        let elapsed = now_ms.saturating_sub(self.ftp.progress_from_ms);
        if elapsed < GUI_RATE_MS {
            return;
        }
        let size = self.entries.iter().find(|e| e.id == id).map_or(0, |e| e.size);
        let total = (f64::from(size) * fraction) as u64;
        if total < self.ftp.progress_bytes {
            self.ftp.progress_bytes = total;
            return;
        }
        let rate = (total - self.ftp.progress_bytes) as f64 / (elapsed as f64 / 1000.0);
        self.ftp.rate_avg = self.ftp.rate_avg * 0.95 + rate * 0.05;
        (self.ftp.progress_bytes, self.ftp.progress_from_ms) = (total, now_ms);
        let status = format!("{} ({}/s)", big_size_text(total), big_size_text(self.ftp.rate_avg as u64));
        self.set_status(id, &status);
    }

    pub fn erase_selected(&mut self) -> Vec<Out> {
        if !self.use_ftp || self.busy() {
            return Vec::new();
        }
        self.ftp.deletes = self.entries.iter().filter(|e| e.selected && e.ftp_path.is_some()).map(|e| e.id).collect();
        if self.ftp.deletes.is_empty() {
            return Vec::new();
        }
        self.ftp.deleting = true;
        self.downloading = true;
        self.next_ftp_delete(0)
    }

    fn next_ftp_delete(&mut self, now_ms: u64) -> Vec<Out> {
        self.ftp.erasing = None;
        if self.ftp.deletes.is_empty() {
            self.ftp.deleting = false;
            self.downloading = false;
            return self.refresh(now_ms);
        }
        let id = self.ftp.deletes.remove(0);
        let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else { return self.next_ftp_delete(now_ms) };
        entry.selected = false;
        entry.status = "Erasing".to_string();
        let path = entry.ftp_path.clone().unwrap_or_default();
        self.ftp.erasing = Some(id);
        vec![Out::FtpDelete(path)]
    }

    pub fn on_ftp_delete_refused(&mut self, now_ms: u64) -> Vec<Out> {
        if !self.ftp.deleting {
            return Vec::new();
        }
        if let Some(id) = self.ftp.erasing {
            self.set_status(id, "Error");
        }
        self.next_ftp_delete(now_ms)
    }

    pub fn on_ftp_deleted(&mut self, now_ms: u64) -> Vec<Out> {
        match self.ftp.deleting {
            true => self.next_ftp_delete(now_ms),
            false => Vec::new(),
        }
    }

    pub fn cancel(&mut self) -> Vec<Out> {
        if self.use_ftp {
            return self.cancel_ftp();
        }
        self.finish_listing();
        if let Some(download) = self.download.take() {
            self.set_status(download.id, "Canceled");
            drop(download.file);
            let _ = std::fs::remove_file(&download.path);
        }
        self.reset_selection(true);
        self.downloading = false;
        self.due = None;
        vec![Out::RequestEnd]
    }

    fn cancel_ftp(&mut self) -> Vec<Out> {
        let listing = self.listing;
        if listing {
            self.ftp.dirs.clear();
            self.finish_listing();
        }
        let stop_listing = listing.then_some(Out::FtpCancel);
        if self.ftp.deleting {
            self.ftp.deletes.clear();
            self.reset_selection(true);
            return stop_listing.into_iter().collect();
        }
        let downloading = self.downloading;
        if downloading {
            if let Some(id) = self.ftp.current.take() {
                self.set_status(id, "Canceled");
            }
            self.ftp.downloads.clear();
        }
        self.reset_selection(true);
        self.downloading = false;
        stop_listing.into_iter().chain(downloading.then_some(Out::FtpCancel)).collect()
    }

    pub fn leave(&mut self) {
        if let Some(download) = self.download.take() {
            drop(download.file);
            let _ = std::fs::remove_file(&download.path);
        }
        *self = OnboardLogs { sort_ascending: self.sort_ascending, ..OnboardLogs::default() };
    }

    pub fn erase_all(&mut self, now_ms: u64) -> Vec<Out> {
        std::iter::once(Out::Erase).chain(self.refresh(now_ms)).collect()
    }

    fn entry_json(entry: &Entry) -> Value {
        json!({
            "children": [],
            "class": "QGCOnboardLogEntry",
            "facts": [],
            "id": entry.id,
            "kind": "object",
            "objectName": "",
            "received": entry.received,
            "selected": entry.selected,
            "size": entry.size,
            "sizeStr": big_size_text(u64::from(entry.size)),
            "status": entry.status,
            "statusId": entry.status_id(),
            "time": entry.time_utc.and_then(local_time).map(|t| t.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()),
        })
    }

    pub fn model_json(&self) -> Value {
        json!({
            "children": [],
            "class": "QmlObjectListModel",
            "count": self.entries.len(),
            "dirty": self.appended,
            "elements": self.entries.iter().map(Self::entry_json).collect::<Vec<_>>(),
            "facts": [],
            "kind": "object",
            "objectName": "",
        })
    }

    pub fn controller_json(&self) -> Value {
        json!({
            "allLogsSelected": self.all_selected(),
            "children": ["model"],
            "class": "OnboardLogController",
            "compressLogs": false,
            "compressing": false,
            "compressionProgress": 0,
            "downloadingLogs": self.downloading,
            "facts": [],
            "kind": "object",
            "objectName": "",
            "requestingList": self.listing,
            "selectedCount": self.selected_count(),
            "sortAscending": self.sort_ascending,
            "transport": self.transport(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_fills_by_index_with_ardupilots_offset_and_sorts_newest_first() {
        let mut logs = OnboardLogs::default();
        assert_eq!(logs.refresh(0), vec![Out::RequestList { start: 0, end: 0xffff }]);
        logs.on_entry(true, 100, 50, 1, 3, 10);
        logs.on_entry(true, 300, 70, 3, 3, 20);
        assert!(logs.listing, "log 2 has not answered");
        assert_eq!(logs.on_timeout(519), vec![]);
        assert_eq!(logs.on_timeout(520), vec![Out::RequestList { start: 2, end: 2 }], "the missing index goes back out with the offset added");
        logs.on_entry(true, 200, 60, 2, 3, 600);
        assert!(!logs.listing);
        assert_eq!(logs.entries.iter().map(|e| e.id).collect::<Vec<_>>(), vec![2, 1, 0], "entries sort newest first by time");
        assert_eq!(logs.entries[0].status_id(), "available");
    }

    #[test]
    fn a_log_that_never_answers_is_an_error_after_the_retries() {
        let mut logs = OnboardLogs::default();
        logs.refresh(0);
        logs.on_entry(false, 100, 50, 0, 2, 0);
        let asked = (1..=4).map(|n| logs.on_timeout(n * 10_000).len()).collect::<Vec<_>>();
        assert_eq!(asked, vec![1, 1, 1, 0], "three retries, then give up");
        assert_eq!((logs.listing, logs.entries.iter().find(|e| e.id == 1).unwrap().status.as_str()), (false, "Error"));
    }

    #[test]
    fn a_download_writes_the_log_and_moves_to_the_next_selection() {
        let folder = std::env::temp_dir().join(format!("qgc-onboard-logs-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut logs = OnboardLogs::default();
        logs.refresh(0);
        logs.on_entry(false, 0, 180, 0, 1, 0);
        logs.select(0, true);
        assert_eq!(logs.download(&folder, ".bin", 0), vec![Out::RequestData { id: 0, offset: 0, count: 180 }]);
        assert!(logs.on_data(90, 0, &[2; 90], 10).is_empty());
        assert!(logs.on_data(0, 0, &[1; 90], 20).is_empty(), "the last missing packet completes the log and nothing else is selected");
        assert_eq!((logs.downloading, logs.entries[0].status.as_str()), (false, "Downloaded"));
        let written = std::fs::read(folder.join("log_0_UnknownDate.bin")).unwrap();
        assert_eq!((written.len(), written[0], written[90]), (180, 1, 2));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    fn ftp_logs() -> OnboardLogs {
        let mut logs = OnboardLogs { ftp_capable: true, ..OnboardLogs::default() };
        logs.set_ftp_fallback_root(Some(PX4_LOG_ROOT));
        logs
    }

    #[test]
    fn ftp_entries_take_the_vehicles_time_or_the_date_folder_and_skip_other_files() {
        assert_eq!(ftp_entry("Fa.ulg\t100\t1700000000", None), Some(("a.ulg".to_string(), 100, Some(1_700_000_000))));
        assert_eq!(ftp_entry("F12_30_05.ulg\t7", Some("2026-09-30")).map(|e| e.2), Some(Some(1_790_771_405)));
        assert_eq!(ftp_entry("F00000001.BIN\t9\t0", None), Some(("00000001.BIN".to_string(), 9, None)), "a zero time is unknown");
        assert_eq!(ftp_entry("Fnotes.txt\t5", None), None);
        assert_eq!(ftp_entry("Dsess001", None), None);
    }

    #[test]
    fn an_ftp_listing_walks_the_date_folders_and_falls_back_like_the_controller() {
        let mut logs = ftp_logs();
        assert_eq!(logs.refresh(0), vec![Out::FtpList(MAVLINK_LOG_ROOT.to_string())]);
        assert_eq!(logs.on_ftp_listed(Err("List directory failed".into()), false, 0), vec![Out::FtpList(PX4_LOG_ROOT.to_string())], "a root @MAV_LOG cannot list falls back to the firmware's own folder");
        let root = vec!["D2026-09-30".to_string(), "D2026-09-29".to_string(), "Fstray.ulg\t3".to_string()];
        assert_eq!(logs.on_ftp_listed(Ok(root), false, 0), vec![Out::FtpList(format!("{PX4_LOG_ROOT}/2026-09-29"))], "folders are walked in sorted order");
        assert_eq!(logs.on_ftp_listed(Ok(vec!["F10_00_00.ulg\t50".into()]), false, 0), vec![Out::FtpList(format!("{PX4_LOG_ROOT}/2026-09-30"))]);
        assert!(logs.on_ftp_listed(Ok(vec!["F11_00_00.ulg\t60".into()]), false, 0).is_empty());
        assert!(!logs.listing);
        assert_eq!(logs.transport(), FTP_TRANSPORT);
        assert_eq!(logs.entries.len(), 3);
        assert_eq!(logs.entries[0].ftp_path.as_deref(), Some("/fs/microsd/log/2026-09-30/11_00_00.ulg"), "newest first");

        let mut old_px4 = ftp_logs();
        old_px4.refresh(0);
        assert_eq!(old_px4.on_ftp_listed(Ok(vec!["Fa.ulg\t1".into()]), true, 0), vec![Out::RequestList { start: 0, end: 0xffff }], "no file times over FTP means the message transport, which has them");
        assert_eq!((old_px4.transport(), old_px4.entries.len()), ("messages", 0));
        assert_eq!(old_px4.refresh(10_000).first(), None, "listing is still busy on the messages request");
    }

    #[test]
    fn ftp_downloads_and_erases_walk_the_selection() {
        let folder = std::env::temp_dir().join(format!("qgc-ftp-logs-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.ulg"), b"x").unwrap();
        let mut logs = ftp_logs();
        logs.refresh(0);
        logs.on_ftp_listed(Ok(vec!["Fa.ulg\t10".into(), "Fb.ulg\t20".into()]), false, 0);
        logs.select_all(true);
        let first = logs.download(&folder, ".ulg", 0);
        assert!(matches!(first.as_slice(), [Out::FtpDownload { path, local }] if path.ends_with("/a.ulg") || path.ends_with("/b.ulg") ) );
        let Out::FtpDownload { local, .. } = &first[0] else { unreachable!() };
        assert!(local.ends_with("a_1.ulg") || local.ends_with("b.ulg"), "an existing file gets a numbered name");
        assert!(matches!(logs.on_ftp_downloaded(Ok(()), 1).as_slice(), [Out::FtpDownload { .. }]));
        assert!(logs.on_ftp_downloaded(Err("Download failed".into()), 2).is_empty());
        assert!(!logs.downloading);
        assert_eq!(logs.entries.iter().filter(|e| e.status == "Downloaded").count(), 1);

        logs.select(0, true);
        assert!(matches!(logs.erase_selected().as_slice(), [Out::FtpDelete(_)]));
        assert_eq!(logs.on_ftp_deleted(3), vec![Out::RequestList { start: 0, end: 0xffff }], "the failed download disabled FTP, so the refresh after erasing goes over messages");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_refused_ftp_listing_falls_back_or_skips_the_folder_like_the_controller() {
        let mut root = ftp_logs();
        root.refresh(0);
        assert_eq!(root.on_ftp_list_refused(false, 0), vec![Out::RequestList { start: 0, end: 0xffff }], "a root listing FTPManager will not start goes straight to messages, no fallback root");

        let mut walk = ftp_logs();
        walk.refresh(0);
        walk.on_ftp_listed(Ok(vec!["D2026-09-29".into(), "D2026-09-30".into()]), false, 0);
        assert_eq!(walk.on_ftp_list_refused(false, 0), vec![Out::FtpList(format!("{MAVLINK_LOG_ROOT}/2026-09-30"))], "a folder that cannot be listed is skipped");
        assert!(walk.on_ftp_list_refused(false, 0).is_empty());
        assert_eq!((walk.listing, walk.transport()), (false, FTP_TRANSPORT));
    }

    #[test]
    fn ftp_erase_marks_a_refused_delete_and_cancel_drops_the_queue_like_the_controller() {
        let mut logs = ftp_logs();
        logs.refresh(0);
        logs.on_ftp_listed(Ok(vec!["Fa.ulg\t10".into(), "Fb.ulg\t20".into(), "Fc.ulg\t30".into()]), false, 0);
        logs.select_all(true);
        let Out::FtpDelete(first) = logs.erase_selected().remove(0) else { unreachable!() };
        let erasing = logs.entries.iter().position(|e| e.ftp_path.as_deref() == Some(first.as_str())).unwrap();
        assert!(matches!(logs.on_ftp_delete_refused(1).as_slice(), [Out::FtpDelete(_)]));
        assert_eq!(logs.entries[erasing].status, "Error");
        assert!(logs.cancel().is_empty(), "the delete in flight cannot be aborted");
        assert_eq!(logs.entries.iter().filter(|e| e.status == "Canceled" && !e.selected).count(), 1, "the still queued log is canceled and deselected");
        assert!(logs.downloading, "the cycle ends when the delete in flight completes");
        assert_eq!(logs.on_ftp_deleted(2), vec![Out::FtpList(MAVLINK_LOG_ROOT.to_string())]);
    }

    #[test]
    fn leaving_the_vehicle_forgets_the_logs_and_the_partial_file_without_ending_the_request() {
        let folder = std::env::temp_dir().join(format!("qgc-onboard-leave-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut logs = ftp_logs();
        logs.ftp_disabled = true;
        logs.refresh(0);
        logs.on_entry(false, 0, 180, 0, 1, 0);
        logs.select(0, true);
        logs.download(&folder, ".bin", 0);
        assert!(folder.join("log_0_UnknownDate.bin").exists());
        logs.leave();
        assert!(!folder.join("log_0_UnknownDate.bin").exists());
        assert_eq!((logs.busy(), logs.entries.len(), logs.ftp_disabled), (false, 0, false), "the next vehicle starts with an empty model and FTP allowed again");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn sizes_read_as_qt_spells_them() {
        assert_eq!(big_size_text(900), "900B");
        assert_eq!(big_size_text(5_250_964), "5.0MB");
    }
}
