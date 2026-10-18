use std::path::Path;

#[cfg(not(target_family = "wasm"))]
pub fn read(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

#[cfg(not(target_family = "wasm"))]
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    std::fs::write(path, contents)
}

#[cfg(not(target_family = "wasm"))]
pub fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

#[cfg(not(target_family = "wasm"))]
pub fn exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    File,
    Dir,
}

#[cfg(not(target_family = "wasm"))]
pub fn kind(path: impl AsRef<Path>) -> Option<Kind> {
    std::fs::metadata(path).ok().map(|meta| if meta.is_dir() { Kind::Dir } else { Kind::File })
}

#[cfg(not(target_family = "wasm"))]
pub fn replace(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let target = path.as_ref();
    let staged = target.with_extension(format!("{}.part", target.extension().and_then(|e| e.to_str()).unwrap_or("")));
    std::fs::write(&staged, contents).and_then(|()| std::fs::rename(&staged, target))
}

#[cfg(not(target_family = "wasm"))]
pub fn create_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(target_family = "wasm")]
pub fn create_dir_all(_path: impl AsRef<Path>) -> std::io::Result<()> {
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn replace(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    write(path, contents)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub dir: bool,
}

#[cfg(not(target_family = "wasm"))]
pub fn list(dir: impl AsRef<Path>) -> Vec<Entry> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| std::fs::metadata(entry.path()).ok().map(|meta| Entry { name: entry.file_name().to_string_lossy().into_owned(), size: meta.len(), dir: meta.is_dir() }))
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(not(target_family = "wasm"))]
#[derive(Debug)]
pub struct Sink(std::fs::File);

#[cfg(not(target_family = "wasm"))]
impl Sink {
    pub fn create(path: impl AsRef<Path>) -> std::io::Result<Sink> {
        std::fs::File::create(path).map(Sink)
    }

    pub fn set_len(&mut self, len: u64) -> std::io::Result<()> {
        self.0.set_len(len)
    }

    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        self.0.seek(SeekFrom::Start(offset)).and_then(|_| self.0.write_all(data))
    }

    pub fn sync(&mut self) {}

    pub fn finish(self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(target_family = "wasm")]
#[derive(Debug)]
pub struct Sink {
    path: std::path::PathBuf,
    bytes: std::io::Cursor<Vec<u8>>,
    dirty_from: Option<u64>,
}

#[cfg(target_family = "wasm")]
impl Sink {
    pub fn create(path: impl AsRef<Path>) -> std::io::Result<Sink> {
        let path = path.as_ref().to_path_buf();
        if let Some(url) = on_device(&path) {
            remote::send(remote::Change { path: path.clone(), url, body: remote::Body::Put(Vec::new()) });
        }
        Ok(Sink { path, bytes: std::io::Cursor::default(), dirty_from: None })
    }

    pub fn set_len(&mut self, len: u64) -> std::io::Result<()> {
        self.bytes.get_mut().resize(len as usize, 0);
        self.dirty_from = Some(self.dirty_from.map_or(0, |from| from.min(len)));
        Ok(())
    }

    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        self.dirty_from = Some(self.dirty_from.map_or(offset, |from| from.min(offset)));
        self.bytes.seek(SeekFrom::Start(offset)).and_then(|_| self.bytes.write_all(data))
    }

    pub fn sync(&mut self) {
        if let (Some(from), Some(url)) = (self.dirty_from.take(), on_device(&self.path)) {
            let tail = self.bytes.get_ref().get(from as usize..).unwrap_or_default().to_vec();
            remote::send(remote::Change { path: self.path.clone(), url, body: remote::Body::Patch(from, tail) });
        }
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        self.sync();
        if on_device(&self.path).is_none() || remote::waiting(&self.path) {
            files().insert(self.path, self.bytes.into_inner());
        }
        Ok(())
    }
}

#[cfg(target_family = "wasm")]
fn on_device(path: &Path) -> Option<String> {
    let root = crate::settingsstore::save_root()?;
    let path = path.to_str()?;
    path.strip_prefix(root.as_str()).is_some_and(|rest| rest.starts_with('/')).then(|| crate::device::files(path)).flatten()
}

#[cfg(target_family = "wasm")]
const REMOTE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[cfg(any(target_family = "wasm", test))]
mod remote {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::time::Duration;

    pub enum Body {
        Put(Vec<u8>),
        Patch(u64, Vec<u8>),
        Delete,
    }

    pub struct Change {
        pub path: PathBuf,
        pub url: String,
        pub body: Body,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub enum Outcome {
        Stored,
        Refused(String),
        Unreached(String),
    }

