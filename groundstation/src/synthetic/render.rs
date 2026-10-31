use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::{Arc, Weak};

use range_alloc::RangeAllocator;
use web_time::Instant;

use super::geo::Camera;
use super::mesh::{Layer, Mesh, solid};
use super::noise;
use super::open::Woods;

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const SKY: wgpu::Color = wgpu::Color {
    r: 0.3,
    g: 0.45,
    b: 0.66,
    a: 1.0,
};
const VERTEX_BYTES: u64 = 32;
const INDEX_BYTES: u64 = 4;
const ARENA_VERTICES: u32 = 1 << 19;
const ARENA_INDICES: u32 = 1 << 21;
const DRAW_BYTES: u64 = 128;
const SCENE_BYTES: u64 = 496;
const FIRST_DRAW_SLOTS: usize = 1024;
const MISSING_TEXTURE: [u8; 4] = [255, 255, 255, 255];
const LAYERS: [Layer; 6] = [
    Layer::Opaque,
    Layer::Building,
    Layer::Foliage,
    Layer::Water,
    Layer::Translucent,
    Layer::OnTop,
];

const ANISOTROPY: u16 = 16;
const BLOOM_MIPS: usize = 5;
const SURFACE: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const OCCLUSION: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
const SHADOWS: bool = true;
const TAA: bool = true;
const MOTION_BLUR: bool = true;
const HISTORY: f32 = 0.9;
const SHUTTER: f32 = 0.5;
const JITTERS: usize = 8;
const CASCADES: usize = 3;
const CASCADE_FAR_M: [f64; CASCADES] = [300.0, 2000.0, 10000.0];
const CASTER_REACH_M: f64 = 10000.0;
const NO_CLOUD_MAP: u32 = u32::BITS - 1;
const SKY_LIGHT_WIDTH: u32 = 64;

#[derive(Clone, Copy)]
struct Quality {
    low: bool,
    cloud_shrink: u32,
    samples: u32,
    shadow_size: u32,
    bloom: bool,
    god_rays: bool,
    woods: Woods,
    render_scale: f64,
    foliage_cascades: usize,
    ssao: bool,
}

const HIGH: Quality = Quality {
    low: false,
    cloud_shrink: NO_CLOUD_MAP,
    samples: 4,
    shadow_size: 2048,
    bloom: true,
    god_rays: true,
    woods: Woods::FULL,
    render_scale: 1.0,
    foliage_cascades: CASCADES,
    ssao: true,
};

const LOW: Quality = Quality {
    low: true,
    cloud_shrink: 2,
    samples: 1,
    shadow_size: 1024,
    bloom: false,
    god_rays: false,
    woods: Woods {
        reach_m: 600.0,
        split_m: [350.0, 150.0],
        far_keep: 0.3,
    },
    render_scale: 0.75,
    foliage_cascades: 1,
    ssao: false,
};

fn quality(info: &wgpu::AdapterInfo) -> Quality {
    let phone = cfg!(any(target_os = "android", target_os = "ios"));
    if phone || info.backend == wgpu::Backend::Gl || info.device_type == wgpu::DeviceType::Cpu {
        LOW
    } else {
        HIGH
    }
}

fn scene_shader(quality: Quality) -> String {
    format!(
        "const LOW_TIER = {};\nconst SHAPE_PERIOD = {:.1};\nconst DETAIL_PERIOD = {:.1};\nconst SKY_LIGHT_WIDTH = {:.1};\n{SHADER}",
        quality.low,
        noise::SHAPE_PERIOD as f64,
        noise::DETAIL_PERIOD as f64,
        f64::from(SKY_LIGHT_WIDTH)
    )
}
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
const DRAW_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4];
const SHADER: &str = r"
struct Scene { unproject: mat4x4<f32>, project: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32>, light: array<mat4x4<f32>, 3>, cascades: array<vec4<f32>, 3>, eye: vec4<f32>, previous: mat4x4<f32>, temporal: vec4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var tile_sampler: sampler;
@group(2) @binding(0) var shadow: texture_depth_2d_array;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
@group(2) @binding(2) var cloud_map: texture_2d<f32>;
@group(2) @binding(3) var cloud_map_sampler: sampler;
@group(2) @binding(4) var sky_map: texture_2d<f32>;
@group(3) @binding(0) var cloud_shape: texture_2d<f32>;
@group(3) @binding(1) var cloud_detail: texture_3d<f32>;
@group(3) @binding(2) var noise_sampler: sampler;
@group(3) @binding(3) var sky_light: texture_2d<f32>;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32> };
struct Placed {
    @location(3) mvp0: vec4<f32>, @location(4) mvp1: vec4<f32>, @location(5) mvp2: vec4<f32>, @location(6) mvp3: vec4<f32>,
    @location(7) model0: vec4<f32>, @location(8) model1: vec4<f32>, @location(9) model2: vec4<f32>, @location(10) model3: vec4<f32>,
};
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) seen: vec3<f32>, @location(2) normal: vec3<f32> };
struct SkyOut { @builtin(position) position: vec4<f32>, @location(0) screen: vec2<f32> };
struct Frag { @location(0) colour: vec4<f32>, @location(1) surface: vec4<f32> };
struct CloudOut { @location(0) cover: vec4<f32>, @location(1) sky: vec4<f32> };

const PLANET_M = 6371000.0;
const ATMOSPHERE_M = 6471000.0;
const RAYLEIGH_H = 8000.0;
const MIE_H = 1200.0;
const RAYLEIGH = vec3<f32>(5.8e-6, 13.5e-6, 33.1e-6);
const MIE = 21e-6;
const MIE_G = 0.76;
const SUN = vec3<f32>(20.0, 19.4, 18.4);
const GROUND_ALBEDO = vec3<f32>(0.24, 0.21, 0.17);
const AMBIENT_SKY = vec3<f32>(0.2, 0.36, 0.72);
const MULTIPLE_SCATTERING = 6.0;
const BAKED_SHADE = 0.6;
const PI = 3.14159265;
const CLOUDS = true;
const CLOUD_BASE_M = 1500.0;
const CLOUD_TOP_M = 2400.0;
const CLOUD_COVER = 0.38;
const CLOUD_SCALE = 1.0 / 3500.0;
const CLOUD_STEPS = 16;
const CLOUD_GRAZING_STEPS = 28;
const CLOUD_EXTINCTION = 0.014;
const CLOUD_ALBEDO = 0.92;
const CLOUD_DRIFT = vec2<f32>(5.0, 2.0);
const CLOUD_LIGHT_STEP_M = 220.0;
const AERIAL_SAMPLES = select(4, 2, LOW_TIER);
const FOG = true;
const FOG_DENSITY = 0.00006;
const FOG_HEIGHT_M = 400.0;

struct Air { light: vec3<f32>, through: vec3<f32> };

fn path_height(along: f32, up: f32) -> f32 {
    let rise = along * along + 2.0 * PLANET_M * along * up;
    return rise / (sqrt(PLANET_M * PLANET_M + rise) + PLANET_M);
}

fn to_space(up: f32) -> f32 {
    return -PLANET_M * up + sqrt(PLANET_M * PLANET_M * up * up + ATMOSPHERE_M * ATMOSPHERE_M - PLANET_M * PLANET_M);
}

fn sun_depth(height: f32) -> vec3<f32> {
    let toward = max(dot(scene.sun.xyz, scene.zenith.xyz), 0.03);
    return (RAYLEIGH * RAYLEIGH_H * exp(-height / RAYLEIGH_H) + vec3<f32>(MIE * 1.1 * MIE_H * exp(-height / MIE_H))) / toward;
}

fn scatter(direction: vec3<f32>, length: f32, samples: i32) -> Air {
    let up = dot(direction, scene.zenith.xyz);
    let toward_sun = dot(direction, scene.sun.xyz);
    let rayleigh_phase = 3.0 / (16.0 * PI) * (1.0 + toward_sun * toward_sun);
    let mie_phase = 3.0 / (8.0 * PI) * (1.0 - MIE_G * MIE_G) * (1.0 + toward_sun * toward_sun) / ((2.0 + MIE_G * MIE_G) * pow(1.0 + MIE_G * MIE_G - 2.0 * MIE_G * toward_sun, 1.5));
    var depth = vec3<f32>(0.0);
    var light = vec3<f32>(0.0);
    for (var i = 0; i < samples; i++) {
        let fraction = (f32(i) + 0.5) / f32(samples);
        let step = 2.0 * fraction * length / f32(samples);
        let height = max(path_height(fraction * fraction * length, up), 0.0);
        let rayleigh_density = exp(-height / RAYLEIGH_H) * step;
        let mie_density = exp(-height / MIE_H) * step;
        let segment = RAYLEIGH * rayleigh_density + vec3<f32>(MIE * 1.1 * mie_density);
        let through = exp(-(depth + 0.5 * segment + sun_depth(height)));
        depth += segment;
        light += through * (RAYLEIGH * rayleigh_phase * rayleigh_density + vec3<f32>(MIE * mie_phase * mie_density));
    }
    light = light * SUN + AMBIENT_SKY * (1.0 - exp(-depth * MULTIPLE_SCATTERING));
    return Air(light, exp(-depth));
}

fn sky_air(direction: vec3<f32>, samples: i32) -> Air {
    let up = dot(direction, scene.zenith.xyz);
    let length = select(to_space(up), -2.0 * PLANET_M * up + 20000.0, up < 0.0);
    return scatter(direction, length, samples);
}

fn sky(direction: vec3<f32>) -> vec3<f32> {
    let air = sky_air(direction, 8);
    let disc = smoothstep(0.99990, 0.99997, dot(direction, scene.sun.xyz));
    return air.light + air.through * SUN * disc;
}

fn hash(p: vec2<f32>) -> f32 {
    var q = fract(vec3<f32>(p.xyx) * 0.1031);
    q += dot(q, q.yzx + 33.33);
    return fract((q.x + q.y) * q.z);
}

fn gradient(cell: vec2<f32>) -> vec2<f32> {
    let angle = hash(cell) * 6.2831853;
    return vec2<f32>(cos(angle), sin(angle));
}

fn perlin(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = dot(gradient(cell), f);
    let b = dot(gradient(cell + vec2<f32>(1.0, 0.0)), f - vec2<f32>(1.0, 0.0));
    let c = dot(gradient(cell + vec2<f32>(0.0, 1.0)), f - vec2<f32>(0.0, 1.0));
    let d = dot(gradient(cell + vec2<f32>(1.0, 1.0)), f - vec2<f32>(1.0, 1.0));
    return 0.5 + 0.7 * mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn worley(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    var nearest = 4.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let offset = vec2<f32>(f32(x), f32(y));
            let point = offset + vec2<f32>(hash(cell + offset), hash(cell + offset + 71.3)) - f;
            nearest = min(nearest, dot(point, point));
        }
    }
    return 1.0 - sqrt(nearest);
}

fn worley_fbm(p: vec2<f32>) -> f32 {
    return worley(p) * 0.625 + worley(p * 2.0 + 11.0) * 0.25 + worley(p * 4.0 + 23.0) * 0.125;
}

fn value3(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash(cell.xy + cell.z * 57.0), hash(cell.xy + vec2<f32>(1.0, 0.0) + cell.z * 57.0), u.x);
    let b = mix(hash(cell.xy + vec2<f32>(0.0, 1.0) + cell.z * 57.0), hash(cell.xy + vec2<f32>(1.0, 1.0) + cell.z * 57.0), u.x);
    let c = mix(hash(cell.xy + (cell.z + 1.0) * 57.0), hash(cell.xy + vec2<f32>(1.0, 0.0) + (cell.z + 1.0) * 57.0), u.x);
    let d = mix(hash(cell.xy + vec2<f32>(0.0, 1.0) + (cell.z + 1.0) * 57.0), hash(cell.xy + vec2<f32>(1.0, 1.0) + (cell.z + 1.0) * 57.0), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

fn billow3(p: vec3<f32>) -> f32 {
    let coarse = 1.0 - abs(2.0 * value3(p) - 1.0);
    let medium = 1.0 - abs(2.0 * value3(p * 2.0 + 11.0) - 1.0);
    let fine = 1.0 - abs(2.0 * value3(p * 4.0 + 23.0) - 1.0);
    return coarse * 0.625 + medium * 0.25 + fine * 0.125;
}

fn remap(value: f32, low: f32, high: f32) -> f32 {
    return clamp((value - low) / (high - low), 0.0, 1.0);
}

fn dither(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))) + scene.sun.w * 0.618034);
}

