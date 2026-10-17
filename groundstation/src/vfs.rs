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

    pub fn finish(self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(target_family = "wasm")]
#[derive(Debug)]
pub struct Sink(std::path::PathBuf, std::io::Cursor<Vec<u8>>);

#[cfg(target_family = "wasm")]
impl Sink {
    pub fn create(path: impl AsRef<Path>) -> std::io::Result<Sink> {
        Ok(Sink(path.as_ref().to_path_buf(), std::io::Cursor::default()))
    }

    pub fn set_len(&mut self, len: u64) -> std::io::Result<()> {
        self.1.get_mut().resize(len as usize, 0);
        Ok(())
    }

    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> std::io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        self.1.seek(SeekFrom::Start(offset)).and_then(|_| self.1.write_all(data))
    }

    pub fn finish(self) -> std::io::Result<()> {
        write(self.0, self.1.into_inner())
    }
}

#[cfg(target_family = "wasm")]
pub fn exists(path: impl AsRef<Path>) -> bool {
    files().contains_key(path.as_ref())
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
    files().get(path.as_ref()).cloned().ok_or_else(|| missing(path.as_ref()))
}

#[cfg(target_family = "wasm")]
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    files().insert(path.as_ref().to_path_buf(), contents.as_ref().to_vec());
    Ok(())
}

#[cfg(target_family = "wasm")]
pub fn remove(path: impl AsRef<Path>) -> std::io::Result<()> {
    files().remove(path.as_ref()).map(drop).ok_or_else(|| missing(path.as_ref()))
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
