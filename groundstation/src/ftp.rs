pub const HEADER_LEN: usize = 12;
pub const PAYLOAD_LEN: usize = 251;
pub const DATA_LEN: usize = PAYLOAD_LEN - HEADER_LEN;
pub const MAX_RETRY: u32 = 3;
pub const SCHEME: &str = "mftp";
pub const COMP_ID_ALL: u8 = 0;
pub const COMP_ID_AUTOPILOT1: u8 = 1;

pub const CMD_TERMINATE_SESSION: u8 = 1;
pub const CMD_RESET_SESSIONS: u8 = 2;
pub const CMD_LIST_DIRECTORY: u8 = 3;
pub const CMD_OPEN_FILE_RO: u8 = 4;
pub const CMD_READ_FILE: u8 = 5;
pub const CMD_CREATE_FILE: u8 = 6;
pub const CMD_WRITE_FILE: u8 = 7;
pub const CMD_REMOVE_FILE: u8 = 8;
pub const CMD_BURST_READ_FILE: u8 = 15;
pub const CMD_LIST_DIRECTORY_WITH_TIME: u8 = 16;
pub const RSP_ACK: u8 = 128;
pub const RSP_NAK: u8 = 129;
pub const ERR_FAIL_ERRNO: u8 = 2;
pub const ERR_EOF: u8 = 6;
pub const ERR_UNKNOWN_COMMAND: u8 = 7;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Request {
    pub seq: u16,
    pub session: u8,
    pub opcode: u8,
    pub req_opcode: u8,
    pub burst_complete: bool,
    pub offset: u32,
    pub data: Vec<u8>,
}

impl Request {
    pub fn encode(&self) -> [u8; PAYLOAD_LEN] {
        let mut payload = [0u8; PAYLOAD_LEN];
        payload[0..2].copy_from_slice(&self.seq.to_le_bytes());
        payload[2] = self.session;
        payload[3] = self.opcode;
        payload[4] = self.data.len().min(DATA_LEN) as u8;
        payload[5] = self.req_opcode;
        payload[6] = self.burst_complete as u8;
        payload[8..12].copy_from_slice(&self.offset.to_le_bytes());
        let n = self.data.len().min(DATA_LEN);
        payload[HEADER_LEN..HEADER_LEN + n].copy_from_slice(&self.data[..n]);
        payload
    }

    pub fn decode(payload: &[u8]) -> Option<Request> {
        if payload.len() < HEADER_LEN {
            return None;
        }
        let size = payload[4] as usize;
        let data = payload.get(HEADER_LEN..HEADER_LEN + size)?.to_vec();
        Some(Request { seq: u16::from_le_bytes([payload[0], payload[1]]), session: payload[2], opcode: payload[3], req_opcode: payload[5], burst_complete: payload[6] != 0, offset: u32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]), data })
    }

    pub fn open_length(&self) -> Option<u32> {
        (self.data.len() == 4).then(|| u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]]))
    }

    pub fn nak_error(&self) -> String {
        let code = self.data.first().copied().unwrap_or(0);
        match (code, self.data.len()) {
            (ERR_FAIL_ERRNO, 2) => format!("errno {}", self.data[1]),
            (ERR_FAIL_ERRNO, _) | (_, 0) => "Invalid Nak format".to_string(),
            (_, 1) => error_text(code).to_string(),
            _ => "Invalid Nak format".to_string(),
        }
    }
}

pub fn error_text(code: u8) -> &'static str {
    match code {
        0 => "None",
        1 => "Fail",
        2 => "FailErrno",
        3 => "InvalidDataSize",
        4 => "InvalidSession",
        5 => "NoSessionsAvailable",
        6 => "EOF",
        7 => "UnknownCommand",
        8 => "FailFileExists",
        9 => "FailFileProtected",
        10 => "FailFileNotFound",
        _ => "Unknown",
    }
}

pub fn parse_uri(from_component: u8, uri: &str) -> Result<(String, u8), String> {
    let prefix = format!("{SCHEME}://");
    let has_scheme = uri.as_bytes().len() >= prefix.len() && uri.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes());
    let mut path = if has_scheme { format!("/{}", &uri[prefix.len()..]) } else { uri.to_string() };
    if path.contains("://") {
        return Err(format!("Incorrect uri scheme or format {uri}"));
    }
    let mut component = if from_component == COMP_ID_ALL { COMP_ID_AUTOPILOT1 } else { from_component };
    let trimmed = path.trim_start_matches('/');
    if let Some(rest) = trimmed.strip_prefix("[;comp=") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !rest[digits.len()..].starts_with(']') {
            return Err(format!("Incorrect format for component id {uri}"));
        }
        component = digits.parse().map_err(|_| format!("Incorrect format for component id {uri}"))?;
        let tag = format!("[;comp={digits}]");
        path = path.replacen(&tag, "", 1);
    }
    Ok((path, component))
}