fn henyey_greenstein(cosine: f32, g: f32) -> f32 {
    return (1.0 - g * g) / (4.0 * PI * pow(1.0 + g * g - 2.0 * g * cosine, 1.5));
}

fn cloud_noise(p: vec2<f32>) -> vec2<f32> {
    if (LOW_TIER) { return textureSampleLevel(cloud_shape, noise_sampler, p / SHAPE_PERIOD, 0.0).rg; }
    return vec2<f32>(perlin(p) * 0.625 + perlin(p * 2.0 + 13.0) * 0.25 + perlin(p * 4.0 + 29.0) * 0.125, worley_fbm(p * 1.5 + 5.0));
}

fn cloud_erosion(p: vec2<f32>, rise: f32) -> f32 {
    if (LOW_TIER) { return textureSampleLevel(cloud_detail, noise_sampler, vec3<f32>(p / DETAIL_PERIOD, rise), 0.0).r; }
    return billow3(vec3<f32>(p, rise * 2.5));
}

fn cloud_body(horizontal: vec2<f32>, rise: f32) -> f32 {
    let p = horizontal * CLOUD_SCALE + CLOUD_DRIFT * scene.zenith.w * CLOUD_SCALE;
    let noise = cloud_noise(p);
    let base = remap(noise.x, noise.y - 1.0, 1.0);
    let profile = smoothstep(0.0, 0.08, rise) * (1.0 - smoothstep(0.55, 1.0, rise));
    return remap(base * profile, 1.0 - CLOUD_COVER, 1.4 - CLOUD_COVER);
}

fn cloud_density(horizontal: vec2<f32>, height: f32, fineness: f32) -> f32 {
    let rise = clamp((height - CLOUD_BASE_M) / (CLOUD_TOP_M - CLOUD_BASE_M), 0.0, 1.0);
    let body = cloud_body(horizontal, rise);
    if (body <= 0.0) { return body; }
    let p = horizontal * CLOUD_SCALE * 9.0 + CLOUD_DRIFT * scene.zenith.w * CLOUD_SCALE * 1.4 + vec2<f32>(3.7, 1.3);
    let erosion = cloud_erosion(p, rise) * mix(0.35, 0.75, rise) * mix(0.6, 1.0, fineness);
    return remap(body, erosion, 1.0);
}

fn slab_reach(up: f32, limit: f32) -> vec2<f32> {
    let eye = scene.eye.w;
    if (eye >= CLOUD_BASE_M && eye <= CLOUD_TOP_M) { return vec2<f32>(0.0, min(select((CLOUD_TOP_M - eye) / up, (CLOUD_BASE_M - eye) / up, up < 0.0), limit)); }
    let edges = vec2<f32>(CLOUD_BASE_M - eye, CLOUD_TOP_M - eye) / up;
    let span = vec2<f32>(min(edges.x, edges.y), max(edges.x, edges.y));
    return vec2<f32>(span.x, min(span.y, limit));
}

fn clouds(direction: vec3<f32>, jitter: f32, limit: f32) -> vec4<f32> {
    let up = dot(direction, scene.zenith.xyz);
    let inside = scene.eye.w >= CLOUD_BASE_M && scene.eye.w <= CLOUD_TOP_M;
    let toward_slab = select(up, -up, scene.eye.w > CLOUD_TOP_M);
    if (!CLOUDS || (!inside && toward_slab < 0.08) || abs(up) < 0.002) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let fade = select(smoothstep(0.08, 0.25, toward_slab), 1.0, inside);
    let reach = slab_reach(up, limit);
    if (reach.y <= reach.x) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let start = reach.x;
    let steps = select(CLOUD_STEPS, CLOUD_GRAZING_STEPS, abs(up) < 0.3);
    let step = min(reach.y - start, 18000.0) / f32(steps);
    let east = normalize(cross(scene.zenith.xyz, vec3<f32>(0.0, 0.0, 1.0)));
    let north = cross(scene.zenith.xyz, east);
    let sun_up = dot(scene.sun.xyz, scene.zenith.xyz);
    let sunlight = SUN * exp(-sun_depth(CLOUD_BASE_M));
    let overhead = sky_air(scene.zenith.xyz, 2).light;
    let level = direction - scene.zenith.xyz * up;
    let sideways = sky_air(normalize(level + scene.zenith.xyz * 0.05), 2).light;
    let cosine = dot(direction, scene.sun.xyz);
    let phase = 4.0 * PI * (0.5 * henyey_greenstein(cosine, 0.4) + 0.3 * henyey_greenstein(cosine, -0.15) + 0.2 * henyey_greenstein(cosine, 0.9));
    let facing = clamp(0.5 - 0.5 * cosine, 0.0, 1.0);
    var through = 1.0;
    var light = vec3<f32>(0.0);
    for (var i = 0; i < steps; i++) {
        let t = start + (f32(i) + jitter) * step;
        let fineness = 1.0 - smoothstep(6000.0, 15000.0, t);
        let p = direction * t;
        let height = scene.eye.w + up * t;
        let density = cloud_density(vec2<f32>(dot(scene.eye.xyz + p, east), dot(scene.eye.xyz + p, north)), height, fineness);
        if (density <= 0.0) { continue; }
        let rise = clamp((height - CLOUD_BASE_M) / (CLOUD_TOP_M - CLOUD_BASE_M), 0.0, 1.0);
        var toward = 0.0;
        for (var j = 0; j < 2; j++) {
            let q = p + scene.sun.xyz * (f32(j) + 0.5) * CLOUD_LIGHT_STEP_M;
            toward += cloud_density(vec2<f32>(dot(scene.eye.xyz + q, east), dot(scene.eye.xyz + q, north)), height + sun_up * (f32(j) + 0.5) * CLOUD_LIGHT_STEP_M, 0.0) * CLOUD_LIGHT_STEP_M;
        }
        let beer = exp(-toward * CLOUD_EXTINCTION);
        let powder = 1.0 - exp(-toward * CLOUD_EXTINCTION * 2.0);
        let energy = beer * mix(1.0, powder, facing);
        let ambient = mix(sideways * 0.45, overhead, rise);
        let lit = sunlight * CLOUD_ALBEDO / PI * 0.8 * phase * energy + ambient * CLOUD_ALBEDO;
        let transmit = exp(-density * CLOUD_EXTINCTION * step);
        light += through * (1.0 - transmit) * lit;
        through *= transmit;
        if (through < 0.01) { break; }
    }
    return vec4<f32>(light * fade, mix(1.0, through, fade));
}

fn cloud_light(direction: vec3<f32>, jitter: f32, limit: f32) -> vec4<f32> {
    let cover = clouds(direction, jitter, limit);
    if (cover.a >= 1.0) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let near = scatter(direction, max(slab_reach(dot(direction, scene.zenith.xyz), limit).x, 1.0), 4);
    return vec4<f32>(cover.rgb * near.through + near.light * (1.0 - cover.a), cover.a);
}

fn screen_uv(screen: vec2<f32>) -> vec2<f32> {
    return screen * vec2<f32>(0.5, -0.5) + 0.5;
}

fn soft(map: texture_2d<f32>, uv: vec2<f32>) -> vec4<f32> {
    let half = 0.5 / vec2<f32>(textureDimensions(map));
    return 0.25 * (textureSampleLevel(map, cloud_map_sampler, uv + half * vec2<f32>(-1.0, -1.0), 0.0)
        + textureSampleLevel(map, cloud_map_sampler, uv + half * vec2<f32>(1.0, -1.0), 0.0)
        + textureSampleLevel(map, cloud_map_sampler, uv + half * vec2<f32>(-1.0, 1.0), 0.0)
        + textureSampleLevel(map, cloud_map_sampler, uv + half * vec2<f32>(1.0, 1.0), 0.0));
}

fn cloud_seen(screen: vec2<f32>, direction: vec3<f32>, pixel: vec2<f32>, limit: f32) -> vec4<f32> {
    if (LOW_TIER) { return soft(cloud_map, screen_uv(screen)); }
    return cloud_light(direction, dither(pixel), limit);
}

fn fog_amount(seen: vec3<f32>) -> f32 {
    if (!FOG) { return 0.0; }
    let climb = dot(seen, scene.zenith.xyz);
    let along = select(FOG_HEIGHT_M * (1.0 - exp(-climb / FOG_HEIGHT_M)) / climb, 1.0, abs(climb) < 1.0);
    return 1.0 - exp(-FOG_DENSITY * exp(-scene.eye.w / FOG_HEIGHT_M) * length(seen) * along);
}

fn level_frame() -> mat2x3<f32> {
    let east = normalize(cross(scene.zenith.xyz, vec3<f32>(0.0, 0.0, 1.0)));
    return mat2x3<f32>(east, cross(scene.zenith.xyz, east));
}

fn sky_light_at(direction: vec3<f32>, row: f32) -> vec3<f32> {
    let frame = level_frame();
    let around = atan2(dot(direction, frame[1]), dot(direction, frame[0])) / (2.0 * PI) + 0.5;
    return textureSampleLevel(sky_light, noise_sampler, vec2<f32>(around, (row + 0.5) / 3.0), 0.0).rgb;
}

fn overhead_light() -> vec3<f32> {
    if (LOW_TIER) { return sky_light_at(scene.zenith.xyz, 2.0); }
    return sky_air(scene.zenith.xyz, 2).light;
}

fn sideways_light(sideways: vec3<f32>) -> vec3<f32> {
    if (LOW_TIER) { return sky_light_at(sideways, 1.0); }
    return sky_air(sideways, 2).light;
}

@fragment fn fs_sky_light(f: SkyOut) -> @location(0) vec4<f32> {
    let frame = level_frame();
    let angle = (f.position.x / SKY_LIGHT_WIDTH - 0.5) * 2.0 * PI;
    let level = frame[0] * cos(angle) + frame[1] * sin(angle);
    if (f.position.y < 1.0) { return vec4<f32>(sky_air(normalize(level + scene.zenith.xyz * 0.02), 4).light, 1.0); }
    if (f.position.y < 2.0) { return vec4<f32>(sky_air(normalize(level + scene.zenith.xyz * 0.001), 2).light, 1.0); }
    return vec4<f32>(sky_air(scene.zenith.xyz, 2).light, 1.0);
}

fn fog_colour(direction: vec3<f32>) -> vec3<f32> {
    if (LOW_TIER) { return sky_light_at(direction, 0.0); }
    let level = direction - scene.zenith.xyz * dot(direction, scene.zenith.xyz);
    let horizon = select(level, vec3<f32>(0.0), dot(level, level) < 1e-6);
    return sky_air(normalize(horizon + scene.zenith.xyz * 0.02), 4).light;
}

fn fogged(colour: vec3<f32>, seen: vec3<f32>) -> vec3<f32> {
    return mix(colour, fog_colour(seen / max(length(seen), 1.0)), fog_amount(seen));
}

fn facing(normal: vec3<f32>, seen: vec3<f32>) -> vec3<f32> {
    return select(normalize(normal), -normalize(normal), dot(normal, seen) > 0.0);
}

