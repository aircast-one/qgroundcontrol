pub const HEADER_LEN: usize = 12;
pub const PAYLOAD_LEN: usize = 251;
pub const DATA_LEN: usize = PAYLOAD_LEN - HEADER_LEN;
pub const MAX_RETRY: u32 = 3;
pub const SCHEME: &str = "mavlinkftp";
pub const COMP_ID_ALL: u8 = 0;
pub const COMP_ID_AUTOPILOT1: u8 = 1;

pub const CMD_TERMINATE_SESSION: u8 = 1;
pub const CMD_RESET_SESSIONS: u8 = 2;
pub const CMD_LIST_DIRECTORY: u8 = 3;
pub const CMD_OPEN_FILE_RO: u8 = 4;
pub const CMD_READ_FILE: u8 = 5;
pub const CMD_BURST_READ_FILE: u8 = 15;
pub const RSP_ACK: u8 = 128;
pub const RSP_NAK: u8 = 129;
pub const ERR_FAIL_ERRNO: u8 = 2;
pub const ERR_EOF: u8 = 6;

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
    let mut path = if uri.len() >= prefix.len() && uri[..prefix.len()].eq_ignore_ascii_case(&prefix) { uri[prefix.len() - 1..].to_string() } else { uri.to_string() };
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
    missing: Vec<Missing>,
    retries: u32,
}

impl Download {
    pub fn start(from_component: u8, uri: &str, check_size: bool) -> Result<(Download, Vec<Out>), String> {
        let (path, component) = parse_uri(from_component, uri)?;
        let mut download = Download { path, component, check_size, phase: Some(Phase::Open), ..Default::default() };
        let mut request = Request { session: 0, opcode: CMD_OPEN_FILE_RO, data: download.path.as_bytes().iter().copied().take(DATA_LEN).collect(), ..Default::default() };
        let out = download.send(&mut request);
        Ok((download, out))
    }

    pub fn in_progress(&self) -> bool {
        self.phase.is_some_and(|p| p != Phase::Idle)
    }

    fn send(&mut self, request: &mut Request) -> Vec<Out> {
        request.seq = self.expected_seq.wrapping_add(1);
        self.expected_seq = self.expected_seq.wrapping_add(2);
        vec![Out::StartTimer, Out::Send(request.clone())]
    }

    fn fail(&mut self, error: &str) -> Vec<Out> {
        self.phase = None;
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

    fn write_at(&mut self, offset: u32, data: &[u8]) {
        let end = offset as usize + data.len();
        if self.file.len() < end {
            self.file.resize(end, 0);
        }
        self.file[offset as usize..end].copy_from_slice(data);
        self.written += data.len() as u32;
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
                if reply.offset != self.expected_offset {
                    if reply.offset > self.expected_offset {
                        self.missing.push(Missing { offset: self.expected_offset, bytes: reply.offset - self.expected_offset });
                    } else {
                        return vec![Out::StartTimer];
                    }
                }
                self.write_at(reply.offset, &reply.data);
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
                self.write_at(reply.offset, &reply.data);
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
        assert_eq!(parse_uri(COMP_ID_ALL, "mavlinkftp://@PARAM/param.pck").unwrap(), ("/@PARAM/param.pck".to_string(), 1));
        assert_eq!(parse_uri(0, "/[;comp=100]/fs/microsd/log.bin").unwrap(), ("//fs/microsd/log.bin".to_string(), 100));
        assert_eq!(parse_uri(1, "/fs/file").unwrap(), ("/fs/file".to_string(), 1));
        assert!(parse_uri(1, "http://x").is_err());
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
    }
}
