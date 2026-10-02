use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::params::ParamValue;

pub fn folder_for(settings: &Path) -> Option<PathBuf> {
    Some(settings.parent()?.join(settings.file_stem()?).join("ParamCache"))
}

pub fn file_for(settings: &Path, vehicle: u8, component: u8) -> Option<PathBuf> {
    folder_for(settings).map(|folder| folder.join(format!("{vehicle}_{component}.json")))
}

pub fn encoded(cache: &BTreeMap<String, ParamValue>) -> Value {
    Value::Object(cache.iter().map(|(name, value)| (name.clone(), json!([value.param_type(), value.encode().to_bits()]))).collect())
}

pub fn decoded(saved: &Value) -> BTreeMap<String, ParamValue> {
    saved
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(name, entry)| {
            let param_type = u8::try_from(entry.get(0)?.as_u64()?).ok()?;
            let bits = u32::try_from(entry.get(1)?.as_u64()?).ok()?;
            Some((name.clone(), ParamValue::decode(param_type, f32::from_bits(bits))?))
        })
        .collect()
}

pub fn load(vehicle: u8, component: u8) -> BTreeMap<String, ParamValue> {
    crate::settingsstore::file()
        .and_then(|settings| file_for(&settings, vehicle, component))
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .map(|saved| decoded(&saved))
        .unwrap_or_default()
}

pub fn save(vehicle: u8, component: u8, cache: &BTreeMap<String, ParamValue>) {
    let Some(path) = crate::settingsstore::file().and_then(|settings| file_for(&settings, vehicle, component)) else { return };
    let written = path.parent().map(std::fs::create_dir_all).transpose().ok().and_then(|_| std::fs::write(&path, encoded(cache).to_string()).ok());
    if written.is_none() {
        log::warn!("Failed to open cache file for writing {}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cache_reads_back_what_it_wrote_and_lives_beside_the_settings() {
        let cache: BTreeMap<String, ParamValue> = [("MPC_XY_VEL_MAX".to_string(), ParamValue::F32(12.5)), ("SYS_AUTOSTART".to_string(), ParamValue::I32(4001))].into_iter().collect();
        assert_eq!(decoded(&encoded(&cache)), cache);
        assert_eq!(file_for(Path::new("/cfg/QGroundControl.ini"), 1, 1), Some(PathBuf::from("/cfg/QGroundControl/ParamCache/1_1.json")), "ParameterManager::parameterCacheDir: the settings folder, the app name, ParamCache");
    }
}