    impl Change {
        pub fn request(&self) -> (&'static str, String, Option<&[u8]>) {
            match &self.body {
                Body::Put(bytes) => ("PUT", self.url.clone(), Some(bytes)),
                Body::Patch(offset, bytes) => ("PATCH", format!("{}?offset={offset}", self.url), Some(bytes)),
                Body::Delete => ("DELETE", self.url.clone(), None),
            }
        }
    }

    pub fn outcome(method: &str, answer: Result<(u16, Vec<u8>), String>) -> Outcome {
        match answer {
            Ok((status, _)) if (200..300).contains(&status) || (method == "DELETE" && status == 404) => Outcome::Stored,
            Ok((status, text)) if (400..500).contains(&status) && ![408, 429].contains(&status) => Outcome::Refused(format!("{status} {}", String::from_utf8_lossy(&text))),
            Ok((status, _)) => Outcome::Unreached(format!("the device answered {status}")),
            Err(error) => Outcome::Unreached(error),
        }
    }

    pub fn backoff(failures: u32) -> Duration {
        Duration::from_secs(1u64 << failures.min(5)).min(Duration::from_secs(30))
    }

    thread_local! {
        static WAITING: RefCell<VecDeque<Change>> = const { RefCell::new(VecDeque::new()) };
        static RETRYING: Cell<bool> = const { Cell::new(false) };
    }

    pub fn waiting(path: &std::path::Path) -> bool {
        WAITING.with(|waiting| waiting.borrow().iter().any(|change| change.path == path))
    }

    #[cfg(target_family = "wasm")]
    pub fn send(change: Change) -> bool {
        if WAITING.with(|waiting| !waiting.borrow().is_empty()) {
            return wait(change);
        }
        let (method, url, body) = change.request();
        match outcome(method, crate::http::send(method, &url, &[], body, super::REMOTE_TIMEOUT)) {
            Outcome::Stored => true,
            Outcome::Refused(why) => {
                log::warn!("The device refused {method} {url}: {why}");
                false
            }
            Outcome::Unreached(why) => {
                log::warn!("The device did not take {method} {url} ({why}); trying again");
                wait(change)
            }
        }
    }

    #[cfg(target_family = "wasm")]
    fn wait(change: Change) -> bool {
        WAITING.with(|waiting| waiting.borrow_mut().push_back(change));
        if !RETRYING.with(|retrying| retrying.replace(true)) {
            wasm_bindgen_futures::spawn_local(retry());
        }
        false
    }

    #[cfg(target_family = "wasm")]
    async fn retry() {
        let mut failures = 0;
        while let Some((method, url, body, path)) = WAITING.with(|waiting| waiting.borrow().front().map(|change| {
            let (method, url, body) = change.request();
            (method, url, body.map(<[u8]>::to_vec), change.path.clone())
        })) {
            match outcome(method, crate::http::send_later(method, &url, &[], body.as_deref(), super::REMOTE_TIMEOUT).await) {
                Outcome::Unreached(_) => {
                    crate::background::pause(backoff(failures)).await;
                    failures += 1;
                }
                settled => {
                    if let Outcome::Refused(why) = settled {
                        log::warn!("The device refused {method} {url}: {why}");
                    }
                    failures = 0;
                    WAITING.with(|waiting| waiting.borrow_mut().pop_front());
                    if !waiting(&path) {
                        super::files().remove(&path);
                    }
                }
            }
        }
        RETRYING.with(|retrying| retrying.set(false));
    }
}

#[cfg(target_family = "wasm")]
fn remote_get(url: &str, path: &Path) -> std::io::Result<Vec<u8>> {
    crate::http::get(url, &[], REMOTE_TIMEOUT).map_err(|error| match error.as_str() {
        "http status: 404" => missing(path),
        _ => std::io::Error::other(error),
    })
}

#[cfg(target_family = "wasm")]
pub fn list(dir: impl AsRef<Path>) -> Vec<Entry> {
    let dir = dir.as_ref();
    let held: Vec<Entry> = files()
        .iter()
        .filter_map(|(path, bytes)| {
            let rest = path.strip_prefix(dir).ok()?;
            let mut parts = rest.components();
            let name = parts.next()?.as_os_str().to_string_lossy().into_owned();
            Some(match parts.next() {
                Some(_) => Entry { name, size: 0, dir: true },
                None => Entry { name, size: bytes.len() as u64, dir: false },
            })
        })
        .collect();
    let stored: Vec<Entry> = on_device(dir).and_then(|url| remote_get(&url, dir).ok()).and_then(|answer| serde_json::from_slice(&answer).ok()).unwrap_or_default();
    held.into_iter().chain(stored).fold(Vec::new(), |listed: Vec<Entry>, entry| match listed.iter().any(|seen| seen.name == entry.name) {
        true => listed,
        false => listed.into_iter().chain(std::iter::once(entry)).collect(),
    })
}

#[cfg(target_family = "wasm")]
pub fn kind(path: impl AsRef<Path>) -> Option<Kind> {
    let path = path.as_ref();
    let held = {
        let files = files();
        match () {
            _ if files.contains_key(path) => Some(Kind::File),
            _ if files.keys().any(|key| key != path && key.starts_with(path)) => Some(Kind::Dir),
            _ => None,
        }
    };
    held.or_else(|| {
        on_device(path)?;
        let (dir, name) = path.parent().zip(path.file_name())?;
        list(dir).into_iter().find(|entry| std::ffi::OsStr::new(&entry.name) == name).map(|entry| if entry.dir { Kind::Dir } else { Kind::File })
    })
}

#[cfg(target_family = "wasm")]
pub fn exists(path: impl AsRef<Path>) -> bool {
    kind(path).is_some()
}

#[cfg(target_family = "wasm")]
static FILES: std::sync::LazyLock<std::sync::Mutex<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>>> = std::sync::LazyLock::new(Default::default);

#[cfg(target_family = "wasm")]
fn files() -> std::sync::MutexGuard<'static, std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>> {
    FILES.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(target_family = "wasm")]