fn shadowed(cascade: i32, position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let detail = scene.cascades[cascade];
    let facing_sun = clamp(dot(normal, scene.sun.xyz), 0.05, 1.0);
    let slope = clamp(sqrt(1.0 - facing_sun * facing_sun) / facing_sun, 0.0, 6.0);
    let lifted = position + normal * detail.y * (1.5 + slope);
    let seen = scene.light[cascade] * vec4<f32>(lifted, 1.0);
    let uv = seen.xy * vec2<f32>(0.5, -0.5) + 0.5;
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || seen.z > 1.0) { return 1.0; }
    let texel = 1.0 / vec2<f32>(textureDimensions(shadow));
    var reached = 0.0;
    let reach = select(1, 0, LOW_TIER);
    for (var y = -reach; y <= reach; y++) {
        for (var x = -reach; x <= reach; x++) {
            reached += textureSampleCompareLevel(shadow, shadow_sampler, uv + vec2<f32>(f32(x), f32(y)) * texel, cascade, seen.z - detail.z * (1.0 + slope));
        }
    }
    return reached / f32((2 * reach + 1) * (2 * reach + 1));
}

fn sunlight_reaching(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let distance = length(position);
    if (distance >= scene.cascades[2].x) { return 1.0; }
    let cascade = select(select(0, 1, distance >= scene.cascades[0].x), 2, distance >= scene.cascades[1].x);
    let near = shadowed(cascade, position, normal);
    let edge = scene.cascades[cascade].x;
    let fade = smoothstep(edge * 0.85, edge, distance);
    if (fade <= 0.0) { return near; }
    return mix(near, select(shadowed(cascade + 1, position, normal), 1.0, cascade == 2), fade);
}

fn sunlit(colour: vec3<f32>, normal: vec3<f32>, seen: vec3<f32>, lit: f32) -> vec3<f32> {
    let sunlight = SUN * exp(-sun_depth(0.0)) / PI;
    if (dot(normal, normal) < 0.25) { return colour * (sunlight * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0) + overhead_light()) * mix(BAKED_SHADE, 1.0, lit); }
    let outward = facing(normal, seen);
    let up = dot(outward, scene.zenith.xyz);
    let sideways = normalize(seen - scene.zenith.xyz * dot(seen, scene.zenith.xyz));
    let skylight = mix(sideways_light(sideways), overhead_light(), 0.5 + 0.5 * up);
    let bounce = GROUND_ALBEDO * sunlight * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0) * (0.5 - 0.5 * up);
    return colour * (sunlight * max(dot(outward, scene.sun.xyz), 0.0) * lit + skylight + bounce);
}

@vertex fn vs(v: In, placed: Placed) -> Out {
    let mvp = mat4x4<f32>(placed.mvp0, placed.mvp1, placed.mvp2, placed.mvp3);
    let model = mat4x4<f32>(placed.model0, placed.model1, placed.model2, placed.model3);
    return Out(mvp * vec4<f32>(v.position, 1.0), v.uv, (model * vec4<f32>(v.position, 1.0)).xyz, (model * vec4<f32>(v.normal, 0.0)).xyz);
}

fn lit_surface(colour: vec4<f32>, f: Out) -> Frag {
    let distance = max(length(f.seen), 1.0);
    let air = scatter(f.seen / distance, distance, AERIAL_SAMPLES);
    let normal = select(vec3<f32>(0.0), facing(f.normal, f.seen), dot(f.normal, f.normal) >= 0.25);
    let lit = sunlight_reaching(f.seen, normal);
    let shaded = fogged(sunlit(colour.rgb, f.normal, f.seen, lit) * air.through + air.light, f.seen);
    if (scene.eye.w > CLOUD_BASE_M) {
        let clip = scene.project * vec4<f32>(f.seen, 1.0);
        let cover = cloud_seen(clip.xy / clip.w, f.seen / distance, f.position.xy, distance);
        return Frag(vec4<f32>(shaded * cover.a + cover.rgb, colour.a), vec4<f32>(normal, distance));
    }
    return Frag(vec4<f32>(shaded, colour.a), vec4<f32>(normal, distance));
}

@fragment fn fs(f: Out) -> Frag {
    return lit_surface(textureSample(picture, tile_sampler, f.uv), f);
}

@fragment fn fs_plain(f: Out) -> @location(0) vec4<f32> {
    return textureSample(picture, tile_sampler, f.uv);
}

@vertex fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return SkyOut(vec4<f32>(corner, 0.0, 1.0), corner);
}

fn sky_seen(direction: vec3<f32>, cover: vec4<f32>) -> vec3<f32> {
    let up = dot(direction, scene.zenith.xyz);
    let reach = min(select(to_space(up), -2.0 * PLANET_M * up + 20000.0, up < 0.0), 60000.0);
    return fogged(sky(direction) * cover.a + cover.rgb, direction * reach);
}

@fragment fn fs_sky(f: SkyOut) -> Frag {
    if (LOW_TIER) {
        let uv = screen_uv(f.screen);
        return Frag(vec4<f32>(soft(sky_map, uv).rgb, 1.0), vec4<f32>(0.0, 0.0, 0.0, soft(cloud_map, uv).a - 1.0));
    }
    let through = scene.unproject * vec4<f32>(f.screen, 0.5, 1.0);
    let direction = normalize(through.xyz / through.w);
    let cover = cloud_light(direction, dither(f.position.xy), 1e9);
    return Frag(vec4<f32>(sky_seen(direction, cover), 1.0), vec4<f32>(0.0, 0.0, 0.0, cover.a - 1.0));
}

@fragment fn fs_clouds(f: SkyOut) -> CloudOut {
    let through = scene.unproject * vec4<f32>(f.screen, 0.5, 1.0);
    let direction = normalize(through.xyz / through.w);
    let cover = cloud_light(direction, dither(f.position.xy), 1e9);
    return CloudOut(cover, vec4<f32>(sky_seen(direction, cover), 1.0));
}

@fragment fn fs_water(f: Out) -> Frag {
    let colour = textureSample(picture, tile_sampler, f.uv);
    let distance = max(length(f.seen), 1.0);
    let seen = f.seen / distance;
    let air = scatter(seen, distance, 4);
    let up = scene.zenith.xyz;
    let grazing = 1.0 - max(dot(-seen, up), 0.0);
    let fresnel = 0.02 + 0.98 * pow(grazing, 5.0);
    let mirrored = reflect(seen, up);
    let reflected = sky_air(mirrored, 2).light;
    let lit = sunlight_reaching(f.seen, up);
    let glint = pow(max(dot(mirrored, scene.sun.xyz), 0.0), 600.0) * SUN * exp(-sun_depth(0.0)) * 0.5 * lit;
    let body = colour.rgb * (0.5 * overhead_light() + 0.25 * SUN * exp(-sun_depth(0.0)) / PI * max(dot(up, scene.sun.xyz), 0.0) * lit);
    let shaded = mix(body, reflected, fresnel) + glint;
    return Frag(vec4<f32>(fogged(shaded * air.through + air.light, f.seen), 1.0), vec4<f32>(up, distance));
}

const WALL_TAG = 2.0;
const NIGHT_FROM = 0.21;
const NIGHT_AT = 0.141;
const WINDOW_LIGHT = vec3<f32>(5.0, 3.3, 1.6);
const LIT_WINDOWS = 0.45;
const GLASS = vec3<f32>(0.05, 0.055, 0.06);
const GLASS_SHEEN = 0.14;

struct BuildingOut { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) seen: vec3<f32>, @location(2) normal: vec3<f32>, @location(3) @interpolate(flat) wall: f32, @location(4) @interpolate(flat) style: vec4<f32>, @location(5) @interpolate(flat) colour: vec3<f32> };

@vertex fn vs_building(v: In, placed: Placed) -> BuildingOut {
    let mvp = mat4x4<f32>(placed.mvp0, placed.mvp1, placed.mvp2, placed.mvp3);
    let model = mat4x4<f32>(placed.model0, placed.model1, placed.model2, placed.model3);
    let tagged = v.normal.z > WALL_TAG - 0.5;
    let normal = select(v.normal, vec3<f32>(v.normal.xy, 0.0), tagged);
    let wall = select(-1.0, v.normal.z - WALL_TAG, tagged);
    let index = floor(wall);
    let style = vec4<f32>(hash(vec2<f32>(index, 1.0)), hash(vec2<f32>(index, 2.0)), hash(vec2<f32>(index, 3.0)), hash(vec2<f32>(index, 4.0)));
    let width = textureDimensions(picture).x;
    let slot = u32(max(index, 0.0));
    let colour = select(textureSampleLevel(picture, tile_sampler, v.uv, 0.0), textureLoad(picture, vec2<u32>(slot % width, slot / width), 0), tagged).rgb;
    return BuildingOut(mvp * vec4<f32>(v.position, 1.0), v.uv, (model * vec4<f32>(v.position, 1.0)).xyz, (model * vec4<f32>(normal, 0.0)).xyz, wall, style, colour);
}

fn band(x: f32, low: f32, high: f32, blur: f32) -> f32 {
    return smoothstep(low - blur, low + blur, x) * (1.0 - smoothstep(high - blur, high + blur, x));
}

fn reflected_light(direction: vec3<f32>) -> vec3<f32> {
    let up = dot(direction, scene.zenith.xyz);
    if (LOW_TIER) { return mix(GROUND_ALBEDO, AMBIENT_SKY, smoothstep(-0.05, 0.05, up)); }
    let ground = GROUND_ALBEDO * (overhead_light() + SUN * exp(-sun_depth(0.0)) / PI * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0));
    return mix(ground, sky_air(normalize(direction + scene.zenith.xyz * max(0.02 - up, 0.0)), 2).light, smoothstep(-0.05, 0.05, up));
}

struct Window { glass: f32, framed: f32, ledge: f32, fade: f32, bay: vec2<f32>, index: i32 };

fn window(f: BuildingOut, blur: vec2<f32>) -> Window {
    let index = i32(floor(f.wall));
    let style = f.style;
    let bay = vec2<f32>(floor(f.uv.x), floor(f.uv.y));
    let at = fract(f.uv);
    let ground_floor = fract(f.wall) > 0.25 && bay.y < 0.5;
    let pane = select(mix(0.34, 0.5, style.x), 0.84, ground_floor);
    let sill = select(mix(0.36, 0.42, style.y), 0.1, ground_floor);
    let head = select(mix(0.78, 0.86, style.z), 0.86, ground_floor);
    let windowed = select(1.0, 0.0, f.uv.x < -0.5 || f.uv.y < 0.0);
    if (LOW_TIER && max(blur.x, blur.y) > 0.6) { return Window(pane * (head - sill) * windowed, 0.0, 0.0, 1.0, bay, index); }
    let fade = smoothstep(0.25, 0.6, max(blur.x, blur.y));
    let sharp = band(at.x, 0.5 - pane * 0.5, 0.5 + pane * 0.5, blur.x) * band(at.y, sill, head, blur.y);
    let glass = mix(sharp, pane * (head - sill), fade) * windowed;
    if (LOW_TIER) { return Window(glass, 0.0, 0.0, fade, bay, index); }
    let rim = 0.045;
    let framed = band(at.x, 0.5 - pane * 0.5 - rim, 0.5 + pane * 0.5 + rim, blur.x) * band(at.y, sill - rim, head + rim, blur.y) * windowed * (1.0 - fade);
    let ledge = band(at.y, -0.03, 0.035, blur.y) * step(0.5, style.w) * (1.0 - fade) * select(1.0, 0.0, f.uv.y < 0.0);
    return Window(glass, framed, ledge, fade, bay, index);
}

fn facade_albedo(f: BuildingOut, blur: vec2<f32>) -> vec3<f32> {
    let pane = window(f, blur);
    let wall = f.colour;
    let frame_colour = select(wall * 0.55, vec3<f32>(0.82, 0.8, 0.76), f.style.w < 0.4);
    let glass = select(GLASS, GLASS + AMBIENT_SKY * GLASS_SHEEN * 0.6, LOW_TIER);
    return mix(mix(wall, frame_colour, max(pane.framed - pane.glass, 0.0)) * (1.0 - 0.3 * pane.ledge), glass, pane.glass);
}

