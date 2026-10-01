use crate::ftp::{Download, FileOp, ListOut, Listing, OpOut, Out, Request};

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Send(Request),
    StartTimer,
    StopTimer,
    Progress(f64),
    Done(Result<Outcome, String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Listed(Vec<String>),
    Downloaded(Vec<u8>),
    Uploaded,
    Deleted,
}

#[derive(Debug)]
enum Machine {
    List(Listing),
    Download(Download),
    Upload(FileOp),
    Delete(FileOp),
}

#[derive(Debug)]
pub struct Job {
    machine: Machine,
    pub path: String,
}

fn from_list(out: ListOut) -> Step {
    match out {
        ListOut::Send(r) => Step::Send(r),
        ListOut::StartTimer => Step::StartTimer,
        ListOut::StopTimer => Step::StopTimer,
        ListOut::Complete { entries, error } => Step::Done(if error.is_empty() { Ok(Outcome::Listed(entries)) } else { Err(error) }),
    }
}

fn from_download(out: Out) -> Step {
    match out {
        Out::Send(r) => Step::Send(r),
        Out::StartTimer => Step::StartTimer,
        Out::StopTimer => Step::StopTimer,
        Out::Progress(p) => Step::Progress(p),
        Out::Complete { ok, error, bytes } => Step::Done(if ok { Ok(Outcome::Downloaded(bytes)) } else { Err(error) }),
    }
}

fn from_op(out: OpOut, done: Outcome) -> Step {
    match out {
        OpOut::Send(r) => Step::Send(r),
        OpOut::StartTimer => Step::StartTimer,
        OpOut::StopTimer => Step::StopTimer,
        OpOut::Progress(p) => Step::Progress(p),
        OpOut::Complete { error } => Step::Done(if error.is_empty() { Ok(done) } else { Err(error) }),
    }
}

impl Job {
    pub fn list(component: u8, path: &str, seq: u16) -> Result<(Job, Vec<Step>), String> {
        let (listing, outs) = Listing::start_from(component, path, seq)?;
        Ok((Job { machine: Machine::List(listing), path: path.to_string() }, outs.into_iter().map(from_list).collect()))
    }

    pub fn list_with_time(component: u8, path: &str, seq: u16) -> Result<(Job, Vec<Step>), String> {
        let (listing, outs) = Listing::start_with_time(component, path, seq)?;
        Ok((Job { machine: Machine::List(listing), path: path.to_string() }, outs.into_iter().map(from_list).collect()))
    }

    pub fn is_list(&self) -> bool {
        matches!(self.machine, Machine::List(_))
    }

    pub fn is_delete(&self) -> bool {
        matches!(self.machine, Machine::Delete(_))
    }

    pub fn list_time_unsupported(&self) -> bool {
        matches!(&self.machine, Machine::List(m) if m.time_unsupported)
    }

    pub fn download(component: u8, path: &str, seq: u16) -> Result<(Job, Vec<Step>), String> {
        let (download, outs) = Download::start_from(component, path, false, seq)?;
        Ok((Job { machine: Machine::Download(download), path: path.to_string() }, outs.into_iter().map(from_download).collect()))
    }

    pub fn upload(component: u8, path: &str, data: Vec<u8>, seq: u16) -> Result<(Job, Vec<Step>), String> {
        let (op, outs) = FileOp::upload(component, path, data, seq)?;
        Ok((Job { machine: Machine::Upload(op), path: path.to_string() }, outs.into_iter().map(|o| from_op(o, Outcome::Uploaded)).collect()))
    }

    pub fn delete(component: u8, path: &str, seq: u16) -> Result<(Job, Vec<Step>), String> {
        let (op, outs) = FileOp::remove(component, path, seq)?;
        Ok((Job { machine: Machine::Delete(op), path: path.to_string() }, outs.into_iter().map(|o| from_op(o, Outcome::Deleted)).collect()))
    }

    pub fn component(&self) -> u8 {
        match &self.machine {
            Machine::List(m) => m.component,
            Machine::Download(m) => m.component,
            Machine::Upload(m) | Machine::Delete(m) => m.component,
        }
    }

    pub fn expected_seq(&self) -> u16 {
        match &self.machine {
            Machine::List(m) => m.expected_seq(),
            Machine::Download(m) => m.expected_seq(),
            Machine::Upload(m) | Machine::Delete(m) => m.expected_seq(),
        }
    }

    pub fn on_payload(&mut self, payload: &[u8]) -> Vec<Step> {
        match &mut self.machine {
            Machine::List(m) => m.on_payload(payload).into_iter().map(from_list).collect(),
            Machine::Download(m) => m.on_payload(payload).into_iter().map(from_download).collect(),
            Machine::Upload(m) => m.on_payload(payload).into_iter().map(|o| from_op(o, Outcome::Uploaded)).collect(),
            Machine::Delete(m) => m.on_payload(payload).into_iter().map(|o| from_op(o, Outcome::Deleted)).collect(),
        }
    }

    pub fn on_timeout(&mut self) -> Vec<Step> {
        match &mut self.machine {
            Machine::List(m) => m.on_timeout().into_iter().map(from_list).collect(),
            Machine::Download(m) => m.on_timeout().into_iter().map(from_download).collect(),
            Machine::Upload(m) => m.on_timeout().into_iter().map(|o| from_op(o, Outcome::Uploaded)).collect(),
            Machine::Delete(m) => m.on_timeout().into_iter().map(|o| from_op(o, Outcome::Deleted)).collect(),
        }
    }

    pub fn cancel(&mut self) -> Vec<Step> {
        match &mut self.machine {
            Machine::Download(m) => m.cancel().into_iter().map(from_download).collect(),
            _ => vec![Step::StopTimer, Step::Done(Err("Aborted".to_string()))],
        }
    }
}

#[derive(Debug, Default)]
pub struct Files {
    pub job: Option<Job>,
    pub due_ms: Option<u64>,
    pub progress: f64,
    pub last: Option<(String, Result<Outcome, String>)>,
    pub generation: u64,
}

impl Files {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_machine_reports_through_one_step_shape() {
        let (job, steps) = Job::delete(1, "/APM/scripts/a.lua", 0).unwrap();
        assert_eq!(job.component(), 1);
        assert!(matches!(steps.last(), Some(Step::Send(r)) if r.opcode == crate::ftp::CMD_REMOVE_FILE));
        let (mut listing, _) = Job::list(1, "/APM/scripts/", 0).unwrap();
        assert!(matches!(listing.cancel().last(), Some(Step::Done(Err(e))) if e == "Aborted"));
    }
}
