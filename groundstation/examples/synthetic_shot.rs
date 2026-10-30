use std::sync::Arc;
use std::time::{Duration, Instant};

use groundstation::synthetic::fetch::Network;
use groundstation::synthetic::render::Renderer;
use groundstation::synthetic::session::{Session, Target};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|at| args.get(at + 1))
        .cloned()
}

fn number(name: &str, fallback: f64) -> f64 {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

fn main() {
    let token = arg("--token").unwrap_or_default();
    let (width, height) = (1280, 720);
    let frames = number("--frames", 1.0) as usize;
    let yaw_per_frame = number("--yawrate", 0.0) / 30.0;
    let target = Target {
        latitude: number("--lat", -35.36213),
        longitude: number("--lng", 149.16514),
        above_home: number("--above", 30.0),
        home_latitude: number("--home-lat", -35.363262),
        home_longitude: number("--home-lng", 149.165237),
        heading: number("--heading", 200.0),
        pitch: number("--pitch", -15.0),
        roll: number("--roll", 0.0),
        fov: number("--fov", 70.0),
        aim: (0.0, 0.0),
        imagery: arg("--imagery").unwrap_or_else(|| "Bing Satellite".into()),
        north_speed: 0.0,
        east_speed: 0.0,
        climb_speed: 0.0,
        heading_rate: None,
        pitch_rate: None,
        roll_rate: 0.0,
        response: groundstation::synthetic::session::Response::default(),
        round_trip: None,
        bottles: Vec::new(),
        locked: false,
    };
    groundstation::terrainservice::set_cache_path(arg("--cache").map(std::path::PathBuf::from));
    let mut session = Session::new(&token, Arc::new(Network)).expect("fetch workers");
    let started = Instant::now();
    let seconds = Duration::from_secs_f64(number("--seconds", 40.0));
    let mut drawn = None;
    while started.elapsed() < seconds {
        drawn = session
            .frame(Some(&target), &[], width, height, Instant::now())
            .or(drawn);
        std::thread::sleep(Duration::from_millis(100));
    }
    drawn.expect("something to draw");
    let when = arg("--utc")
        .map(|utc| utc.parse().expect("an RFC 3339 time"))
        .unwrap_or_else(chrono::Utc::now);
    println!("{:?}", session.take_stats());
    println!("google took over after {:?}", session.handover());
    let mut renderer = Renderer::new().expect("a GPU adapter");
    let targets = renderer.targets(width, height);
    let picture = (0..frames.max(1))
        .map(|index| {
            let turned = Target {
                heading: target.heading + yaw_per_frame * index as f64,
                ..target.clone()
            };
            let frame = session
                .frame(Some(&turned), &[], width, height, Instant::now())
                .expect("a frame to draw");
            let (camera, meshes) = frame.view_at(None, width, height, when);
            println!("{} meshes in frame {index}", meshes.len());
            renderer
                .picture(&camera, &frame.backfill, &meshes, &targets, width, height)
                .expect("rendered picture")
        })
        .last()
        .expect("a picture");
    let out = arg("--out").unwrap_or_else(|| "synthetic.png".into());
    picture.save(&out).expect("saved picture");
    println!("wrote {out}");
}
