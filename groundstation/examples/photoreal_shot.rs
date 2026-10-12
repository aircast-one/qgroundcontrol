use std::time::Instant;

use groundstation::photoreal::geo::{Camera, Pose};
use groundstation::photoreal::render::Renderer;
use groundstation::photoreal::tileset::{self, Tileset, GOOGLE_ROOT, ION_GOOGLE_ASSET};
use url::Url;

fn fetch(url: &Url) -> Option<Vec<u8>> {
    ureq::get(url.as_str()).call().ok()?.body_mut().with_config().limit(64 * 1024 * 1024).read_to_vec().ok()
}

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|at| args.get(at + 1)).cloned()
}

fn number(name: &str, fallback: f64) -> f64 {
    arg(name).and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

fn root_url() -> Option<Url> {
    match (arg("--google"), arg("--ion")) {
        (Some(key), _) => Url::parse_with_params(GOOGLE_ROOT, [("key", key)]).ok(),
        (None, Some(token)) => {
            let endpoint = Url::parse_with_params(&format!("https://api.cesium.com/v1/assets/{ION_GOOGLE_ASSET}/endpoint"), [("access_token", token)]).ok()?;
            let reply: serde_json::Value = serde_json::from_slice(&fetch(&endpoint)?).ok()?;
            Url::parse(reply["options"]["url"].as_str()?).ok()
        }
        (None, None) => None,
    }
}

fn main() {
    let url = root_url().expect("pass --google <Map Tiles API key> or --ion <Cesium ion token>");
    let root = tileset::root(&fetch(&url).expect("root tileset"), &url).expect("root tile");
    let mut tiles = Tileset::new(root, &url);
    tiles.maximum_screen_space_error = number("--sse", 16.0);
    let (width, height) = (1280, 720);
    let pose = Pose { latitude: number("--lat", -35.36213), longitude: number("--lng", 149.16514), height: number("--height", 633.0), heading: number("--heading", 200.0), pitch: number("--pitch", -15.0), roll: 0.0, fov: 60.0 };
    let camera = Camera::new(&pose, width, height);
    let started = Instant::now();
    let (meshes, rounds, bytes) = (0..200).try_fold((Vec::new(), 0, 0usize), |(_, rounds, bytes), _| {
        let (meshes, wanted) = tiles.select(&camera, 64);
        let fetched: Vec<(Vec<usize>, Option<Vec<u8>>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = wanted.into_iter().map(|request| scope.spawn(move || (request.path, fetch(&request.url)))).collect();
            handles.into_iter().filter_map(|handle| handle.join().ok()).collect()
        });
        let got: usize = fetched.iter().map(|(_, bytes)| bytes.as_ref().map_or(0, Vec::len)).sum();
        let count = fetched.len();
        fetched.into_iter().for_each(|(path, bytes)| tiles.deliver(&path, bytes));
        match count {
            0 => Err((meshes, rounds, bytes)),
            _ => Ok((meshes, rounds + 1, bytes + got)),
        }
    }).unwrap_or_else(|done| done);
    println!("{} meshes after {rounds} rounds, {} KB in {:.1} s", meshes.len(), bytes / 1024, started.elapsed().as_secs_f64());
    let mut renderer = Renderer::new().expect("a GPU adapter");
    let picture = renderer.picture(&camera, &meshes, width, height).expect("rendered picture");
    let out = arg("--out").unwrap_or_else(|| "photoreal.png".into());
    picture.save(&out).expect("saved picture");
    println!("wrote {out}");
}
