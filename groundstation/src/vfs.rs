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
            remote::send(remote::Change::Put(url, Vec::new()));
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
            remote::send(remote::Change::Patch(url, from, tail));
        }
    }

    pub fn finish(mut self) -> std::io::Result<()> {
        self.sync();
        files().insert(self.path, self.bytes.into_inner());
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

#[cfg(target_family = "wasm")]
mod remote {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    pub enum Change {
        Put(String, Vec<u8>),
        Patch(String, u64, Vec<u8>),
        Delete(String),
    }

    thread_local! {
        static OUTBOX: RefCell<VecDeque<Change>> = const { RefCell::new(VecDeque::new()) };
        static SENDING: Cell<bool> = const { Cell::new(false) };
    }

    pub fn send(change: Change) {
        OUTBOX.with(|outbox| outbox.borrow_mut().push_back(change));
        if !SENDING.with(|sending| sending.replace(true)) {
            wasm_bindgen_futures::spawn_local(drain());
        }
    }

    async fn drain() {
        while let Some(change) = OUTBOX.with(|outbox| outbox.borrow_mut().pop_front()) {
            let (method, url, body) = match change {
                Change::Put(url, bytes) => ("PUT", url, Some(bytes)),
                Change::Patch(url, offset, bytes) => ("PATCH", format!("{url}?offset={offset}"), Some(bytes)),
                Change::Delete(url) => ("DELETE", url, None),
            };
            match crate::http::send_later(method, &url, &[], body.as_deref(), super::REMOTE_TIMEOUT).await {
                Ok((status, _)) if (200..300).contains(&status) || (method == "DELETE" && status == 404) => {}
                Ok((status, answer)) => log::warn!("The device refused {method} {url}: {status} {}", String::from_utf8_lossy(&answer)),
                Err(error) => log::warn!("Could not reach the device for {method} {url}: {error}"),
            }
        }
        SENDING.with(|sending| sending.set(false));
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
        (None, Some(url)) => {
            let bytes = remote_get(&url, path)?;
            files().insert(path.to_path_buf(), bytes.clone());
            Ok(bytes)
        }
    }
}

#[cfg(target_family = "wasm")]
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = path.as_ref();
    files().insert(path.to_path_buf(), contents.as_ref().to_vec());
    if let Some(url) = on_device(path) {
        remote::send(remote::Change::Put(url, contents.as_ref().to_vec()));
    }
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    let path = path.as_ref();
    let held = files().remove(path).is_some();
    match on_device(path) {
        Some(url) => {
            remote::send(remote::Change::Delete(url));
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
}