fn facade_glow(f: BuildingOut, blur: vec2<f32>) -> vec3<f32> {
    let pane = window(f, blur);
    let normal = facing(f.normal, f.seen);
    let view = normalize(f.seen);
    let fresnel = GLASS_SHEEN + (1.0 - GLASS_SHEEN) * pow(1.0 - max(dot(-view, normal), 0.0), 5.0);
    let night = smoothstep(NIGHT_FROM, NIGHT_AT, dot(scene.sun.xyz, scene.zenith.xyz));
    let seed = f32(pane.index);
    let lit = step(hash(pane.bay + vec2<f32>(seed * 7.31, seed * 3.17)), LIT_WINDOWS) * (0.6 + 0.8 * hash(pane.bay.yx + seed));
    let inside = WINDOW_LIGHT * night * mix(lit, LIT_WINDOWS, pane.fade);
    return pane.glass * (reflected_light(reflect(view, normal)) * fresnel + inside) * (1.0 - fog_amount(f.seen));
}

@fragment fn fs_building(f: BuildingOut) -> Frag {
    let blur = max(fwidth(f.uv), vec2<f32>(0.002));
    let wall = f.wall >= 0.0;
    var albedo = f.colour;
    if (wall) { albedo = facade_albedo(f, blur); }
    let shaded = lit_surface(vec4<f32>(albedo, 1.0), Out(f.position, f.uv, f.seen, f.normal));
    var glow = vec3<f32>(0.0);
    let dark = smoothstep(NIGHT_FROM, NIGHT_AT, dot(scene.sun.xyz, scene.zenith.xyz)) > 0.0;
    if (wall && (!LOW_TIER || dark)) { glow = facade_glow(f, blur); }
    return Frag(vec4<f32>(shaded.colour.rgb + glow, 1.0), shaded.surface);
}
";
const SHADOW_SHADER: &str = r"
struct Cascade { light: mat4x4<f32> };
@group(0) @binding(0) var<uniform> cascade: Cascade;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32> };
struct Placed {
    @location(3) mvp0: vec4<f32>, @location(4) mvp1: vec4<f32>, @location(5) mvp2: vec4<f32>, @location(6) mvp3: vec4<f32>,
    @location(7) model0: vec4<f32>, @location(8) model1: vec4<f32>, @location(9) model2: vec4<f32>, @location(10) model3: vec4<f32>,
};

@vertex fn vs(v: In, placed: Placed) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(placed.model0, placed.model1, placed.model2, placed.model3);
    return cascade.light * model * vec4<f32>(v.position, 1.0);
}

@fragment fn fs() {}
";
const POST_SHADER: &str = r"
struct Scene { unproject: mat4x4<f32>, project: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32>, light: array<mat4x4<f32>, 3>, cascades: array<vec4<f32>, 3>, eye: vec4<f32>, previous: mat4x4<f32>, temporal: vec4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var screen_sampler: sampler;
@group(1) @binding(1) var picture: texture_2d<f32>;
@group(1) @binding(2) var second: texture_2d<f32>;
@group(1) @binding(3) var third: texture_2d<f32>;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };

const EXPOSURE = 0.36;
const BLOOM = 0.06;
const GOD_RAYS = 0.2;
const RAY_SAMPLES = 40;
const RAY_DECAY = 0.96;
const RAY_REACH = 0.9;
const RAY_SPREAD = 12.0;
const BLOOM_THRESHOLD = 1.0 / EXPOSURE;
const BLOOM_KNEE = 0.5;
const AO_RADIUS = 6.0;
const AO_BIAS = 0.05;
const AO_STRENGTH = 1.5;
const AO_SAMPLES = 12;
const KERNEL = array<vec3<f32>, 12>(
    vec3<f32>(-0.088, 0.007, 0.047), vec3<f32>(-0.022, 0.102, 0.020), vec3<f32>(-0.061, -0.103, 0.036), vec3<f32>(0.060, -0.055, 0.133),
    vec3<f32>(-0.038, -0.098, 0.170), vec3<f32>(-0.164, 0.048, 0.191), vec3<f32>(-0.314, -0.010, 0.083), vec3<f32>(-0.178, 0.187, 0.314),
    vec3<f32>(-0.067, 0.422, 0.259), vec3<f32>(-0.301, -0.159, 0.502), vec3<f32>(-0.078, -0.675, 0.254), vec3<f32>(0.467, -0.645, 0.314),
);
const CLIP_SIGMA = 1.0;
const BLUR_TAPS = 8;
const BLUR_REACH_PX = 16.0;

@vertex fn vs(@builtin(vertex_index) index: u32) -> Out {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Out(vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0), vec2<f32>(corner.x, 1.0 - corner.y));
}

const ACES_IN = mat3x3<f32>(0.59719, 0.07600, 0.02840, 0.35458, 0.90834, 0.13383, 0.04823, 0.01566, 0.83777);
const ACES_OUT = mat3x3<f32>(1.60475, -0.10208, -0.00327, -0.53108, 1.10813, -0.07276, -0.07367, -0.00605, 1.07602);

fn aces(colour: vec3<f32>) -> vec3<f32> {
    let v = ACES_IN * colour;
    let fitted = (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
    return clamp(ACES_OUT * fitted, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn tap(uv: vec2<f32>, texel: vec2<f32>, x: f32, y: f32) -> vec3<f32> {
    return textureSampleLevel(picture, screen_sampler, uv + texel * vec2<f32>(x, y), 0.0).rgb;
}

fn down13(uv: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(picture));
    let corners = tap(uv, texel, -2.0, -2.0) + tap(uv, texel, 2.0, -2.0) + tap(uv, texel, -2.0, 2.0) + tap(uv, texel, 2.0, 2.0);
    let edges = tap(uv, texel, 0.0, -2.0) + tap(uv, texel, -2.0, 0.0) + tap(uv, texel, 2.0, 0.0) + tap(uv, texel, 0.0, 2.0);
    let inner = tap(uv, texel, -1.0, -1.0) + tap(uv, texel, 1.0, -1.0) + tap(uv, texel, -1.0, 1.0) + tap(uv, texel, 1.0, 1.0);
    return tap(uv, texel, 0.0, 0.0) * 0.125 + corners * 0.03125 + edges * 0.0625 + inner * 0.125;
}

fn bright(colour: vec3<f32>) -> vec3<f32> {
    let level = max(colour.r, max(colour.g, colour.b));
    let soft = clamp(level - BLOOM_THRESHOLD + BLOOM_KNEE, 0.0, 2.0 * BLOOM_KNEE);
    let knee = soft * soft / (4.0 * BLOOM_KNEE + 0.0001);
    return colour * max(knee, level - BLOOM_THRESHOLD) / max(level, 0.0001);
}

@fragment fn fs_prefilter(f: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(bright(down13(f.uv)), 1.0);
}

@fragment fn fs_down(f: Out) -> @location(0) vec4<f32> {
    return vec4<f32>(down13(f.uv), 1.0);
}

@fragment fn fs_up(f: Out) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(picture));
    let corners = tap(f.uv, texel, -1.0, -1.0) + tap(f.uv, texel, 1.0, -1.0) + tap(f.uv, texel, -1.0, 1.0) + tap(f.uv, texel, 1.0, 1.0);
    let edges = tap(f.uv, texel, 0.0, -1.0) + tap(f.uv, texel, -1.0, 0.0) + tap(f.uv, texel, 1.0, 0.0) + tap(f.uv, texel, 0.0, 1.0);
    return vec4<f32>((tap(f.uv, texel, 0.0, 0.0) * 4.0 + edges * 2.0 + corners) / 16.0, 1.0);
}

fn ray(uv: vec2<f32>) -> vec3<f32> {
    let through = scene.unproject * vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.5, 1.0);
    return normalize(through.xyz / through.w);
}

fn noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

fn surface_at(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(picture, screen_sampler, uv, 0.0);
}

@fragment fn fs_ssao(f: Out) -> @location(0) vec4<f32> {
    let surface = surface_at(f.uv);
    let depth = surface.w;
    if (depth <= 0.0 || dot(surface.xyz, surface.xyz) < 0.25) { return vec4<f32>(0.0); }
    let normal = normalize(surface.xyz);
    let position = ray(f.uv) * depth;
    let angle = noise(f.position.xy) * 6.2831853;
    let random = vec3<f32>(cos(angle), sin(angle), 0.37);
    let tangent = normalize(random - normal * dot(random, normal));
    let bitangent = cross(normal, tangent);
    var kernel = KERNEL;
    var occlusion = 0.0;
    for (var i = 0; i < AO_SAMPLES; i++) {
        let k = kernel[i];
        let sample = position + (tangent * k.x + bitangent * k.y + normal * k.z) * AO_RADIUS;
        let clip = scene.project * vec4<f32>(sample, 1.0);
        if (clip.w <= 0.0) { continue; }
        let ndc = clip.xy / clip.w;
        let seen = surface_at(vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5)).w;
        if (seen <= 0.0) { continue; }
        let range = smoothstep(0.0, 1.0, AO_RADIUS / abs(depth - seen));
        occlusion += select(0.0, range, seen < length(sample) - AO_BIAS);
    }
    return vec4<f32>(occlusion / f32(AO_SAMPLES));
}

@fragment fn fs_rays(f: Out) -> @location(0) vec4<f32> {
    let clip = scene.project * vec4<f32>(scene.sun.xyz * 100000.0, 1.0);
    if (clip.w <= 0.0) { return vec4<f32>(0.0); }
    let sun_uv = vec2<f32>(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5);
    let outside = max(abs(sun_uv - 0.5) - 0.5, vec2<f32>(0.0));
    let edge = 1.0 - smoothstep(0.0, 0.3, length(outside));
    if (edge <= 0.0) { return vec4<f32>(0.0); }
    let towards = (sun_uv - f.uv) * RAY_REACH / f32(RAY_SAMPLES);
    var uv = f.uv + towards * noise(f.position.xy);
    var weight = 1.0;
    var total = vec3<f32>(0.0);
    for (var i = 0; i < RAY_SAMPLES; i++) {
        uv += towards;
        if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) { break; }
        let seen = textureSampleLevel(second, screen_sampler, uv, 0.0).w;
        let sky = select(0.0, 1.0 + seen, seen <= 0.0);
        let glow = pow(max(dot(ray(uv), scene.sun.xyz), 0.0), RAY_SPREAD);
        total += textureSampleLevel(picture, screen_sampler, uv, 0.0).rgb * sky * glow * weight;
        weight *= RAY_DECAY;
    }
    return vec4<f32>(total / f32(RAY_SAMPLES) * edge * GOD_RAYS / BLOOM, 1.0);
}

@fragment fn fs_blur(f: Out) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(picture));
    let depth = textureSampleLevel(second, screen_sampler, f.uv, 0.0).w;
    var total = 0.0;
    var weight = 0.0;
    for (var y = -2; y < 2; y++) {
        for (var x = -2; x < 2; x++) {
            let at = f.uv + (vec2<f32>(f32(x), f32(y)) + 0.5) * texel;
            let near = textureSampleLevel(second, screen_sampler, at, 0.0).w;
            let w = select(0.0, 1.0, abs(near - depth) < AO_RADIUS);
            total += textureSampleLevel(picture, screen_sampler, at, 0.0).r * w;
            weight += w;
        }
    }
    return vec4<f32>(total / max(weight, 1.0));
}

@fragment fn fs_shown(f: Out) -> @location(0) vec4<f32> {
    let glow = textureSampleLevel(second, screen_sampler, f.uv, 0.0).rgb * BLOOM;
    let shade = max(1.0 - textureSampleLevel(third, screen_sampler, f.uv, 0.0).r * AO_STRENGTH, 0.0);
    return vec4<f32>(aces((textureSampleLevel(picture, screen_sampler, f.uv, 0.0).rgb * shade + glow) * EXPOSURE), 1.0);
}

fn velocity(uv: vec2<f32>, surface: vec4<f32>) -> vec2<f32> {
    let direction = ray(uv);
    let seen = select(vec4<f32>(direction, 0.0), vec4<f32>(direction * surface.w, 1.0), surface.w > 0.0);
    let clip = scene.previous * seen;
    if (clip.w <= 0.0) { return vec2<f32>(2.0); }
    let before = vec2<f32>(clip.x / clip.w * 0.5 + 0.5, 0.5 - clip.y / clip.w * 0.5);
    return uv - scene.temporal.xy - before;
}

