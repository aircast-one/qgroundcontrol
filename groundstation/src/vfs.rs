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
pub fn remove_prefixed(prefix: &str) {
    files().retain(|path, _| !path.to_string_lossy().starts_with(prefix));
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
