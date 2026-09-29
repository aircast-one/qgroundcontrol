use std::io::{Seek, SeekFrom, Write};

use serde_json::{Value, json};

pub const LOG_DATA_LEN: u32 = 90;
const TABLE_BINS: u32 = 2048;
const CHUNK_SIZE: u32 = TABLE_BINS * LOG_DATA_LEN;
const LIST_TIMEOUT_MS: u64 = 5000;
const TIMEOUT_MS: u64 = 500;
const GUI_RATE_MS: u64 = 500;
const SIZE_UPDATE_BYTES: u64 = 102_400;

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    RequestList { start: u16, end: u16 },
    RequestData { id: u16, offset: u32, count: u32 },
    RequestEnd,
    Erase,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: u16,
    pub size: u32,
    pub time_utc: Option<u32>,
    pub received: bool,
    pub selected: bool,
    pub status: String,
}

impl Entry {
    fn pending(id: u16) -> Entry {
        Entry { id, size: 0, time_utc: None, received: false, selected: false, status: "Pending".to_string() }
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
        self.request_list(0, 0xffff, now_ms)
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
        if !self.listing {
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
        let wrote = download.file.seek(SeekFrom::Start(u64::from(offset))).and_then(|_| download.file.write_all(data)).is_ok();
        if !wrote {
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

    pub fn cancel(&mut self) -> Vec<Out> {
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
            "transport": "messages",
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

    #[test]
    fn sizes_read_as_qt_spells_them() {
        assert_eq!(big_size_text(900), "900B");
        assert_eq!(big_size_text(5_250_964), "5.0MB");
    }
}