fn ycocg(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(0.25 * c.r + 0.5 * c.g + 0.25 * c.b, 0.5 * c.r - 0.5 * c.b, -0.25 * c.r + 0.5 * c.g - 0.25 * c.b);
}

fn rgb(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(c.x + c.y - c.z, c.x + c.z, c.x - c.y - c.z);
}

fn history_at(at: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(second, screen_sampler, at, 0.0).rgb;
}

fn catmull_rom(uv: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let sample = uv * size - 0.5;
    let centre = floor(sample) + 0.5;
    let t = sample - centre + 0.5;
    let w0 = t * (-0.5 + t * (1.0 - 0.5 * t));
    let w1 = 1.0 + t * t * (-2.5 + 1.5 * t);
    let w2 = t * (0.5 + t * (2.0 - 1.5 * t));
    let w3 = t * t * (-0.5 + 0.5 * t);
    let w12 = w1 + w2;
    let offset = w2 / w12;
    let p0 = (centre - 1.0) / size;
    let p3 = (centre + 2.0) / size;
    let p12 = (centre + offset) / size;
    var total = history_at(vec2<f32>(p12.x, p0.y)) * w12.x * w0.y;
    total += history_at(vec2<f32>(p0.x, p12.y)) * w0.x * w12.y;
    total += history_at(p12) * w12.x * w12.y;
    total += history_at(vec2<f32>(p3.x, p12.y)) * w3.x * w12.y;
    total += history_at(vec2<f32>(p12.x, p3.y)) * w12.x * w3.y;
    return max(total, vec3<f32>(0.0));
}

@fragment fn fs_taa(f: Out) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(picture));
    let texel = 1.0 / size;
    let current = textureSampleLevel(picture, screen_sampler, f.uv, 0.0).rgb;
    let moved = velocity(f.uv, textureSampleLevel(third, screen_sampler, f.uv, 0.0));
    let before = f.uv - moved;
    var mean = vec3<f32>(0.0);
    var square = vec3<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let c = ycocg(textureSampleLevel(picture, screen_sampler, f.uv + vec2<f32>(f32(x), f32(y)) * texel, 0.0).rgb);
            mean += c;
            square += c * c;
        }
    }
    mean /= 9.0;
    let sigma = sqrt(max(square / 9.0 - mean * mean, vec3<f32>(0.0)));
    let low = mean - sigma * CLIP_SIGMA;
    let high = mean + sigma * CLIP_SIGMA;
    let history = clamp(ycocg(catmull_rom(before, size)), low, high);
    let pixels = length(moved * size);
    let inside = f32(all(before >= vec2<f32>(0.0)) && all(before <= vec2<f32>(1.0)));
    let weight = scene.temporal.z * inside * (1.0 - 0.5 * smoothstep(0.0, 32.0, pixels));
    return vec4<f32>(mix(current, rgb(history), weight), 1.0);
}

@fragment fn fs(f: Out) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(picture));
    let moved = velocity(f.uv, textureSampleLevel(second, screen_sampler, f.uv, 0.0)) * scene.temporal.w;
    let pixels = length(moved * size);
    let colour = textureSampleLevel(picture, screen_sampler, f.uv, 0.0).rgb;
    if (pixels < 0.5 || pixels > 1000.0) { return vec4<f32>(colour, 1.0); }
    let along = moved * min(1.0, BLUR_REACH_PX / pixels);
    let start = noise(f.position.xy) - 0.5;
    var total = vec3<f32>(0.0);
    for (var i = 0; i < BLUR_TAPS; i++) {
        let t = (f32(i) + start) / f32(BLUR_TAPS) - 0.5;
        total += textureSampleLevel(picture, screen_sampler, f.uv + along * t, 0.0).rgb;
    }
    return vec4<f32>(total / f32(BLUR_TAPS), 1.0);
}
";

pub struct Resident<T> {
    entries: HashMap<u64, (Weak<Mesh>, T)>,
}

impl<T> Default for Resident<T> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<T> Resident<T> {
    pub fn ensure(&mut self, meshes: &[Arc<Mesh>], mut upload: impl FnMut(&Mesh) -> T) -> usize {
        let missing: Vec<&Arc<Mesh>> = meshes
            .iter()
            .filter(|mesh| !self.entries.contains_key(&mesh.id))
            .collect();
        let uploaded = missing.len();
        missing.into_iter().for_each(|mesh| {
            let resident = upload(mesh);
            self.entries
                .insert(mesh.id, (Arc::downgrade(mesh), resident));
        });
        uploaded
    }

    pub fn evict(&mut self) -> Vec<T> {
        self.entries
            .extract_if(|_, (mesh, _)| mesh.strong_count() == 0)
            .map(|(_, (_, resident))| resident)
            .collect()
    }

