use std::io::{BufRead, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use groundstation::synthetic::fetch::Network;
use groundstation::synthetic::overlay::Overlays;
use groundstation::synthetic::render::Renderer;
use groundstation::synthetic::session::{Frame, Session, Target};
use serde_json::{Value, json};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

fn size(name: &str, fallback: u32) -> u32 {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

const GIVE_UP: Duration = Duration::from_secs(60);

fn drawn(
    session: &mut Session,
    target: &Target,
    width: u32,
    height: u32,
    settle: Duration,
) -> Option<Frame> {
    let started = Instant::now();
    let mut latest = None;
    loop {
        session.jump();
        latest = session
            .frame(Some(target), &[], width, height, Instant::now())
            .or(latest);
        let waited = started.elapsed();
        if (waited >= settle && latest.is_some()) || waited >= GIVE_UP {
            return latest;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn main() {
    let (width, height) = (size("--width", 1280), size("--height", 720));
    groundstation::terrainservice::set_cache_path(arg("--cache").map(std::path::PathBuf::from));
    let mut renderer = Renderer::new().expect("a GPU adapter");
    let mut session = Session::new(
        &arg("--token").unwrap_or_default(),
        Arc::new(Network),
        renderer.detail(),
    )
    .expect("fetch workers");
    let targets = renderer.targets(width, height);
    let out = std::io::stdout();
    std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .for_each(|line| {
            let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
            let Some(target) = Target::from_view(&request["pose"]) else {
                eprintln!("not a pose: {line}");
                return;
            };
            let overlays =
                Overlays::from_view(&json!({ "fenceTop": 0.0, "cars": request["cars"].clone() }));
            let settle = Duration::from_millis(request["settleMs"].as_u64().unwrap_or(0));
            let picture = drawn(&mut session, &target, width, height, settle).and_then(|frame| {
                let (camera, meshes) = frame.view(overlays.as_ref(), width, height);
                renderer
                    .picture(&camera, &frame.backfill, &meshes, &targets, width, height)
                    .ok()
            });
            let rgb: Vec<u8> = picture.map_or_else(
                || vec![0; (width * height * 3) as usize],
                |picture| picture.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect(),
            );
            let mut writer = out.lock();
            writer.write_all(&rgb).expect("stdout");
            writer.flush().expect("stdout");
        });
}
