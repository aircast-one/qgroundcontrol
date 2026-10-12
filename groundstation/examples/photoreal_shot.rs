use std::time::{Duration, Instant};

use groundstation::photoreal::render::Renderer;
use groundstation::photoreal::session::{Session, Target};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|at| args.get(at + 1)).cloned()
}

fn number(name: &str, fallback: f64) -> f64 {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

fn main() {
    let token = arg("--token").unwrap_or_default();
    let (width, height) = (1280, 720);
    let target = Target {
        latitude: number("--lat", -35.36213),
        longitude: number("--lng", 149.16514),
        above_home: number("--above", 30.0),
        home_latitude: number("--home-lat", -35.363262),
        home_longitude: number("--home-lng", 149.165237),
        heading: number("--heading", 200.0),
        pitch: number("--pitch", -15.0),
        roll: 0.0,
        fov: number("--fov", 70.0),
        imagery: arg("--imagery").unwrap_or_else(|| "Bing Satellite".into()),
    };
    let mut session = Session::new(&token);
    let started = Instant::now();
    let seconds = Duration::from_secs_f64(number("--seconds", 40.0));
    let mut drawn = None;
    while started.elapsed() < seconds {
        drawn = session.frame(Some(&target), width, height).or(drawn);
        std::thread::sleep(Duration::from_millis(100));
    }
    let (camera, meshes) = drawn.expect("something to draw");
    println!("{} meshes after {:.1} s", meshes.len(), started.elapsed().as_secs_f64());
    let mut renderer = Renderer::new().expect("a GPU adapter");
    let picture = renderer.picture(&camera, &meshes, width, height).expect("rendered picture");
    let out = arg("--out").unwrap_or_else(|| "photoreal.png".into());
    picture.save(&out).expect("saved picture");
    println!("wrote {out}");
}
