use std::io::Write;
use std::path::Path;

const EXCLUDED: [&str; 5] = ["AP_Periph-", "Blimp-", "Copter-3.", "Plane-3.", "Rover-3."];

fn main() {
    println!("cargo:rerun-if-env-changed=QGC_APM_PARAMS_DIR");
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let cache = concat!(env!("CARGO_MANIFEST_DIR"), "/../.cache/CPM/ardupilotparams");
    let cached = || std::fs::read_dir(cache).ok()?.filter_map(Result::ok).map(|entry| entry.path()).find(|path| path.join("Copter-4.5").is_dir()).map(|path| path.to_string_lossy().into_owned());
    let source = std::env::var("QGC_APM_PARAMS_DIR").ok().filter(|dir| Path::new(dir).is_dir()).or_else(cached);
    let entries: Vec<(String, String)> = source
        .iter()
        .flat_map(|dir| {
            println!("cargo:rerun-if-changed={dir}");
            std::fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok).map(|entry| entry.path())
        })
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_string();
            let pdef = path.join("apm.pdef.json");
            (pdef.is_file() && !EXCLUDED.iter().any(|prefix| name.starts_with(prefix))).then_some((name, pdef.to_string_lossy().into_owned()))
        })
        .collect();
    let table: Vec<String> = entries
        .iter()
        .map(|(name, pdef)| {
            let bytes = std::fs::read(pdef).expect("pdef readable");
            let target = Path::new(&out).join(format!("{name}.json.gz"));
            let mut encoder = flate2::write::GzEncoder::new(std::fs::File::create(&target).expect("gz writable"), flate2::Compression::best());
            encoder.write_all(&bytes).expect("gz written");
            encoder.finish().expect("gz finished");
            format!("    ({name:?}, include_bytes!({:?})),", target.to_string_lossy())
        })
        .collect();
    let generated = format!("pub const APM_PARAMETER_FILES: &[(&str, &[u8])] = &[\n{}\n];\n", table.join("\n"));
    std::fs::write(Path::new(&out).join("apm_parameter_files.rs"), generated).expect("table written");
}