#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    Send(Request),
    StartTimer,
    StopTimer,
    Progress(f64),
    Complete { ok: bool, error: String, bytes: Vec<u8> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Open,
    Burst,
    Fill,
    Reset,
    Terminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Missing {
    offset: u32,
    bytes: u32,
}

#[derive(Debug, Default)]
pub struct Download {
    pub path: String,
    pub component: u8,
    pub check_size: bool,
    expected_seq: u16,
    phase: Option<Phase>,
    session: u8,
    expected_offset: u32,
    written: u32,
    file_size: u32,
    file: Vec<u8>,
    sink_path: Option<std::path::PathBuf>,
    sink: Option<std::fs::File>,
    missing: Vec<Missing>,
    retries: u32,
}

impl Download {
    pub fn stream_to(&mut self, local: &std::path::Path) -> Result<(), String> {
        self.sink_path = Some(local.to_path_buf());
        Ok(())
    }

    pub fn start(from_component: u8, uri: &str, check_size: bool) -> Result<(Download, Vec<Out>), String> {
        Self::start_from(from_component, uri, check_size, 0)
    }

    pub fn start_from(from_component: u8, uri: &str, check_size: bool, expected_seq: u16) -> Result<(Download, Vec<Out>), String> {
        let (path, component) = parse_uri(from_component, uri)?;
        let mut download = Download { path, component, check_size, phase: Some(Phase::Open), expected_seq, ..Default::default() };
        let mut request = Request { session: 0, opcode: CMD_OPEN_FILE_RO, data: download.path.as_bytes().iter().copied().take(DATA_LEN).collect(), ..Default::default() };
        let out = download.send(&mut request);
        Ok((download, out))
    }

    pub fn in_progress(&self) -> bool {
        self.phase.is_some_and(|p| p != Phase::Idle)
    }

    pub fn expected_seq(&self) -> u16 {
        self.expected_seq
    }

    fn send(&mut self, request: &mut Request) -> Vec<Out> {
        request.seq = self.expected_seq.wrapping_add(1);
        self.expected_seq = self.expected_seq.wrapping_add(2);
        vec![Out::StartTimer, Out::Send(request.clone())]
    }

    fn fail(&mut self, error: &str) -> Vec<Out> {
        self.phase = None;
        if let (Some(file), Some(path)) = (self.sink.take(), self.sink_path.as_ref()) {
            drop(file);
            let _ = std::fs::remove_file(path);
        }
        vec![Out::StopTimer, Out::Complete { ok: false, error: error.to_string(), bytes: Vec::new() }]
    }

    fn burst(&mut self, first: bool) -> Vec<Out> {
        if first {
            self.retries = 0;
        } else {
            self.expected_seq = self.expected_seq.wrapping_sub(2);
        }
        let mut request = Request { session: self.session, opcode: CMD_BURST_READ_FILE, offset: self.expected_offset, data: vec![0; DATA_LEN], ..Default::default() };
        self.send(&mut request)
    }

    fn fill(&mut self, first: bool) -> Vec<Out> {
        let Some(missing) = self.missing.first().cloned() else {
            return if !self.check_size || self.written == self.file_size { self.reset() } else { self.fail("Download failed") };
        };
        if first {
            self.retries = 0;
        } else {
            self.expected_seq = self.expected_seq.wrapping_sub(2);
        }
        let count = missing.bytes.min(DATA_LEN as u32);
        self.expected_offset = missing.offset;
        let mut request = Request { session: self.session, opcode: CMD_READ_FILE, offset: missing.offset, data: vec![0; count as usize], ..Default::default() };
        self.send(&mut request)
    }

    fn reset(&mut self) -> Vec<Out> {
        self.phase = Some(Phase::Reset);
        let mut request = Request { opcode: CMD_RESET_SESSIONS, ..Default::default() };
        self.send(&mut request)
    }

    fn within_file(&self, offset: u32, len: usize) -> bool {
        (offset as u64 + len as u64) <= self.file_size as u64
    }

    fn write_at(&mut self, offset: u32, data: &[u8]) -> Result<(), String> {
        match self.sink.as_mut() {
            Some(file) => {
                use std::io::{Seek, SeekFrom, Write};
                file.seek(SeekFrom::Start(u64::from(offset))).and_then(|_| file.write_all(data)).map_err(|e| format!("Download failed for: {} - {e}", self.path))?;
            }
            None => {
                let end = offset as usize + data.len();
                if self.file.len() < end {
                    self.file.resize(end, 0);
                }
                self.file[offset as usize..end].copy_from_slice(data);
            }
        }
        self.written += data.len() as u32;
        Ok(())
    }

    fn progress(&self) -> Option<Out> {
        (self.file_size != 0).then(|| Out::Progress(self.written as f64 / self.file_size as f64))
    }

    pub fn cancel(&mut self) -> Vec<Out> {
        if !self.in_progress() {
            return Vec::new();
        }
        self.phase = Some(Phase::Terminate);
        self.retries = 0;
        let mut request = Request { session: self.session, opcode: CMD_TERMINATE_SESSION, ..Default::default() };
        let mut out = vec![Out::StopTimer];
        out.append(&mut self.send(&mut request));
        out
    }

    pub fn on_timeout(&mut self) -> Vec<Out> {
        match self.phase {
            Some(Phase::Open) => self.fail("Download failed"),
            Some(Phase::Burst) => {
                self.retries += 1;
                if self.retries > MAX_RETRY { self.fail("Download failed") } else { self.burst(false) }
            }
            Some(Phase::Fill) => {
                self.retries += 1;
                if self.retries > MAX_RETRY { self.fail("Download failed") } else { self.fill(false) }
            }
            Some(Phase::Reset) => self.complete_ok(),
            Some(Phase::Terminate) => {
                self.retries += 1;
                if self.retries > MAX_RETRY {
                    self.fail("Download cancelled")
                } else {
                    self.expected_seq = self.expected_seq.wrapping_sub(2);
                    let mut request = Request { session: self.session, opcode: CMD_TERMINATE_SESSION, ..Default::default() };
                    self.send(&mut request)
                }
            }
            _ => Vec::new(),
        }
    }

    fn complete_ok(&mut self) -> Vec<Out> {
        self.phase = None;
        vec![Out::StopTimer, Out::Complete { ok: true, error: String::new(), bytes: std::mem::take(&mut self.file) }]
    }

    pub fn on_payload(&mut self, payload: &[u8]) -> Vec<Out> {
        let Some(reply) = Request::decode(payload) else { return Vec::new() };
        if self.expected_seq.wrapping_sub(1).wrapping_sub(reply.seq) < u16::MAX / 2 {
            return Vec::new();
        }
        match self.phase {
            Some(Phase::Open) => self.on_open(&reply),
            Some(Phase::Burst) => self.on_burst(&reply),
            Some(Phase::Fill) => self.on_fill(&reply),
            Some(Phase::Reset) => self.on_reset(&reply),
            Some(Phase::Terminate) => self.on_terminate(&reply),
            _ => Vec::new(),
        }
    }

    fn on_open(&mut self, reply: &Request) -> Vec<Out> {
        if reply.req_opcode != CMD_OPEN_FILE_RO || reply.seq != self.expected_seq {
            return Vec::new();
        }
        match reply.opcode {
            RSP_ACK => match reply.open_length() {
                Some(size) => {
                    if let Some(path) = self.sink_path.clone() {
                        match std::fs::File::create(&path) {
                            Ok(file) => self.sink = Some(file),
                            Err(e) => return self.fail(&format!("Download failed for: {} - {e}", self.path)),
                        }
                    }
                    self.session = reply.session;
                    self.file_size = size;
                    self.expected_offset = 0;
                    self.phase = Some(Phase::Burst);
                    let mut out = vec![Out::StopTimer];
                    out.append(&mut self.burst(true));
                    out
                }
                None => self.fail("Download failed"),
            },
            RSP_NAK => {
                let reason = format!("Download failed: {}", reply.nak_error());
                self.fail(&reason)
            }
            _ => Vec::new(),
        }
    }

    fn on_burst(&mut self, reply: &Request) -> Vec<Out> {
        if reply.req_opcode != CMD_BURST_READ_FILE || reply.session != self.session {
            return Vec::new();
        }
        match reply.opcode {
            RSP_ACK => {
                if reply.seq < self.expected_seq {
                    return vec![Out::StopTimer];
                }
                if !self.within_file(reply.offset, reply.data.len()) {
                    return self.fail("Download failed");
                }
                if reply.offset != self.expected_offset {
                    if reply.offset > self.expected_offset {
                        self.missing.push(Missing { offset: self.expected_offset, bytes: reply.offset - self.expected_offset });
                    } else {
                        return vec![Out::StartTimer];
                    }
                }
                if let Err(error) = self.write_at(reply.offset, &reply.data) {
                    return self.fail(&error);
                }
                self.expected_offset = reply.offset + reply.data.len() as u32;
                let mut out = vec![Out::StopTimer];
                if reply.burst_complete {
                    self.expected_seq = reply.seq;
                    out.append(&mut self.burst(true));
                } else {
                    self.expected_seq = reply.seq.wrapping_add(1);
                    out.push(Out::StartTimer);
                }
                out.extend(self.progress());
                out
            }
            RSP_NAK => {
                if reply.data.first() == Some(&ERR_EOF) {
                    if reply.seq != self.expected_seq {
                        self.expected_seq = reply.seq;
                        let mut out = vec![Out::StopTimer];
                        out.append(&mut self.burst(true));
                        out
                    } else {
                        self.phase = Some(Phase::Fill);
                        let mut out = vec![Out::StopTimer];
                        out.append(&mut self.fill(true));
                        out
                    }
                } else {
                    self.fail("Download failed")
                }
            }
            _ => Vec::new(),
        }
    }

    fn on_fill(&mut self, reply: &Request) -> Vec<Out> {
        if reply.req_opcode != CMD_READ_FILE || reply.seq != self.expected_seq || reply.session != self.session {
            return Vec::new();
        }
        match reply.opcode {
            RSP_ACK => {
                if reply.offset != self.expected_offset {
                    self.retries += 1;
                    return if self.retries > MAX_RETRY { self.fail("Download failed") } else { self.fill(false) };
                }
                if !self.within_file(reply.offset, reply.data.len()) {
                    return self.fail("Download failed");
                }
                if let Err(error) = self.write_at(reply.offset, &reply.data) {
                    return self.fail(&error);
                }
                let done = {
                    let missing = &mut self.missing[0];
                    missing.offset += reply.data.len() as u32;
                    missing.bytes = missing.bytes.saturating_sub(reply.data.len() as u32);
                    missing.bytes == 0
                };
                if done {
                    self.missing.remove(0);
                }
                let mut out = vec![Out::StopTimer];
                out.append(&mut self.fill(true));
                out.extend(self.progress());
                out
            }
            RSP_NAK if reply.data.first() == Some(&ERR_EOF) && (!self.check_size || self.written == self.file_size) => {
                let mut out = vec![Out::StopTimer];
                out.append(&mut self.reset());
                out
            }
            RSP_NAK => self.fail("Download failed"),
            _ => Vec::new(),
        }
    }

    fn on_reset(&mut self, reply: &Request) -> Vec<Out> {
        if reply.req_opcode != CMD_RESET_SESSIONS || reply.seq != self.expected_seq {
            return Vec::new();
        }
        match reply.opcode {
            RSP_ACK | RSP_NAK => self.complete_ok(),
            _ => Vec::new(),
        }
    }

    fn on_terminate(&mut self, reply: &Request) -> Vec<Out> {
        if reply.req_opcode != CMD_TERMINATE_SESSION || reply.seq != self.expected_seq {
            return Vec::new();
        }
        self.fail("Download cancelled")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListOut {
    Send(Request),
    StartTimer,
    StopTimer,
    Complete { entries: Vec<String>, error: String },
}

#[derive(Debug, Default)]
pub struct Listing {
    pub path: String,
    pub component: u8,
    expected_seq: u16,
    expected_offset: u32,
    entries: Vec<String>,
    retries: u32,
    active: bool,
    opcode: u8,
    pub time_unsupported: bool,
}

impl Listing {
    pub fn start(from_component: u8, uri: &str) -> Result<(Listing, Vec<ListOut>), String> {
        Self::start_from(from_component, uri, 0)
    }

    pub fn start_from(from_component: u8, uri: &str, expected_seq: u16) -> Result<(Listing, Vec<ListOut>), String> {
        Self::start_listing(from_component, uri, expected_seq, false)
    }

    pub fn start_with_time(from_component: u8, uri: &str, expected_seq: u16) -> Result<(Listing, Vec<ListOut>), String> {
        Self::start_listing(from_component, uri, expected_seq, true)
    }

    fn start_listing(from_component: u8, uri: &str, expected_seq: u16, with_time: bool) -> Result<(Listing, Vec<ListOut>), String> {
        let (path, component) = parse_uri(from_component, uri)?;
        let opcode = if with_time { CMD_LIST_DIRECTORY_WITH_TIME } else { CMD_LIST_DIRECTORY };
        let mut listing = Listing { path, component, active: true, expected_seq, opcode, ..Default::default() };
        let out = listing.request(true);
        Ok((listing, out))
    }

    pub fn in_progress(&self) -> bool {
        self.active
    }

    pub fn expected_seq(&self) -> u16 {
        self.expected_seq
    }

    fn request(&mut self, first: bool) -> Vec<ListOut> {
        if first {
            self.retries = 0;
        } else {
            self.expected_seq = self.expected_seq.wrapping_sub(2);
        }
        let request = Request { seq: self.expected_seq.wrapping_add(1), session: 0, opcode: self.opcode, offset: self.expected_offset, data: self.path.as_bytes().iter().copied().take(DATA_LEN).collect(), ..Default::default() };
        self.expected_seq = self.expected_seq.wrapping_add(2);
        vec![ListOut::StartTimer, ListOut::Send(request)]
    }

    fn complete(&mut self, error: &str) -> Vec<ListOut> {
        self.active = false;
        let entries = if error.is_empty() { std::mem::take(&mut self.entries) } else { Vec::new() };
        vec![ListOut::StopTimer, ListOut::Complete { entries, error: error.to_string() }]
    }

    pub fn on_timeout(&mut self) -> Vec<ListOut> {
        if !self.active {
            return Vec::new();
        }
        self.retries += 1;
        if self.retries > MAX_RETRY { self.complete("List directory failed") } else { self.request(false) }
    }

    pub fn on_payload(&mut self, payload: &[u8]) -> Vec<ListOut> {
        let Some(reply) = Request::decode(payload) else { return Vec::new() };
        if self.active && self.opcode == CMD_LIST_DIRECTORY_WITH_TIME && reply.req_opcode == CMD_LIST_DIRECTORY_WITH_TIME && reply.opcode == RSP_NAK && reply.data.first() == Some(&ERR_UNKNOWN_COMMAND) {
            self.opcode = CMD_LIST_DIRECTORY;
            self.time_unsupported = true;
            self.expected_offset = 0;
            self.entries.clear();
            self.expected_seq = reply.seq;
            let mut out = vec![ListOut::StopTimer];
            out.append(&mut self.request(true));
            return out;
        }
        if !self.active || reply.req_opcode != self.opcode || self.expected_seq.wrapping_sub(1).wrapping_sub(reply.seq) < u16::MAX / 2 {
            return Vec::new();
        }
        match reply.opcode {
            RSP_ACK => {
                if reply.seq < self.expected_seq {
                    return vec![ListOut::StopTimer];
                }
                let entries: Vec<String> = reply.data.split(|b| *b == 0).filter(|e| !e.is_empty()).map(|e| String::from_utf8_lossy(e).into_owned()).collect();
                self.expected_offset += entries.len() as u32;
                self.entries.extend(entries);
                self.expected_seq = reply.seq;
                let mut out = vec![ListOut::StopTimer];
                out.append(&mut self.request(true));
                out
            }
            RSP_NAK if reply.data.first() == Some(&ERR_EOF) => {
                if reply.seq != self.expected_seq { vec![ListOut::StartTimer] } else { self.complete("") }
            }
            RSP_NAK => self.complete("List directory failed"),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpOut {
    Send(Request),
    StartTimer,
    StopTimer,
    Progress(f64),
    Complete { error: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Create,
    Write,
    Reset,
    Remove,
    Terminate,
}

#[derive(Debug)]
pub struct FileOp {
    pub path: String,
    pub component: u8,
    step: Step,
    expected_seq: u16,
    session: u8,
    data: Vec<u8>,
    sent: usize,
    chunk: usize,
    retries: u32,
    active: bool,
}

impl FileOp {
    fn begin(from_component: u8, uri: &str, step: Step, data: Vec<u8>, expected_seq: u16) -> Result<(FileOp, Vec<OpOut>), String> {
        let (path, component) = parse_uri(from_component, uri)?;
        let mut op = FileOp { path, component, step, expected_seq, session: 0, data, sent: 0, chunk: 0, retries: 0, active: true };
        let out = op.request(true);
        Ok((op, out))
    }

    pub fn upload(from_component: u8, uri: &str, data: Vec<u8>, expected_seq: u16) -> Result<(FileOp, Vec<OpOut>), String> {
        Self::begin(from_component, uri, Step::Create, data, expected_seq)
    }

    pub fn remove(from_component: u8, uri: &str, expected_seq: u16) -> Result<(FileOp, Vec<OpOut>), String> {
        Self::begin(from_component, uri, Step::Remove, Vec::new(), expected_seq)
    }

    pub fn in_progress(&self) -> bool {
        self.active
    }

    pub fn expected_seq(&self) -> u16 {
        self.expected_seq
    }

    fn opcode(&self) -> u8 {
        match self.step {
            Step::Create => CMD_CREATE_FILE,
            Step::Write => CMD_WRITE_FILE,
            Step::Reset => CMD_RESET_SESSIONS,
            Step::Remove => CMD_REMOVE_FILE,
            Step::Terminate => CMD_TERMINATE_SESSION,
        }
    }

    fn request(&mut self, first: bool) -> Vec<OpOut> {
        if first {
            self.retries = 0;
        } else {
            self.expected_seq = self.expected_seq.wrapping_sub(2);
        }
        let (session, offset, data) = match self.step {
            Step::Create | Step::Remove => (0, 0, self.path.as_bytes().iter().copied().take(DATA_LEN).collect()),
            Step::Write => {
                self.chunk = (self.data.len() - self.sent).min(DATA_LEN);
                (self.session, self.sent as u32, self.data[self.sent..self.sent + self.chunk].to_vec())
            }
            Step::Reset => (0, 0, Vec::new()),
            Step::Terminate => (self.session, 0, Vec::new()),
        };
        let request = Request { seq: self.expected_seq.wrapping_add(1), session, opcode: self.opcode(), offset, data, ..Default::default() };
        self.expected_seq = self.expected_seq.wrapping_add(2);
        vec![OpOut::StartTimer, OpOut::Send(request)]
    }

    fn complete(&mut self, error: String) -> Vec<OpOut> {
        self.active = false;
        vec![OpOut::StopTimer, OpOut::Complete { error }]
    }

    fn upload_failed(&mut self, why: &str) -> Vec<OpOut> {
        let error = format!("Upload failed for: {} - {why}", self.path);
        self.complete(error)
    }

    fn advance(&mut self) -> Vec<OpOut> {
        match self.step {
            Step::Create | Step::Write if self.sent < self.data.len() => {
                self.step = Step::Write;
                self.request(true)
            }
            Step::Create | Step::Write => {
                self.step = Step::Reset;
                self.request(true)
            }
            Step::Reset | Step::Remove => self.complete(String::new()),
            Step::Terminate => {
                let error = format!("Aborted for: {}", self.path);
                self.complete(error)
            }
        }
    }

    pub fn cancel(&mut self) -> Vec<OpOut> {
        match self.step {
            _ if !self.active => Vec::new(),
            Step::Create | Step::Write | Step::Reset if self.session != 0 => {
                self.step = Step::Terminate;
                [vec![OpOut::StopTimer], self.request(true)].concat()
            }
            _ => self.complete("Aborted".to_string()),
        }
    }

    pub fn on_timeout(&mut self) -> Vec<OpOut> {
        if !self.active {
            return Vec::new();
        }
        self.retries += 1;
        match self.step {
            Step::Create => self.upload_failed("no response from vehicle"),
            Step::Write | Step::Terminate if self.retries > MAX_RETRY => self.upload_failed("no response from vehicle"),
            Step::Remove if self.retries > MAX_RETRY => self.complete("Delete failed".to_string()),
            Step::Reset => self.complete(String::new()),
            _ => self.request(false),
        }
    }

    pub fn on_payload(&mut self, payload: &[u8]) -> Vec<OpOut> {
        let Some(reply) = Request::decode(payload) else { return Vec::new() };
        if !self.active || reply.req_opcode != self.opcode() {
            return Vec::new();
        }
        let current = match self.step {
            Step::Write => reply.session == self.session && reply.seq >= self.expected_seq,
            _ => reply.seq == self.expected_seq,
        };
        if !current {
            return Vec::new();
        }
        match (reply.opcode, self.step) {
            (RSP_ACK, Step::Create) => {
                self.session = reply.session;
                [vec![OpOut::StopTimer], self.advance()].concat()
            }
            (RSP_ACK, Step::Write) => {
                self.sent += self.chunk;
                self.chunk = 0;
                self.expected_seq = reply.seq;
                let progress = OpOut::Progress(if self.data.is_empty() { 1.0 } else { self.sent as f64 / self.data.len() as f64 });
                [vec![OpOut::StopTimer, progress], self.advance()].concat()
            }
            (RSP_ACK, _) => [vec![OpOut::StopTimer], self.advance()].concat(),
            (RSP_NAK, Step::Create | Step::Write | Step::Terminate) => {
                let why = format!("error: {}", reply.nak_error());
                self.upload_failed(&why)
            }
            (RSP_NAK, Step::Remove) => {
                let error = format!("Delete failed: {}", reply.nak_error());
                self.complete(error)
            }
            (RSP_NAK, Step::Reset) => self.complete(String::new()),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(out: &[Out]) -> Request {
        out.iter().find_map(|o| match o { Out::Send(r) => Some(r.clone()), _ => None }).expect("a request was sent")
    }

    fn ack(seq: u16, session: u8, req_opcode: u8, offset: u32, data: &[u8], burst_complete: bool) -> Vec<u8> {
        Request { seq, session, opcode: RSP_ACK, req_opcode, burst_complete, offset, data: data.to_vec() }.encode().to_vec()
    }

    fn nak(seq: u16, session: u8, req_opcode: u8, code: u8) -> Vec<u8> {
        Request { seq, session, opcode: RSP_NAK, req_opcode, offset: 0, burst_complete: false, data: vec![code] }.encode().to_vec()
    }

    #[test]
    fn the_payload_codec_round_trips_and_the_uri_yields_path_and_component() {
        let request = Request { seq: 513, session: 3, opcode: CMD_READ_FILE, req_opcode: 0, burst_complete: true, offset: 70000, data: b"hello".to_vec() };
        let payload = request.encode();
        assert_eq!((payload.len(), payload[4], payload[6]), (PAYLOAD_LEN, 5, 1));
        assert_eq!(Request::decode(&payload).unwrap(), request);
        assert_eq!(Request::decode(&payload[..5]), None);
        assert_eq!(parse_uri(COMP_ID_ALL, "mftp://@PARAM/param.pck").unwrap(), ("/@PARAM/param.pck".to_string(), 1));
        assert_eq!(parse_uri(0, "/[;comp=100]/fs/microsd/log.bin").unwrap(), ("//fs/microsd/log.bin".to_string(), 100));
        assert_eq!(parse_uri(1, "/fs/file").unwrap(), ("/fs/file".to_string(), 1));
        assert!(parse_uri(1, "http://x").is_err());
        assert_eq!(parse_uri(1, "/fs/microsd/логи").unwrap().0, "/fs/microsd/логи");
        assert_eq!(parse_uri(1, "MFTP://a/b").unwrap().0, "/a/b");
        assert!(parse_uri(1, "[;comp=x]/f").is_err());
        assert_eq!(Request { data: vec![ERR_FAIL_ERRNO, 13], ..Default::default() }.nak_error(), "errno 13");
        assert_eq!(Request { data: vec![10], ..Default::default() }.nak_error(), "FailFileNotFound");
        assert_eq!(Request { data: vec![], ..Default::default() }.nak_error(), "Invalid Nak format");
    }

    #[test]
    fn a_burst_download_with_a_gap_fills_the_gap_and_completes_with_the_file() {
        let (mut download, out) = Download::start(1, "/fs/log.bin", true).unwrap();
        let open = sent(&out);
        assert_eq!((open.opcode, open.seq, open.data.as_slice()), (CMD_OPEN_FILE_RO, 1, b"/fs/log.bin".as_slice()));
        let file: Vec<u8> = (0..600u16).map(|i| i as u8).collect();
        let opened = download.on_payload(&ack(2, 7, CMD_OPEN_FILE_RO, 0, &(file.len() as u32).to_le_bytes(), false));
        let burst = sent(&opened);
        assert_eq!((burst.opcode, burst.session, burst.offset, burst.seq), (CMD_BURST_READ_FILE, 7, 0, 3));
        let first = download.on_payload(&ack(4, 7, CMD_BURST_READ_FILE, 0, &file[..239], false));
        assert!(matches!(first.last(), Some(Out::Progress(p)) if (*p - 239.0 / 600.0).abs() < 1e-9));
        let skipped = download.on_payload(&ack(5, 7, CMD_BURST_READ_FILE, 478, &file[478..], true));
        let reburst = sent(&skipped);
        assert_eq!((reburst.opcode, reburst.offset), (CMD_BURST_READ_FILE, 600));
        let eof = download.on_payload(&nak(reburst.seq + 1, 7, CMD_BURST_READ_FILE, ERR_EOF));
        let fill = sent(&eof);
        assert_eq!((fill.opcode, fill.offset, fill.data.len()), (CMD_READ_FILE, 239, 239));
        let filled = download.on_payload(&ack(fill.seq + 1, 7, CMD_READ_FILE, 239, &file[239..478], false));
        let reset = sent(&filled);
        assert_eq!(reset.opcode, CMD_RESET_SESSIONS);
        let done = download.on_payload(&ack(reset.seq + 1, 0, CMD_RESET_SESSIONS, 0, &[], false));
        assert!(matches!(done.last(), Some(Out::Complete { ok: true, bytes, .. }) if *bytes == file));
        assert!(!download.in_progress());
    }

    #[test]
    fn a_saved_download_is_written_at_each_offset_and_removed_on_failure_as_ftp_manager_does() {
        let dir = std::env::temp_dir().join(format!("groundstation-ftp-stream-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let local = dir.join("log.bin");
        let (mut download, _) = Download::start(1, "/fs/log.bin", true).unwrap();
        download.stream_to(&local).unwrap();
        let file: Vec<u8> = (0..400u16).map(|i| i as u8).collect();
        download.on_payload(&ack(2, 7, CMD_OPEN_FILE_RO, 0, &(file.len() as u32).to_le_bytes(), false));
        download.on_payload(&ack(4, 7, CMD_BURST_READ_FILE, 0, &file[..239], false));
        assert_eq!(std::fs::metadata(&local).unwrap().len(), 239, "each burst reaches the disk as it arrives, not at the end");
        let last = download.on_payload(&ack(5, 7, CMD_BURST_READ_FILE, 239, &file[239..], true));
        let reburst = sent(&last);
        let eof = download.on_payload(&nak(reburst.seq + 1, 7, CMD_BURST_READ_FILE, ERR_EOF));
        let reset = sent(&eof);
        let done = download.on_payload(&ack(reset.seq + 1, 0, CMD_RESET_SESSIONS, 0, &[], false));
        assert!(matches!(done.last(), Some(Out::Complete { ok: true, bytes, .. }) if bytes.is_empty()), "the bytes are on disk, not held in memory");
        assert_eq!(std::fs::read(&local).unwrap(), file);
        let kept = dir.join("kept.bin");
        std::fs::write(&kept, b"earlier copy").unwrap();
        let (mut missing, _) = Download::start(1, "/fs/missing", true).unwrap();
        missing.stream_to(&kept).unwrap();
        missing.on_payload(&nak(2, 0, CMD_OPEN_FILE_RO, 10));
        assert_eq!(std::fs::read(&kept).unwrap(), b"earlier copy", "_openFileROAckOrNak opens the file only on the ack, so a refused open leaves the old one alone");
        let partial = dir.join("partial.bin");
        let (mut broken, _) = Download::start(1, "/fs/log.bin", true).unwrap();
        broken.stream_to(&partial).unwrap();
        broken.on_payload(&ack(2, 7, CMD_OPEN_FILE_RO, 0, &400u32.to_le_bytes(), false));
        broken.on_payload(&ack(4, 7, CMD_BURST_READ_FILE, 0, &file[..239], false));
        (0..4).for_each(|_| { broken.on_timeout(); });
        assert!(!partial.exists(), "a download that fails after opening removes what it wrote");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failures_retries_and_cancel_follow_the_manager() {
        let (mut download, _) = Download::start(1, "/fs/missing", true).unwrap();
        let refused = download.on_payload(&nak(2, 0, CMD_OPEN_FILE_RO, 10));
        assert!(matches!(refused.last(), Some(Out::Complete { ok: false, error, .. }) if error == "Download failed: FailFileNotFound"));
        let (mut stalled, _) = Download::start(1, "/fs/a", false).unwrap();
        stalled.on_payload(&ack(2, 1, CMD_OPEN_FILE_RO, 0, &100u32.to_le_bytes(), false));
        let retries: Vec<Vec<Out>> = (0..4).map(|_| stalled.on_timeout()).collect();
        assert!(retries[..3].iter().all(|r| r.iter().any(|o| matches!(o, Out::Send(req) if req.opcode == CMD_BURST_READ_FILE))));
        assert!(matches!(retries[3].last(), Some(Out::Complete { ok: false, .. })));
        let (mut stale, _) = Download::start(1, "/fs/b", false).unwrap();
        assert!(stale.on_payload(&ack(1, 0, CMD_OPEN_FILE_RO, 0, &[], false)).is_empty());
        let (mut cancelled, _) = Download::start(1, "/fs/c", false).unwrap();
        cancelled.on_payload(&ack(2, 4, CMD_OPEN_FILE_RO, 0, &10u32.to_le_bytes(), false));
        let terminate = sent(&cancelled.cancel());
        assert_eq!((terminate.opcode, terminate.session), (CMD_TERMINATE_SESSION, 4));
        let ended = cancelled.on_payload(&ack(terminate.seq + 1, 4, CMD_TERMINATE_SESSION, 0, &[], false));
        assert!(matches!(ended.last(), Some(Out::Complete { ok: false, error, .. }) if error == "Download cancelled"));
        let (mut short_file, _) = Download::start(1, "/fs/d", true).unwrap();
        short_file.on_payload(&ack(2, 2, CMD_OPEN_FILE_RO, 0, &50u32.to_le_bytes(), false));
        let short = short_file.on_payload(&nak(4, 2, CMD_BURST_READ_FILE, ERR_EOF));
        assert!(matches!(short.last(), Some(Out::Complete { ok: false, .. })), "a short file with checksize fails");
        let (mut hostile, _) = Download::start(1, "/fs/e", false).unwrap();
        hostile.on_payload(&ack(2, 3, CMD_OPEN_FILE_RO, 0, &600u32.to_le_bytes(), false));
        let bomb = hostile.on_payload(&ack(4, 3, CMD_BURST_READ_FILE, 0xFFFF_FF00, &[1, 2, 3], false));
        assert!(matches!(bomb.last(), Some(Out::Complete { ok: false, .. })), "an offset past the file size is refused");
        let (seeded, out) = Download::start_from(1, "/fs/f", false, 40).unwrap();
        assert_eq!(sent(&out).seq, 41);
        assert_eq!(seeded.expected_seq(), 42);
    }

    #[test]
    fn a_timed_listing_falls_back_to_the_plain_one_when_the_server_does_not_know_it() {
        let sent_of = |out: &[ListOut]| out.iter().find_map(|o| match o { ListOut::Send(r) => Some(r.clone()), _ => None }).unwrap();
        let (mut listing, out) = Listing::start_with_time(1, "@MAV_LOG", 0).unwrap();
        assert_eq!(sent_of(&out).opcode, CMD_LIST_DIRECTORY_WITH_TIME);
        let retry = listing.on_payload(&Request { seq: 2, opcode: RSP_NAK, req_opcode: CMD_LIST_DIRECTORY_WITH_TIME, data: vec![ERR_UNKNOWN_COMMAND], ..Default::default() }.encode());
        let plain = sent_of(&retry);
        assert_eq!((plain.opcode, plain.offset), (CMD_LIST_DIRECTORY, 0));
        assert!(listing.time_unsupported);
        let page = listing.on_payload(&Request { seq: plain.seq + 1, opcode: RSP_ACK, req_opcode: CMD_LIST_DIRECTORY, data: b"Fa.ulg\t10\0".to_vec(), ..Default::default() }.encode());
        let next = sent_of(&page);
        let done = listing.on_payload(&Request { seq: next.seq + 1, opcode: RSP_NAK, req_opcode: CMD_LIST_DIRECTORY, data: vec![ERR_EOF], ..Default::default() }.encode());
        assert!(matches!(done.last(), Some(ListOut::Complete { entries, error }) if entries == &vec!["Fa.ulg\t10".to_string()] && error.is_empty()));
    }

    #[test]
    fn a_directory_listing_pages_by_entry_count_until_the_end_of_file() {
        let (mut listing, out) = Listing::start(1, "/fs/microsd").unwrap();
        let first = out.iter().find_map(|o| match o { ListOut::Send(r) => Some(r.clone()), _ => None }).unwrap();
        assert_eq!((first.opcode, first.offset, first.seq, first.data.as_slice()), (CMD_LIST_DIRECTORY, 0, 1, b"/fs/microsd".as_slice()));
        let page = b"Flog1.bin\t4096\0Dlogs\0S\0".to_vec();
        let next = listing.on_payload(&Request { seq: 2, opcode: RSP_ACK, req_opcode: CMD_LIST_DIRECTORY, data: page, ..Default::default() }.encode());
        let second = next.iter().find_map(|o| match o { ListOut::Send(r) => Some(r.clone()), _ => None }).unwrap();
        assert_eq!((second.offset, second.seq), (3, 3));
        let stale = listing.on_payload(&Request { seq: 1, opcode: RSP_ACK, req_opcode: CMD_LIST_DIRECTORY, data: b"Fx\0".to_vec(), ..Default::default() }.encode());
        assert!(stale.is_empty());
        let done = listing.on_payload(&Request { seq: 4, opcode: RSP_NAK, req_opcode: CMD_LIST_DIRECTORY, data: vec![ERR_EOF], ..Default::default() }.encode());
        assert_eq!(done.last(), Some(&ListOut::Complete { entries: vec!["Flog1.bin\t4096".into(), "Dlogs".into(), "S".into()], error: String::new() }));
        assert!(!listing.in_progress());
        let (mut failing, _) = Listing::start(1, "/nope").unwrap();
        let retried: Vec<Vec<ListOut>> = (0..4).map(|_| failing.on_timeout()).collect();
        assert!(retried[2].iter().any(|o| matches!(o, ListOut::Send(r) if r.seq == 1)));
        assert!(matches!(retried[3].last(), Some(ListOut::Complete { error, .. }) if error == "List directory failed"));
    }

    fn op_ack(to: &Request, opcode: u8, session: u8, data: Vec<u8>) -> [u8; PAYLOAD_LEN] {
        Request { seq: to.seq.wrapping_add(1), session, opcode, req_opcode: to.opcode, data, ..Default::default() }.encode()
    }

    fn op_sent(out: &[OpOut]) -> Request {
        out.iter().find_map(|o| match o { OpOut::Send(r) => Some(r.clone()), _ => None }).expect("a request is sent")
    }

    #[test]
    fn an_upload_creates_writes_in_chunks_then_resets_the_sessions() {
        let data: Vec<u8> = (0..300u16).map(|i| i as u8).collect();
        let (mut op, out) = FileOp::upload(1, "/APM/scripts/hello.lua", data.clone(), 0).unwrap();
        let create = op_sent(&out);
        assert_eq!((create.opcode, create.data.as_slice()), (CMD_CREATE_FILE, b"/APM/scripts/hello.lua".as_slice()));
        let first = op_sent(&op.on_payload(&op_ack(&create, RSP_ACK, 4, vec![])));
        assert_eq!((first.opcode, first.session, first.offset, first.data.len()), (CMD_WRITE_FILE, 4, 0, DATA_LEN));
        let out = op.on_payload(&op_ack(&first, RSP_ACK, 4, vec![]));
        let second = op_sent(&out);
        assert_eq!((second.offset as usize, second.data.len()), (DATA_LEN, 300 - DATA_LEN));
        assert_eq!([first.data, second.data.clone()].concat(), data);
        let reset = op_sent(&op.on_payload(&op_ack(&second, RSP_ACK, 4, vec![])));
        assert_eq!(reset.opcode, CMD_RESET_SESSIONS);
        assert_eq!(op.on_payload(&op_ack(&reset, RSP_ACK, 0, vec![])).last(), Some(&OpOut::Complete { error: String::new() }));
        assert!(!op.in_progress());
    }

    #[test]
    fn a_cancelled_upload_terminates_its_session_like_ftp_manager() {
        let (mut op, out) = FileOp::upload(1, "/APM/scripts/x.lua", vec![1; 300], 0).unwrap();
        let create = op_sent(&out);
        let write = op_sent(&op.on_payload(&op_ack(&create, RSP_ACK, 4, vec![])));
        let terminate = op_sent(&op.cancel());
        assert_eq!((terminate.opcode, terminate.session), (CMD_TERMINATE_SESSION, 4));
        assert!(op.on_payload(&op_ack(&write, RSP_ACK, 4, vec![])).is_empty(), "the write ack in flight is ignored");
        assert_eq!(op.on_payload(&op_ack(&terminate, RSP_ACK, 4, vec![])).last(), Some(&OpOut::Complete { error: "Aborted for: /APM/scripts/x.lua".into() }));
        let (mut unopened, _) = FileOp::upload(1, "/APM/scripts/x.lua", vec![1; 10], 0).unwrap();
        assert_eq!(unopened.cancel().last(), Some(&OpOut::Complete { error: "Aborted".into() }));
        assert!(unopened.cancel().is_empty());
        let (mut silent, out) = FileOp::upload(1, "/APM/scripts/x.lua", vec![1; 10], 0).unwrap();
        silent.on_payload(&op_ack(&op_sent(&out), RSP_ACK, 2, vec![]));
        silent.cancel();
        let retries: Vec<Vec<OpOut>> = (0..=MAX_RETRY).map(|_| silent.on_timeout()).collect();
        assert!(matches!(op_sent(&retries[0]), Request { opcode: CMD_TERMINATE_SESSION, session: 2, .. }));
        assert_eq!(retries.last().unwrap().last(), Some(&OpOut::Complete { error: "Upload failed for: /APM/scripts/x.lua - no response from vehicle".into() }));
    }

    #[test]
    fn a_nak_or_silence_fails_with_qgcs_wording() {
        let (mut op, out) = FileOp::upload(1, "/APM/scripts/x.lua", vec![1; 10], 0).unwrap();
        let create = op_sent(&out);
        let failed = op.on_payload(&op_ack(&create, RSP_NAK, 0, vec![ERR_FAIL_ERRNO, 13]));
        assert_eq!(failed.last(), Some(&OpOut::Complete { error: "Upload failed for: /APM/scripts/x.lua - error: errno 13".into() }));
        let (mut silent, _) = FileOp::upload(1, "/APM/scripts/x.lua", vec![1; 10], 0).unwrap();
        assert_eq!(silent.on_timeout().last(), Some(&OpOut::Complete { error: "Upload failed for: /APM/scripts/x.lua - no response from vehicle".into() }));
        let (mut remove, out) = FileOp::remove(1, "/APM/scripts/x.lua", 0).unwrap();
        assert_eq!(op_sent(&out).opcode, CMD_REMOVE_FILE);
        (0..MAX_RETRY).for_each(|_| assert!(matches!(remove.on_timeout().last(), Some(OpOut::Send(_)))));
        assert_eq!(remove.on_timeout().last(), Some(&OpOut::Complete { error: "Delete failed".into() }));
    }
}