fn missing(path: &Path) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} is not in the browser's file store", path.display()))
}

#[cfg(target_family = "wasm")]
pub fn read(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let path = path.as_ref();
    let held = files().get(path).cloned();
    match (held, on_device(path)) {
        (Some(bytes), _) => Ok(bytes),
        (None, None) => Err(missing(path)),
        (None, Some(url)) => remote_get(&url, path),
    }
}

#[cfg(target_family = "wasm")]
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = path.as_ref();
    let stored = on_device(path).is_some_and(|url| remote::send(remote::Change { path: path.to_path_buf(), url, body: remote::Body::Put(contents.as_ref().to_vec()) }));
    match stored {
        true => files().remove(path),
        false => files().insert(path.to_path_buf(), contents.as_ref().to_vec()),
    };
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    let held = files().remove(path).is_some();
    match on_device(path) {
        Some(url) => {
            remote::send(remote::Change { path: path.to_path_buf(), url, body: remote::Body::Delete });
            Ok(())
        }
        None if held => Ok(()),
        None => Err(missing(path)),
    }
}

#[cfg(target_family = "wasm")]
pub fn remove_prefixed(prefix: &str, keep: Option<&str>) {
    files().retain(|path, _| {
        let path = path.to_string_lossy();
        !path.starts_with(prefix) || keep == Some(path.as_ref())
    });
}

pub fn read_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    String::from_utf8(read(path)?).map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_written_file_reads_back_and_is_gone_after_removal() {
        let path = std::env::temp_dir().join(format!("groundstation-vfs-{}.txt", std::process::id()));
        super::write(&path, "plan").unwrap();
        assert_eq!(super::read_to_string(&path).unwrap(), "plan");
        super::remove(&path).unwrap();
        assert!(super::read(&path).is_err());
    }

    #[test]
    fn the_device_store_retries_what_it_could_not_reach_and_drops_what_it_refused() {
        use super::remote::{Outcome, backoff, outcome};
        assert_eq!(outcome("PUT", Ok((204, Vec::new()))), Outcome::Stored);
        assert_eq!(outcome("DELETE", Ok((404, Vec::new()))), Outcome::Stored, "deleting what is already gone is done");
        assert_eq!(outcome("PUT", Ok((413, b"not enough space".to_vec()))), Outcome::Refused("413 not enough space".into()), "a full device will not take it on a retry either");
        assert!(matches!(outcome("PUT", Ok((502, Vec::new()))), Outcome::Unreached(_)));
        assert!(matches!(outcome("PATCH", Ok((429, Vec::new()))), Outcome::Unreached(_)));
        assert!(matches!(outcome("PUT", Err("No answer from falcon.local".into())), Outcome::Unreached(_)));
        assert_eq!((backoff(0), backoff(3), backoff(9)), (std::time::Duration::from_secs(1), std::time::Duration::from_secs(8), std::time::Duration::from_secs(30)));
    }
}