    pub fn get(&self, id: u64) -> Option<&T> {
        self.entries.get(&id).map(|(_, resident)| resident)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

struct Arena<B> {
    vertex_space: RangeAllocator<u32>,
    index_space: RangeAllocator<u32>,
    buffers: B,
}

struct ArenaBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

#[derive(Clone, Debug, PartialEq)]
struct Place {
    arena: usize,
    vertices: Range<u32>,
    indices: Range<u32>,
}

fn place_in<B>(
    arenas: &mut Vec<Option<Arena<B>>>,
    vertices: u32,
    indices: u32,
    open: impl FnOnce(u32, u32) -> B,
) -> Option<Place> {
    let fitted = arenas.iter_mut().enumerate().find_map(|(arena, slot)| {
        let space = slot.as_mut()?;
        let taken = space.vertex_space.allocate_range(vertices).ok()?;
        match space.index_space.allocate_range(indices) {
            Ok(indices) => Some(Place {
                arena,
                vertices: taken,
                indices,
            }),
            Err(_) => {
                space.vertex_space.free_range(taken);
                None
            }
        }
    });
    fitted.or_else(|| {
        let arena = arenas
            .iter()
            .position(Option::is_none)
            .unwrap_or(arenas.len());
        let (vertex_capacity, index_capacity) =
            (vertices.max(ARENA_VERTICES), indices.max(ARENA_INDICES));
        let opened = Some(Arena {
            vertex_space: RangeAllocator::new(0..vertex_capacity),
            index_space: RangeAllocator::new(0..index_capacity),
            buffers: open(vertex_capacity, index_capacity),
        });
        match arenas.get_mut(arena) {
            Some(slot) => *slot = opened,
            None => arenas.push(opened),
        }
        let space = arenas.get_mut(arena)?.as_mut()?;
        Some(Place {
            arena,
            vertices: space.vertex_space.allocate_range(vertices).ok()?,
            indices: space.index_space.allocate_range(indices).ok()?,
        })
    })
}

fn release<B>(arenas: &mut [Option<Arena<B>>], place: Place) {
    if let Some(space) = arenas.get_mut(place.arena).and_then(Option::as_mut) {
        space.vertex_space.free_range(place.vertices);
        space.index_space.free_range(place.indices);
    }
}

fn close_empty<B>(arenas: &mut [Option<Arena<B>>]) {
    arenas
        .iter_mut()
        .skip(1)
        .filter(|slot| {
            slot.as_ref()
                .is_some_and(|arena| arena.vertex_space.is_empty() && arena.index_space.is_empty())
        })
        .for_each(|slot| *slot = None);
}

struct GpuMesh {
    place: Option<Place>,
    bind: wgpu::BindGroup,
}

fn record(
    pass: &mut wgpu::RenderPass,
    arenas: &[Option<Arena<ArenaBuffers>>],
    bound: &Cell<Option<usize>>,
    gpu: &GpuMesh,
    instances: Range<u32>,
) {
    let Some((place, arena)) = gpu
        .place
        .as_ref()
        .and_then(|place| Some((place, arenas.get(place.arena)?.as_ref()?)))
    else {
        return;
    };
    if bound.get() != Some(place.arena) {
        pass.set_vertex_buffer(0, arena.buffers.vertices.slice(..));
        pass.set_index_buffer(arena.buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        bound.set(Some(place.arena));
    }
    pass.set_bind_group(1, &gpu.bind, &[]);
    pass.draw_indexed(place.indices.clone(), 0, instances);
}

struct Draws {
    buffer: wgpu::Buffer,
    slots: usize,
}

fn draw_slots(needed: usize) -> usize {
    needed.max(FIRST_DRAW_SLOTS).next_power_of_two()
}

pub struct Targets {
    shadow_layers: Vec<wgpu::TextureView>,
    shadow_bind: wgpu::BindGroup,
    cloud_map: wgpu::TextureView,
    sky_map: wgpu::TextureView,
    samples: Option<(wgpu::TextureView, wgpu::TextureView)>,
    depth: wgpu::TextureView,
    resolved: wgpu::TextureView,
    surface: wgpu::TextureView,
    occlusion: wgpu::TextureView,
    blurred: wgpu::TextureView,
    ssao_bind: wgpu::BindGroup,
    blur_bind: wgpu::BindGroup,
    mips: Vec<wgpu::TextureView>,
    down_binds: Vec<wgpu::BindGroup>,
    up_binds: Vec<wgpu::BindGroup>,
    post_bind: wgpu::BindGroup,
    rays_bind: wgpu::BindGroup,
    history: [wgpu::TextureView; 2],
    taa_binds: [wgpu::BindGroup; 2],
    motion_binds: [wgpu::BindGroup; 2],
    tonemapped: wgpu::TextureView,
    frame: Cell<usize>,
    previous: Cell<Option<(glam::DMat4, glam::DVec3)>>,
    height_px: u32,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: [wgpu::RenderPipeline; LAYERS.len()],
    backfill: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    post: wgpu::RenderPipeline,
    shown: wgpu::RenderPipeline,
    taa: wgpu::RenderPipeline,
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    ssao: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    rays: wgpu::RenderPipeline,
    clouds: wgpu::RenderPipeline,
    noise_bind: wgpu::BindGroup,
    quality: Quality,
    sky_light: wgpu::TextureView,
    sky_light_pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    screen_layout: wgpu::BindGroupLayout,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    cascades: Vec<(wgpu::Buffer, wgpu::BindGroup)>,
    draws: Option<Draws>,
    scene: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    post_sampler: wgpu::Sampler,
    resident: Resident<GpuMesh>,
    arenas: Vec<Option<Arena<ArenaBuffers>>>,
    pub uploads: usize,
    to_upload: Vec<Weak<Mesh>>,
    started: Instant,
}

fn uniform(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn halton(index: usize, base: usize) -> f64 {
    std::iter::successors(Some(index), |i| (i / base > 0).then_some(i / base))
        .zip(std::iter::successors(Some(1.0 / base as f64), |f| {
            Some(f / base as f64)
        }))
        .map(|(i, f)| (i % base) as f64 * f)
        .sum()
}

fn jitter(frame: usize, camera: &Camera, height_px: u32) -> glam::DVec2 {
    let index = frame % JITTERS + 1;
    let height = f64::from(height_px.max(1));
    glam::DVec2::new(
        (halton(index, 2) - 0.5) * 2.0 / (camera.aspect * height),
        (halton(index, 3) - 0.5) * 2.0 / height,
    )
}

fn bytes(values: impl IntoIterator<Item = f32>) -> Vec<u8> {
    bytemuck::cast_slice(&values.into_iter().collect::<Vec<f32>>()).to_vec()
}

fn depth_state(write: bool, compare: wgpu::CompareFunction) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH,
        depth_write_enabled: Some(write),
        depth_compare: Some(compare),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn vertex_buffers() -> [Option<wgpu::VertexBufferLayout<'static>>; 2] {
    [
        Some(wgpu::VertexBufferLayout {
            array_stride: VERTEX_BYTES,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: DRAW_BYTES,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &DRAW_ATTRIBUTES,
        }),
    ]
}

struct Cascade {
    light: glam::DMat4,
    view: glam::DMat4,
    low: glam::DVec2,
    high: glam::DVec2,
    close: f64,
    deep: f64,
    split: f64,
    texel: f64,
    bias: f64,
}

fn cascade(camera: &Camera, near: f64, far: f64, size: u32) -> Cascade {
    let tan_h = (camera.fovy / 2.0).tan();
    let tan_w = tan_h * camera.aspect;
    let along = (0.5 * (near + far) * (1.0 + tan_h * tan_h + tan_w * tan_w)).min(far);
    let corner =
        |depth: f64| glam::DVec3::new(depth * tan_w, depth * tan_h, depth - along).length();
    let radius = corner(near).max(corner(far));
    let side = if camera.sun.dot(camera.zenith).abs() > 0.99 {
        camera.forward
    } else {
        camera.zenith
    };
    let view = glam::dcamera::rh::view::look_to_mat4(glam::DVec3::ZERO, -camera.sun, side);
    let texel = 2.0 * radius / f64::from(size);
    let at = view.transform_point3(camera.forward * along);
    let snapped = (at.truncate() / texel).round() * texel;
    let (low, high) = (snapped - radius, snapped + radius);
    let (close, deep) = (-at.z - radius - CASTER_REACH_M, -at.z + radius);
    let light =
        glam::dcamera::rh::proj::directx::orthographic(low.x, high.x, low.y, high.y, close, deep)
            * view;
    Cascade {
        light,
        view,
        low,
        high,
        close,
        deep,
        split: far,
        texel,
        bias: (1.5 * texel).max(0.5) / (deep - close),
    }
}

fn cascades(camera: &Camera, size: u32) -> Vec<Cascade> {
    CASCADE_FAR_M
        .iter()
        .scan(0.0, |near, far| {
            let fitted = cascade(camera, *near, *far, size);
            *near = *far;
            Some(fitted)
        })
        .collect()
}

fn casts(cascade: &Cascade, model: glam::DMat4, mesh: &Mesh) -> bool {
    let to_light = cascade.view * model;
    let (low, high) = (0..8)
        .map(|corner| {
            let pick = |axis: usize| {
                f64::from(if corner >> axis & 1 == 0 {
                    mesh.low[axis]
                } else {
                    mesh.high[axis]
                })
            };
            to_light.transform_point3(glam::DVec3::new(pick(0), pick(1), pick(2)))
        })
        .fold((glam::DVec3::MAX, glam::DVec3::MIN), |(low, high), p| {
            (low.min(p), high.max(p))
        });
    low.x <= cascade.high.x
        && high.x >= cascade.low.x
        && low.y <= cascade.high.y
        && high.y >= cascade.low.y
        && -high.z <= cascade.deep
        && -low.z >= cascade.close
}

fn shadow_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic shadow"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &vertex_buffers(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
        }),
        multisample: multisample(1),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn multisample(count: u32) -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count,
        ..Default::default()
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color: wgpu::TextureFormat,
    layer: Layer,
    samples: u32,
) -> wgpu::RenderPipeline {
    let (depth_write, compare, blend, fragment) = match layer {
        Layer::Backfill => (false, wgpu::CompareFunction::Always, None, "fs"),
        Layer::Opaque | Layer::Foliage => (true, wgpu::CompareFunction::Greater, None, "fs"),
        Layer::Water => (true, wgpu::CompareFunction::Greater, None, "fs_water"),
        Layer::Building => (true, wgpu::CompareFunction::Greater, None, "fs_building"),
        Layer::Translucent => (
            false,
            wgpu::CompareFunction::Greater,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "fs",
        ),
        Layer::OnTop => (
            false,
            wgpu::CompareFunction::Always,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            "fs_plain",
        ),
    };
    let targets = [
        Some(wgpu::ColorTargetState {
            format: color,
            blend,
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(wgpu::ColorTargetState {
            format: SURFACE,
            blend: None,
            write_mask: if layer == Layer::Translucent {
                wgpu::ColorWrites::empty()
            } else {
                wgpu::ColorWrites::ALL
            },
        }),
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if layer == Layer::Building {
                "vs_building"
            } else {
                "vs"
            }),
            compilation_options: Default::default(),
            buffers: &vertex_buffers(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: (layer != Layer::OnTop).then(|| depth_state(depth_write, compare)),
        multisample: multisample(if layer == Layer::OnTop { 1 } else { samples }),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment),
            compilation_options: Default::default(),
            targets: if layer == Layer::OnTop {
                &targets[..1]
            } else {
                &targets
            },
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn sky_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    color: wgpu::TextureFormat,
    samples: u32,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic sky"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_sky"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: Some(depth_state(false, wgpu::CompareFunction::Always)),
        multisample: multisample(samples),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_sky"),
            compilation_options: Default::default(),
            targets: &[
                Some(wgpu::ColorTargetState {
                    format: color,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
                Some(wgpu::ColorTargetState {
                    format: SURFACE,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }),
            ],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn sky_pass_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry: &str,
    outputs: usize,
) -> wgpu::RenderPipeline {
    let target = Some(wgpu::ColorTargetState {
        format: HDR,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_sky"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: multisample(1),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &vec![target; outputs],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn screen_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry: &str,
    color: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: multisample(1),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: color,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn filtering(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn noise_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: wgpu::Extent3d,
    format: wgpu::TextureFormat,
    texel_bytes: u32,
    bytes: &[u8],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("synthetic cloud noise"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: if size.depth_or_array_layers > 1 {
            wgpu::TextureDimension::D3
        } else {
            wgpu::TextureDimension::D2
        },
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(size.width * texel_bytes),
            rows_per_image: Some(size.height),
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(if size.depth_or_array_layers > 1 {
            wgpu::TextureViewDimension::D3
        } else {
            wgpu::TextureViewDimension::D2
        }),
        ..Default::default()
    })
}

fn noise_bind(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    quality: Quality,
    sky_light: &wgpu::TextureView,
) -> wgpu::BindGroup {
    let extent = |side: u32, depth: u32| wgpu::Extent3d {
        width: side,
        height: side,
        depth_or_array_layers: depth,
    };
    let (shape, detail) = if quality.low {
        (
            noise_texture(
                device,
                queue,
                extent(noise::SHAPE_SIZE, 1),
                wgpu::TextureFormat::Rgba8Unorm,
                4,
                &noise::cloud_shape(),
            ),
            noise_texture(
                device,
                queue,
                extent(noise::DETAIL_SIZE, noise::DETAIL_DEPTH),
                wgpu::TextureFormat::R8Unorm,
                1,
                &noise::cloud_detail(),
            ),
        )
    } else {
        (
            noise_texture(
                device,
                queue,
                extent(1, 1),
                wgpu::TextureFormat::Rgba8Unorm,
                4,
                &[0; 4],
            ),
            noise_texture(
                device,
                queue,
                extent(1, 2),
                wgpu::TextureFormat::R8Unorm,
                1,
                &[0; 2],
            ),
        )
    };
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic cloud noise"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&shape),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&detail),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(sky_light),
            },
        ],
    })
}

fn sampled(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::REPLACE,
};

impl Renderer {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| format!("no GPU adapter: {error}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|error| format!("the GPU refused a device: {error}"))?;
        Ok(Self::with_device(device, queue, COLOR))
    }

    pub fn with_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let quality = quality(&device.adapter_info());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic"),
            source: wgpu::ShaderSource::Wgsl(scene_shader(quality).into()),
        });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic scene"),
            entries: &[uniform(0, wgpu::ShaderStages::FRAGMENT)],
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic shadow"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                sampled(2),
                filtering(3),
                sampled(4),
            ],
        });
        let noise_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic cloud noise"),
            entries: &[
                sampled(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                filtering(2),
                sampled(3),
            ],
        });
        let cascade_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic cascade"),
            entries: &[uniform(0, wgpu::ShaderStages::VERTEX)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic"),
            bind_group_layouts: &[
                Some(&scene_layout),
                Some(&layout),
                Some(&shadow_layout),
                Some(&noise_layout),
            ],
            immediate_size: 0,
        });
        let plain_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic plain"),
            bind_group_layouts: &[Some(&scene_layout), Some(&layout)],
            immediate_size: 0,
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("synthetic shadow"),
                bind_group_layouts: &[Some(&cascade_layout), Some(&layout)],
                immediate_size: 0,
            });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic shadow"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });
        let shadow = shadow_pipeline(&device, &shadow_pipeline_layout, &shadow_shader);
        let cascades = (0..CASCADES)
            .map(|_| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("synthetic cascade"),
                    size: 64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("synthetic cascade"),
                    layout: &cascade_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffer.as_entire_binding(),
                    }],
                });
                (buffer, bind)
            })
            .collect();
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic sky"),
            bind_group_layouts: &[
                Some(&scene_layout),
                None,
                Some(&shadow_layout),
                Some(&noise_layout),
            ],
            immediate_size: 0,
        });
        let clouds_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic clouds"),
            bind_group_layouts: &[Some(&scene_layout), None, None, Some(&noise_layout)],
            immediate_size: 0,
        });
        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic post"),
            source: wgpu::ShaderSource::Wgsl(POST_SHADER.into()),
        });
        let screen_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic screen"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                sampled(1),
                sampled(2),
                sampled(3),
            ],
        });
        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic post"),
            bind_group_layouts: &[Some(&scene_layout), Some(&screen_layout)],
            immediate_size: 0,
        });
        let pipelines = LAYERS.map(|layer| {
            let (color, layout) = if layer == Layer::OnTop {
                (format, &plain_layout)
            } else {
                (HDR, &pipeline_layout)
            };
            pipeline(&device, layout, &shader, color, layer, quality.samples)
        });
        let backfill = pipeline(
            &device,
            &pipeline_layout,
            &shader,
            HDR,
            Layer::Backfill,
            quality.samples,
        );
        let sky = sky_pipeline(&device, &sky_layout, &shader, HDR, quality.samples);
        let screen = |entry: &str, color: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            screen_pipeline(&device, &post_layout, &post_shader, entry, color, blend)
        };
        let clouds = sky_pass_pipeline(&device, &clouds_layout, &shader, "fs_clouds", 2);
        let post = screen("fs", format, None);
        let shown = screen("fs_shown", HDR, None);
        let taa = screen("fs_taa", HDR, None);
        let prefilter = screen("fs_prefilter", HDR, None);
        let down = screen("fs_down", HDR, None);
        let up = screen("fs_up", HDR, Some(ADDITIVE));
        let ssao = screen("fs_ssao", OCCLUSION, None);
        let blur = screen("fs_blur", OCCLUSION, None);
        let rays = screen("fs_rays", HDR, Some(ADDITIVE));
        let scene = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic scene"),
            size: SCENE_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic scene"),
            layout: &scene_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: scene.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            anisotropy_clamp: ANISOTROPY,
            ..Default::default()
        });
        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let sky_light = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("synthetic sky light"),
                size: wgpu::Extent3d {
                    width: SKY_LIGHT_WIDTH,
                    height: 3,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let noise_bind = noise_bind(&device, &queue, &noise_layout, quality, &sky_light);
        let sky_light_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic sky light"),
            bind_group_layouts: &[Some(&scene_layout)],
            immediate_size: 0,
        });
        let sky_light_pipeline =
            sky_pass_pipeline(&device, &sky_light_layout, &shader, "fs_sky_light", 1);
        Self {
            device,
            queue,
            pipelines,
            backfill,
            shadow,
            sky,
            post,
            shown,
            taa,
            prefilter,
            down,
            up,
            ssao,
            blur,
            rays,
            clouds,
            noise_bind,
            quality,
            sky_light,
            sky_light_pipeline,
            layout,
            screen_layout,
            shadow_layout,
            shadow_sampler,
            cascades,
            draws: None,
            scene,
            scene_bind,
            sampler,
            post_sampler,
            resident: Resident::default(),
            arenas: Vec::new(),
            uploads: 0,
            to_upload: Vec::new(),
            started: Instant::now(),
        }
    }

    fn upload(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        arenas: &mut Vec<Option<Arena<ArenaBuffers>>>,
        mesh: &Mesh,
    ) -> GpuMesh {
        let normals = mesh
            .normals
            .iter()
            .copied()
            .chain(std::iter::repeat([0.0; 3]));
        let vertices: Vec<u8> = bytes(
            mesh.positions
                .iter()
                .zip(&mesh.uvs)
                .zip(normals)
                .flat_map(|((p, uv), n)| p.iter().chain(uv).copied().chain(n)),
        );
        let levels = mesh
            .take_texture()
            .unwrap_or_else(|| vec![solid(MISSING_TEXTURE)]);
        let (width, height) = levels[0].dimensions();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        levels.iter().enumerate().for_each(|(level, image)| {
            let (w, h) = image.dimensions();
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                image.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        let (vertex_count, index_count) = (
            u32::try_from(vertices.len() as u64 / VERTEX_BYTES).unwrap_or(0),
            u32::try_from(mesh.indices.len()).unwrap_or(0),
        );
        let opened = |vertices: u32, indices: u32| ArenaBuffers {
            vertices: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic vertices"),
                size: u64::from(vertices) * VERTEX_BYTES,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            indices: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("synthetic indices"),
                size: u64::from(indices) * INDEX_BYTES,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        };
        let place = (vertex_count > 0 && index_count > 0)
            .then(|| place_in(arenas, vertex_count, index_count, opened))
            .flatten();
        if let Some((place, arena)) = place
            .as_ref()
            .and_then(|place| Some((place, arenas.get(place.arena)?.as_ref()?)))
        {
            let rebased: Vec<u32> = mesh
                .indices
                .iter()
                .map(|index| index + place.vertices.start)
                .collect();
            queue.write_buffer(
                &arena.buffers.vertices,
                u64::from(place.vertices.start) * VERTEX_BYTES,
                &vertices,
            );
            queue.write_buffer(
                &arena.buffers.indices,
                u64::from(place.indices.start) * INDEX_BYTES,
                bytemuck::cast_slice(&rebased),
            );
        }
        GpuMesh { place, bind }
    }

    pub fn woods(&self) -> Woods {
        self.quality.woods
    }

    pub fn resident(&self) -> usize {
        self.resident.len()
    }

    fn pass<'a>(
        encoder: &'a mut wgpu::CommandEncoder,
        targets: &'a Targets,
    ) -> wgpu::RenderPass<'a> {
        let attach = |resolved: &'a wgpu::TextureView,
                      samples: Option<&'a wgpu::TextureView>,
                      clear: wgpu::Color| {
            Some(wgpu::RenderPassColorAttachment {
                view: samples.unwrap_or(resolved),
                depth_slice: None,
                resolve_target: samples.map(|_| resolved),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: if samples.is_some() {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })
        };
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic"),
            color_attachments: &[
                attach(
                    &targets.resolved,
                    targets.samples.as_ref().map(|(colour, _)| colour),
                    SKY,
                ),
                attach(
                    &targets.surface,
                    targets.samples.as_ref().map(|(_, surface)| surface),
                    wgpu::Color::TRANSPARENT,
                ),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }

    fn finish<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        targets: &'a Targets,
    ) -> wgpu::RenderPass<'a> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic post"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(&self.post);
        pass.set_bind_group(0, &self.scene_bind, &[]);
        pass.set_bind_group(1, &targets.motion_binds[targets.frame.get() % 2], &[]);
        pass.draw(0..3, 0..1);
        pass
    }

    fn temporal(&self, encoder: &mut wgpu::CommandEncoder, targets: &Targets) {
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        let parity = targets.frame.get() % 2;
        self.screen(
            encoder,
            &targets.tonemapped,
            clear,
            &self.shown,
            &targets.post_bind,
        );
        self.screen(
            encoder,
            &targets.history[parity],
            clear,
            &self.taa,
            &targets.taa_binds[parity],
        );
    }

    fn screen(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        load: wgpu::LoadOp<wgpu::Color>,
        pipeline: &wgpu::RenderPipeline,
        bind: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic screen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.scene_bind, &[]);
        pass.set_bind_group(1, bind, &[]);
        pass.draw(0..3, 0..1);
    }

    fn effects(&self, encoder: &mut wgpu::CommandEncoder, targets: &Targets) {
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        if self.quality.ssao {
            self.screen(
                encoder,
                &targets.occlusion,
                clear,
                &self.ssao,
                &targets.ssao_bind,
            );
            self.screen(
                encoder,
                &targets.blurred,
                clear,
                &self.blur,
                &targets.blur_bind,
            );
        }
        if self.quality.bloom {
            targets
                .mips
                .iter()
                .zip(&targets.down_binds)
                .enumerate()
                .for_each(|(level, (mip, bind))| {
                    let pipeline = if level == 0 {
                        &self.prefilter
                    } else {
                        &self.down
                    };
                    self.screen(encoder, mip, clear, pipeline, bind);
                });
            targets
                .mips
                .iter()
                .zip(&targets.up_binds)
                .rev()
                .for_each(|(mip, bind)| {
                    self.screen(encoder, mip, wgpu::LoadOp::Load, &self.up, bind)
                });
        }
        if self.quality.god_rays {
            let load = if self.quality.bloom {
                wgpu::LoadOp::Load
            } else {
                clear
            };
            self.screen(
                encoder,
                &targets.mips[0],
                load,
                &self.rays,
                &targets.rays_bind,
            );
        }
    }

    pub fn warm(&mut self, arrived: Vec<Arc<Mesh>>, most: usize) -> usize {
        let queued = std::mem::take(&mut self.to_upload);
        let missing: Vec<Arc<Mesh>> = queued
            .iter()
            .filter_map(Weak::upgrade)
            .chain(arrived)
            .filter(|mesh| self.resident.get(mesh.id).is_none())
            .collect();
        let (first, rest) = missing.split_at(missing.len().min(most));
        let (device, queue, layout, sampler, arenas) = (
            &self.device,
            &self.queue,
            &self.layout,
            &self.sampler,
            &mut self.arenas,
        );
        self.uploads += self.resident.ensure(first, |mesh| {
            Self::upload(device, queue, layout, sampler, arenas, mesh)
        });
        self.to_upload = rest.iter().map(Arc::downgrade).collect();
        self.to_upload.len()
    }

    pub fn draw(
        &mut self,
        target: &wgpu::TextureView,
        targets: &Targets,
        camera: &Camera,
        backfill: &[Arc<Mesh>],
        meshes: &[Arc<Mesh>],
    ) {
        let (device, queue, layout, sampler, arenas) = (
            &self.device,
            &self.queue,
            &self.layout,
            &self.sampler,
            &mut self.arenas,
        );
        let everything: Vec<Arc<Mesh>> = backfill.iter().chain(meshes).cloned().collect();
        self.uploads += self.resident.ensure(&everything, |mesh| {
            Self::upload(device, queue, layout, sampler, arenas, mesh)
        });
        self.resident
            .evict()
            .into_iter()
            .filter_map(|gone| gone.place)
            .for_each(|place| release(&mut self.arenas, place));
        close_empty(&mut self.arenas);
        let steady = camera.projection() * camera.view();
        let offset = if TAA {
            jitter(targets.frame.get(), camera, targets.height_px)
        } else {
            glam::DVec2::ZERO
        };
        let view_projection = glam::DMat4::from_translation(offset.extend(0.0)) * steady;
        let previous = targets.previous.get().map_or(steady, |(before, eye)| {
            before * glam::DMat4::from_translation(camera.eye - eye)
        });
        let temporal = [
            offset.x as f32 * 0.5,
            offset.y as f32 * -0.5,
            if TAA && targets.previous.get().is_some() {
                HISTORY
            } else {
                0.0
            },
            if MOTION_BLUR { SHUTTER } else { 0.0 },
        ];
        targets.previous.set(Some((steady, camera.eye)));
        let layer_of = |mesh: &Mesh| {
            LAYERS
                .iter()
                .position(|layer| *layer == mesh.layer)
                .unwrap_or(LAYERS.len())
        };
        let arena_of = |mesh: &Mesh, gpu: &GpuMesh| {
            gpu.place
                .as_ref()
                .filter(|_| matches!(mesh.layer, Layer::Opaque | Layer::Building))
                .map_or(0, |place| place.arena)
        };
        let drawable: Vec<(&Arc<Mesh>, &GpuMesh)> = meshes
            .iter()
            .filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?)))
            .enumerate()
            .map(|(order, (mesh, gpu))| ((layer_of(mesh), arena_of(mesh, gpu), order), (mesh, gpu)))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        let behind: Vec<(&Arc<Mesh>, &GpuMesh)> = backfill
            .iter()
            .filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?)))
            .collect();
        let slots = behind.len() + drawable.len();
        if self.draws.as_ref().is_none_or(|draws| draws.slots < slots) {
            self.draws = Some(self.draw_buffer(draw_slots(slots)));
        }
        let Some(draws) = self.draws.as_ref() else {
            return;
        };
        let models: Vec<glam::DMat4> = behind
            .iter()
            .chain(&drawable)
            .map(|(mesh, _)| camera.relative(mesh.to_ecef))
            .collect();
        let matrices: Vec<f32> = models
            .iter()
            .flat_map(|model| {
                (view_projection * *model)
                    .as_mat4()
                    .to_cols_array()
                    .into_iter()
                    .chain(model.as_mat4().to_cols_array())
            })
            .collect();
        let cascades = cascades(camera, self.quality.shadow_size);
        if !matrices.is_empty() {
            self.queue
                .write_buffer(&draws.buffer, 0, bytemuck::cast_slice(&matrices));
        }
        let placed = |slot: usize| u32::try_from(slot).map_or(0..0, |slot| slot..slot + 1);
        let elapsed = self.started.elapsed().as_secs_f32() % 86400.0;
        let dither_phase = if TAA {
            (targets.frame.get() % JITTERS) as f32
        } else {
            0.0
        };
        self.queue.write_buffer(
            &self.scene,
            0,
            &bytes(
                view_projection
                    .inverse()
                    .as_mat4()
                    .to_cols_array()
                    .into_iter()
                    .chain(view_projection.as_mat4().to_cols_array())
                    .chain(camera.zenith.as_vec3().extend(elapsed).to_array())
                    .chain(camera.sun.as_vec3().extend(dither_phase).to_array())
                    .chain(
                        cascades
                            .iter()
                            .flat_map(|cascade| cascade.light.as_mat4().to_cols_array()),
                    )
                    .chain(cascades.iter().flat_map(|cascade| {
                        [
                            if SHADOWS { cascade.split as f32 } else { 0.0 },
                            cascade.texel as f32,
                            cascade.bias as f32,
                            0.0,
                        ]
                    }))
                    .chain(
                        camera
                            .eye
                            .as_vec3()
                            .extend(camera.above_ground as f32)
                            .to_array(),
                    )
                    .chain(previous.as_mat4().to_cols_array())
                    .chain(temporal),
            ),
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        cascades
            .iter()
            .zip(&targets.shadow_layers)
            .zip(&self.cascades)
            .enumerate()
            .filter(|_| SHADOWS)
            .for_each(|(index, ((cascade, layer), (buffer, bind)))| {
                let foliage = index < self.quality.foliage_cascades;
                self.queue
                    .write_buffer(buffer, 0, &bytes(cascade.light.as_mat4().to_cols_array()));
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("synthetic shadow"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: layer,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                pass.set_pipeline(&self.shadow);
                pass.set_bind_group(0, bind, &[]);
                pass.set_vertex_buffer(1, draws.buffer.slice(..));
                let bound = Cell::new(None);
                drawable
                    .iter()
                    .zip(&models[behind.len()..])
                    .enumerate()
                    .filter(|(_, ((mesh, _), model))| {
                        (matches!(mesh.layer, Layer::Opaque | Layer::Building)
                            || (foliage && mesh.layer == Layer::Foliage))
                            && casts(cascade, **model, mesh)
                    })
                    .for_each(|(slot, ((_, gpu), _))| {
                        record(
                            &mut pass,
                            &self.arenas,
                            &bound,
                            gpu,
                            placed(behind.len() + slot),
                        )
                    });
            });
        let layer_draws =
            |pass: &mut wgpu::RenderPass, bound: &Cell<Option<usize>>, layer: Layer| {
                drawable
                    .iter()
                    .enumerate()
                    .filter(|(_, (mesh, _))| mesh.layer == layer)
                    .for_each(|(slot, (_, gpu))| {
                        record(pass, &self.arenas, bound, gpu, placed(behind.len() + slot))
                    });
            };
        if self.quality.low {
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("synthetic sky light"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.sky_light,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });
                pass.set_pipeline(&self.sky_light_pipeline);
                pass.set_bind_group(0, &self.scene_bind, &[]);
                pass.draw(0..3, 0..1);
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("synthetic clouds"),
                color_attachments: &[&targets.cloud_map, &targets.sky_map].map(|view| {
                    Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })
                }),
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(&self.clouds);
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_bind_group(3, &self.noise_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = Self::pass(&mut encoder, targets);
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_bind_group(2, &targets.shadow_bind, &[]);
            pass.set_bind_group(3, &self.noise_bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);
            pass.set_vertex_buffer(1, draws.buffer.slice(..));
            let bound = Cell::new(None);
            pass.set_pipeline(&self.backfill);
            behind.iter().enumerate().for_each(|(slot, (_, gpu))| {
                record(&mut pass, &self.arenas, &bound, gpu, placed(slot))
            });
            LAYERS
                .iter()
                .zip(&self.pipelines)
                .filter(|(layer, _)| **layer != Layer::OnTop)
                .for_each(|(layer, pipeline)| {
                    pass.set_pipeline(pipeline);
                    layer_draws(&mut pass, &bound, *layer);
                });
        }
        self.effects(&mut encoder, targets);
        self.temporal(&mut encoder, targets);
        {
            let mut pass = self.finish(&mut encoder, target, targets);
            pass.set_vertex_buffer(1, draws.buffer.slice(..));
            LAYERS
                .iter()
                .zip(&self.pipelines)
                .filter(|(layer, _)| **layer == Layer::OnTop)
                .for_each(|(layer, pipeline)| {
                    pass.set_pipeline(pipeline);
                    layer_draws(&mut pass, &Cell::new(None), *layer);
                });
        }
        self.queue.submit([encoder.finish()]);
        targets.frame.set(targets.frame.get() + 1);
    }

    fn draw_buffer(&self, slots: usize) -> Draws {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic draws"),
            size: slots as u64 * DRAW_BYTES,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Draws { buffer, slots }
    }

    pub fn clear(&mut self, target: &wgpu::TextureView, targets: &Targets) {
        self.queue
            .write_buffer(&self.scene, SCENE_BYTES - 16, &bytes([0.0; 4]));
        targets.previous.set(None);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        drop(Self::pass(&mut encoder, targets));
        self.effects(&mut encoder, targets);
        self.temporal(&mut encoder, targets);
        drop(self.finish(&mut encoder, target, targets));
        self.queue.submit([encoder.finish()]);
        targets.frame.set(targets.frame.get() + 1);
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }

    pub fn targets(&self, width: u32, height: u32) -> Targets {
        let inner =
            |side: u32| ((f64::from(side) * self.quality.render_scale).round() as u32).max(1);
        let (width, height) = (inner(width), inner(height));
        let sized = |label: &str,
                     format: wgpu::TextureFormat,
                     sample_count: u32,
                     usage: wgpu::TextureUsages,
                     shrink: u32| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: (width >> shrink).max(1),
                        height: (height >> shrink).max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let texture = |label: &str,
                       format: wgpu::TextureFormat,
                       sample_count: u32,
                       usage: wgpu::TextureUsages| {
            sized(label, format, sample_count, usage, 0)
        };
        let transient =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TRANSIENT_ATTACHMENT;
        let readable =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let resolved = texture("synthetic resolved", HDR, 1, readable);
        let mips: Vec<wgpu::TextureView> = (0..BLOOM_MIPS)
            .map(|level| sized("synthetic bloom", HDR, 1, readable, level as u32 + 1))
            .collect();
        let surface = texture("synthetic surface", SURFACE, 1, readable);
        let cloud_map = sized(
            "synthetic clouds",
            HDR,
            1,
            readable,
            self.quality.cloud_shrink,
        );
        let sky_map = sized("synthetic sky", HDR, 1, readable, self.quality.cloud_shrink);
        let occlusion = sized("synthetic occlusion", OCCLUSION, 1, readable, 1);
        let blurred = sized("synthetic occlusion", OCCLUSION, 1, readable, 1);
        let bind =
            |picture: &wgpu::TextureView, second: &wgpu::TextureView, third: &wgpu::TextureView| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("synthetic screen"),
                    layout: &self.screen_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(picture),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(second),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(third),
                        },
                    ],
                })
            };
        let down_binds = std::iter::once(&resolved)
            .chain(&mips)
            .take(BLOOM_MIPS)
            .map(|source| bind(source, source, source))
            .collect();
        let up_binds = mips
            .iter()
            .skip(1)
            .map(|source| bind(source, source, source))
            .collect();
        let post_bind = bind(&resolved, &mips[0], &blurred);
        let tonemapped = texture("synthetic tonemapped", HDR, 1, readable);
        let history = [
            texture("synthetic history", HDR, 1, readable),
            texture("synthetic history", HDR, 1, readable),
        ];
        let taa_binds = [
            bind(&tonemapped, &history[1], &surface),
            bind(&tonemapped, &history[0], &surface),
        ];
        let motion_binds = [
            bind(&history[0], &surface, &surface),
            bind(&history[1], &surface, &surface),
        ];
        let ssao_bind = bind(&surface, &surface, &surface);
        let blur_bind = bind(&occlusion, &surface, &surface);
        let rays_bind = bind(&resolved, &surface, &surface);
        let shadow = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic shadow"),
            size: wgpu::Extent3d {
                width: self.quality.shadow_size,
                height: self.quality.shadow_size,
                depth_or_array_layers: CASCADES as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: readable,
            view_formats: &[],
        });
        let shadow_layers = (0..CASCADES as u32)
            .map(|layer| {
                shadow.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: layer,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        let shadow_bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("synthetic shadow"),
            layout: &self.shadow_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow.create_view(
                        &wgpu::TextureViewDescriptor {
                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&cloud_map),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.post_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&sky_map),
                },
            ],
        });
        Targets {
            shadow_layers,
            shadow_bind,
            cloud_map,
            sky_map,
            samples: (self.quality.samples > 1).then(|| {
                (
                    texture("synthetic samples", HDR, self.quality.samples, transient),
                    texture(
                        "synthetic surface samples",
                        SURFACE,
                        self.quality.samples,
                        transient,
                    ),
                )
            }),
            depth: texture("synthetic depth", DEPTH, self.quality.samples, transient),
            resolved,
            surface,
            occlusion,
            blurred,
            ssao_bind,
            blur_bind,
            mips,
            down_binds,
            up_binds,
            post_bind,
            rays_bind,
            history,
            taa_binds,
            motion_binds,
            tonemapped,
            frame: Cell::new(0),
            previous: Cell::new(None),
            height_px: height,
        }
    }

    pub fn picture(
        &mut self,
        camera: &Camera,
        backfill: &[Arc<Mesh>],
        meshes: &[Arc<Mesh>],
        targets: &Targets,
        width: u32,
        height: u32,
    ) -> Result<image::RgbaImage, String> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic picture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.draw(
            &target.create_view(&Default::default()),
            targets,
            camera,
            backfill,
            meshes,
        );
        let row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(height),
                },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| error.to_string())?;
        let mapped = readback
            .slice(..)
            .get_mapped_range()
            .map_err(|error| error.to_string())?;
        let pixels: Vec<u8> = mapped
            .chunks(row as usize)
            .flat_map(|line| &line[..(4 * width) as usize])
            .copied()
            .collect();
        image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "the read-back picture had the wrong size".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DMat4;

    fn mesh() -> Arc<Mesh> {
        Arc::new(Mesh::new(
            DMat4::IDENTITY,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            image::RgbaImage::new(1, 1),
            Layer::Opaque,
        ))
    }

    #[test]
    fn every_draw_gets_its_own_slot_in_one_shared_buffer() {
        assert_eq!(
            draw_slots(10),
            1024,
            "the buffer starts big enough for a city"
        );
        assert_eq!(
            draw_slots(1100),
            2048,
            "and doubles rather than growing a draw at a time"
        );
    }

    #[test]
    fn a_mesh_uploads_once_and_leaves_the_gpu_when_the_scene_drops_it() {
        let mut resident: Resident<u64> = Resident::default();
        let (kept, dropped) = (mesh(), mesh());
        assert_eq!(
            resident.ensure(&[kept.clone(), dropped.clone()], |mesh| mesh.id),
            2
        );
        assert_eq!(
            resident.ensure(std::slice::from_ref(&kept), |mesh| mesh.id),
            0,
            "not drawn this frame is not a reason to upload again"
        );
        drop(dropped);
        assert_eq!(resident.evict().len(), 1);
        assert_eq!(resident.get(kept.id), Some(&kept.id));
    }

    #[test]
    fn a_new_mesh_never_inherits_a_dropped_one_even_at_the_same_address() {
        let mut resident: Resident<u64> = Resident::default();
        let first = mesh();
        let first_id = first.id;
        resident.ensure(std::slice::from_ref(&first), |mesh| mesh.id);
        drop(first);
        let second = mesh();
        resident.evict();
        assert_eq!(
            resident.ensure(std::slice::from_ref(&second), |mesh| mesh.id),
            1,
            "the new mesh gets its own upload"
        );
        assert_eq!(resident.get(second.id), Some(&second.id));
        assert_eq!(resident.get(first_id), None);
    }

    #[test]
    fn meshes_share_one_arena_and_an_emptied_extra_arena_gives_its_memory_back() {
        let mut arenas: Vec<Option<Arena<()>>> = Vec::new();
        let first = place_in(&mut arenas, 100, 300, |_, _| ()).expect("room");
        let second = place_in(&mut arenas, 100, 300, |_, _| ()).expect("room");
        assert_eq!(
            (first.arena, second.arena, second.vertices.clone()),
            (0, 0, 100..200),
            "packed after the first in the same buffers"
        );
        let opened = Cell::new(0);
        let big = place_in(&mut arenas, ARENA_VERTICES + 1, 3, |vertices, _| {
            opened.set(vertices)
        })
        .expect("room");
        assert_eq!(
            (big.arena, opened.get()),
            (1, ARENA_VERTICES + 1),
            "a mesh bigger than an arena gets one sized to fit"
        );
        release(&mut arenas, big);
        close_empty(&mut arenas);
        assert!(arenas[1].is_none(), "an emptied extra arena is freed");
        assert_eq!(
            place_in(&mut arenas, ARENA_VERTICES, 3, |_, _| ()).map(|place| place.arena),
            Some(1),
            "and its slot is reused"
        );
        [first, second]
            .into_iter()
            .for_each(|place| release(&mut arenas, place));
        close_empty(&mut arenas);
        assert!(
            arenas[0].is_some(),
            "the first arena stays for the next tiles"
        );
    }

    #[test]
    fn an_arena_out_of_index_room_hands_back_the_vertices_it_tried() {
        let mut arenas: Vec<Option<Arena<()>>> = Vec::new();
        place_in(&mut arenas, 10, ARENA_INDICES, |_, _| ()).expect("room");
        assert_eq!(
            place_in(&mut arenas, 10, 3, |_, _| ()).map(|place| place.arena),
            Some(1)
        );
        assert_eq!(
            arenas[0]
                .as_ref()
                .map(|arena| arena.vertex_space.total_available()),
            Some(ARENA_VERTICES - 10)
        );
    }
}
