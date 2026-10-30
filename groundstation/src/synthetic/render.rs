use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::{Arc, Weak};

use range_alloc::RangeAllocator;
use web_time::Instant;

use super::geo::Camera;
use super::mesh::{Layer, Mesh, solid};

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
const SCENE_BYTES: u64 = 416;
const SAMPLES: u32 = 4;
const FIRST_DRAW_SLOTS: usize = 1024;
const MISSING_TEXTURE: [u8; 4] = [255, 255, 255, 255];
const LAYERS: [Layer; 3] = [Layer::Opaque, Layer::Translucent, Layer::OnTop];

const ANISOTROPY: u16 = 16;
const BLOOM: bool = true;
const BLOOM_MIPS: usize = 5;
const SSAO: bool = true;
const SURFACE: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const OCCLUSION: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
const SHADOWS: bool = true;
const SHADOW_SIZE: u32 = 2048;
const CASCADES: usize = 3;
const CASCADE_FAR_M: [f64; CASCADES] = [300.0, 2000.0, 10000.0];
const CASTER_REACH_M: f64 = 10000.0;
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
const DRAW_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4];
const SHADER: &str = r"
struct Scene { unproject: mat4x4<f32>, project: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32>, light: array<mat4x4<f32>, 3>, cascades: array<vec4<f32>, 3>, eye: vec4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var tile_sampler: sampler;
@group(2) @binding(0) var shadow: texture_depth_2d_array;
@group(2) @binding(1) var shadow_sampler: sampler_comparison;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32> };
struct Placed {
    @location(3) mvp0: vec4<f32>, @location(4) mvp1: vec4<f32>, @location(5) mvp2: vec4<f32>, @location(6) mvp3: vec4<f32>,
    @location(7) model0: vec4<f32>, @location(8) model1: vec4<f32>, @location(9) model2: vec4<f32>, @location(10) model3: vec4<f32>,
};
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) seen: vec3<f32>, @location(2) normal: vec3<f32> };
struct SkyOut { @builtin(position) position: vec4<f32>, @location(0) screen: vec2<f32> };
struct Frag { @location(0) colour: vec4<f32>, @location(1) surface: vec4<f32> };

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
const CLOUD_EXTINCTION = 0.006;
const CLOUD_ALBEDO = 0.92;
const CLOUD_DRIFT = vec2<f32>(5.0, 2.0);
const CLOUD_LIGHT_STEP_M = 220.0;
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

fn value_noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let low = mix(hash(cell), hash(cell + vec2<f32>(1.0, 0.0)), u.x);
    let high = mix(hash(cell + vec2<f32>(0.0, 1.0)), hash(cell + vec2<f32>(1.0, 1.0)), u.x);
    return mix(low, high, u.y);
}

fn fbm(p: vec2<f32>, octaves: i32) -> f32 {
    var total = 0.0;
    var amplitude = 0.5;
    var at = p;
    for (var i = 0; i < octaves; i++) {
        total += amplitude * value_noise(at);
        at = at * 2.03 + 17.1;
        amplitude *= 0.5;
    }
    return total;
}

fn dither(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

fn henyey_greenstein(cosine: f32, g: f32) -> f32 {
    return (1.0 - g * g) / (4.0 * PI * pow(1.0 + g * g - 2.0 * g * cosine, 1.5));
}

fn cloud_density(horizontal: vec2<f32>, height: f32, octaves: i32, fineness: f32) -> f32 {
    let shape = fbm(horizontal * CLOUD_SCALE + CLOUD_DRIFT * scene.zenith.w * CLOUD_SCALE, octaves);
    let rise = (height - CLOUD_BASE_M) / (CLOUD_TOP_M - CLOUD_BASE_M);
    let profile = smoothstep(0.0, 0.12, rise) * (1.0 - smoothstep(0.55, 1.0, rise));
    let detail = fbm(horizontal * CLOUD_SCALE * 6.0 + vec2<f32>(3.7, 1.3) + CLOUD_DRIFT * scene.zenith.w * CLOUD_SCALE * 1.4, octaves);
    let body = clamp((shape - (1.0 - CLOUD_COVER)) / CLOUD_COVER * 6.0 - rise * 0.8 - detail * 0.8 * fineness, 0.0, 1.0);
    return body * profile;
}

fn clouds(direction: vec3<f32>, jitter: f32) -> vec4<f32> {
    let up = dot(direction, scene.zenith.xyz);
    if (!CLOUDS || up < 0.08) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }
    let fade = smoothstep(0.08, 0.25, up);
    let start = CLOUD_BASE_M / up;
    let steps = select(CLOUD_STEPS, CLOUD_GRAZING_STEPS, up < 0.3);
    let step = min(CLOUD_TOP_M / up - start, 18000.0) / f32(steps);
    let east = normalize(cross(scene.zenith.xyz, vec3<f32>(0.0, 0.0, 1.0)));
    let north = cross(scene.zenith.xyz, east);
    let sun_up = dot(scene.sun.xyz, scene.zenith.xyz);
    let sunlight = SUN * exp(-sun_depth(CLOUD_BASE_M));
    let ambient = sky_air(scene.zenith.xyz, 2).light * 0.8;
    let cosine = dot(direction, scene.sun.xyz);
    let phase = 4.0 * PI * (0.6 * henyey_greenstein(cosine, 0.4) + 0.4 * henyey_greenstein(cosine, -0.15));
    var through = 1.0;
    var light = vec3<f32>(0.0);
    for (var i = 0; i < steps; i++) {
        let t = start + (f32(i) + 0.25 + 0.5 * jitter) * step;
        let fineness = 1.0 - smoothstep(6000.0, 15000.0, t);
        let p = direction * t;
        let density = cloud_density(vec2<f32>(dot(scene.eye.xyz + p, east), dot(scene.eye.xyz + p, north)), up * t, 4, fineness);
        if (density <= 0.0) { continue; }
        var toward = 0.0;
        for (var j = 0; j < 2; j++) {
            let q = p + scene.sun.xyz * (f32(j) + 0.5) * CLOUD_LIGHT_STEP_M;
            toward += cloud_density(vec2<f32>(dot(scene.eye.xyz + q, east), dot(scene.eye.xyz + q, north)), up * t + sun_up * (f32(j) + 0.5) * CLOUD_LIGHT_STEP_M, 3, fineness) * CLOUD_LIGHT_STEP_M;
        }
        let beer = exp(-toward * CLOUD_EXTINCTION);
        let powder = 1.0 - exp(-toward * CLOUD_EXTINCTION * 2.0);
        let lit = sunlight * CLOUD_ALBEDO / PI * 0.6 * phase * beer * mix(0.5, 1.0, powder) + ambient * CLOUD_ALBEDO;
        let transmit = exp(-density * CLOUD_EXTINCTION * step);
        light += through * (1.0 - transmit) * lit;
        through *= transmit;
        if (through < 0.01) { break; }
    }
    return vec4<f32>(light * fade, mix(1.0, through, fade));
}

fn clouded_sky(direction: vec3<f32>, jitter: f32) -> vec3<f32> {
    let cover = clouds(direction, jitter);
    let clear = sky(direction);
    if (cover.a >= 1.0) { return clear; }
    let near = scatter(direction, CLOUD_BASE_M / max(dot(direction, scene.zenith.xyz), 0.015), 4);
    return clear * cover.a + cover.rgb * near.through + near.light * (1.0 - cover.a);
}

fn fog_amount(seen: vec3<f32>) -> f32 {
    if (!FOG) { return 0.0; }
    let climb = dot(seen, scene.zenith.xyz);
    let along = select(FOG_HEIGHT_M * (1.0 - exp(-climb / FOG_HEIGHT_M)) / climb, 1.0, abs(climb) < 1.0);
    return 1.0 - exp(-FOG_DENSITY * length(seen) * along);
}

fn fog_colour(direction: vec3<f32>) -> vec3<f32> {
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
    let lifted = position + normal * detail.y * 1.5;
    let seen = scene.light[cascade] * vec4<f32>(lifted, 1.0);
    let uv = seen.xy * vec2<f32>(0.5, -0.5) + 0.5;
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || seen.z > 1.0) { return 1.0; }
    let texel = 1.0 / vec2<f32>(textureDimensions(shadow));
    var reached = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            reached += textureSampleCompareLevel(shadow, shadow_sampler, uv + vec2<f32>(f32(x), f32(y)) * texel, cascade, seen.z - detail.z);
        }
    }
    return reached / 9.0;
}

fn sunlight_reaching(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let distance = length(position);
    if (distance >= scene.cascades[2].x) { return 1.0; }
    let cascade = select(select(0, 1, distance >= scene.cascades[0].x), 2, distance >= scene.cascades[1].x);
    let near = shadowed(cascade, position, normal);
    let edge = scene.cascades[cascade].x;
    let fade = smoothstep(edge * 0.85, edge, distance);
    if (fade <= 0.0 || cascade == 2) { return near; }
    return mix(near, shadowed(cascade + 1, position, normal), fade);
}

fn sunlit(colour: vec3<f32>, normal: vec3<f32>, seen: vec3<f32>, lit: f32) -> vec3<f32> {
    let sunlight = SUN * exp(-sun_depth(0.0)) / PI;
    if (dot(normal, normal) < 0.25) { return colour * (sunlight * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0) + sky_air(scene.zenith.xyz, 2).light) * mix(BAKED_SHADE, 1.0, lit); }
    let outward = facing(normal, seen);
    let up = dot(outward, scene.zenith.xyz);
    let sideways = normalize(seen - scene.zenith.xyz * dot(seen, scene.zenith.xyz));
    let skylight = mix(sky_air(sideways, 2).light, sky_air(scene.zenith.xyz, 2).light, 0.5 + 0.5 * up);
    let bounce = GROUND_ALBEDO * sunlight * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0) * (0.5 - 0.5 * up);
    return colour * (sunlight * max(dot(outward, scene.sun.xyz), 0.0) * lit + skylight + bounce);
}

@vertex fn vs(v: In, placed: Placed) -> Out {
    let mvp = mat4x4<f32>(placed.mvp0, placed.mvp1, placed.mvp2, placed.mvp3);
    let model = mat4x4<f32>(placed.model0, placed.model1, placed.model2, placed.model3);
    return Out(mvp * vec4<f32>(v.position, 1.0), v.uv, (model * vec4<f32>(v.position, 1.0)).xyz, (model * vec4<f32>(v.normal, 0.0)).xyz);
}

@fragment fn fs(f: Out) -> Frag {
    let colour = textureSample(picture, tile_sampler, f.uv);
    let distance = max(length(f.seen), 1.0);
    let air = scatter(f.seen / distance, distance, 4);
    let normal = select(vec3<f32>(0.0), facing(f.normal, f.seen), dot(f.normal, f.normal) >= 0.25);
    let lit = sunlight_reaching(f.seen, normal);
    return Frag(vec4<f32>(fogged(sunlit(colour.rgb, f.normal, f.seen, lit) * air.through + air.light, f.seen), colour.a), vec4<f32>(normal, distance));
}

@fragment fn fs_plain(f: Out) -> @location(0) vec4<f32> {
    return textureSample(picture, tile_sampler, f.uv);
}

@vertex fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return SkyOut(vec4<f32>(corner, 0.0, 1.0), corner);
}

@fragment fn fs_sky(f: SkyOut) -> Frag {
    let through = scene.unproject * vec4<f32>(f.screen, 0.5, 1.0);
    let direction = normalize(through.xyz / through.w);
    let up = dot(direction, scene.zenith.xyz);
    let reach = min(select(to_space(up), -2.0 * PLANET_M * up + 20000.0, up < 0.0), 60000.0);
    return Frag(vec4<f32>(fogged(clouded_sky(direction, dither(f.position.xy)), direction * reach), 1.0), vec4<f32>(0.0));
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
struct Scene { unproject: mat4x4<f32>, project: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var screen_sampler: sampler;
@group(1) @binding(1) var picture: texture_2d<f32>;
@group(1) @binding(2) var second: texture_2d<f32>;
@group(1) @binding(3) var third: texture_2d<f32>;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };

const EXPOSURE = 0.36;
const BLOOM = 0.06;
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
const EDGE_THRESHOLD = 0.166;
const EDGE_THRESHOLD_MIN = 0.0833;
const SUBPIXEL = 0.75;
const SEARCH_STEPS = 12;

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

fn shown(uv: vec2<f32>) -> vec4<f32> {
    let glow = textureSampleLevel(second, screen_sampler, uv, 0.0).rgb * BLOOM;
    let shade = max(1.0 - textureSampleLevel(third, screen_sampler, uv, 0.0).r * AO_STRENGTH, 0.0);
    let colour = aces((textureSampleLevel(picture, screen_sampler, uv, 0.0).rgb * shade + glow) * EXPOSURE);
    return vec4<f32>(colour, sqrt(dot(colour, vec3<f32>(0.299, 0.587, 0.114))));
}

fn search_step(i: i32) -> f32 {
    return select(select(2.0, 1.5, i == 1), select(4.0, 8.0, i == SEARCH_STEPS - 1), i >= SEARCH_STEPS - 2);
}

@fragment fn fs(f: Out) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(picture));
    let m = shown(f.uv);
    let n = shown(f.uv + vec2<f32>(0.0, -texel.y)).a;
    let s = shown(f.uv + vec2<f32>(0.0, texel.y)).a;
    let w = shown(f.uv + vec2<f32>(-texel.x, 0.0)).a;
    let e = shown(f.uv + vec2<f32>(texel.x, 0.0)).a;
    let brightest = max(m.a, max(max(n, s), max(w, e)));
    let range = brightest - min(m.a, min(min(n, s), min(w, e)));
    if (range < max(EDGE_THRESHOLD_MIN, brightest * EDGE_THRESHOLD)) { return vec4<f32>(m.rgb, 1.0); }
    let nw = shown(f.uv - texel).a;
    let ne = shown(f.uv + vec2<f32>(texel.x, -texel.y)).a;
    let sw = shown(f.uv + vec2<f32>(-texel.x, texel.y)).a;
    let se = shown(f.uv + texel).a;
    let edge_horizontal = abs(-2.0 * w + nw + sw) + abs(-2.0 * m.a + n + s) * 2.0 + abs(-2.0 * e + ne + se);
    let edge_vertical = abs(-2.0 * s + sw + se) + abs(-2.0 * m.a + w + e) * 2.0 + abs(-2.0 * n + nw + ne);
    let horizontal = edge_horizontal >= edge_vertical;
    let luma1 = select(w, n, horizontal);
    let luma2 = select(e, s, horizontal);
    let gradient1 = luma1 - m.a;
    let gradient2 = luma2 - m.a;
    let steepest1 = abs(gradient1) >= abs(gradient2);
    let gradient_scaled = 0.25 * max(abs(gradient1), abs(gradient2));
    let step_length = select(texel.x, texel.y, horizontal) * select(1.0, -1.0, steepest1);
    let local_average = 0.5 * (select(luma2, luma1, steepest1) + m.a);
    let along = select(vec2<f32>(0.0, texel.y), vec2<f32>(texel.x, 0.0), horizontal);
    let across = select(vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), horizontal);
    let start = f.uv + across * step_length * 0.5;
    var uv1 = start - along;
    var uv2 = start + along;
    var end1 = shown(uv1).a - local_average;
    var end2 = shown(uv2).a - local_average;
    var reached1 = abs(end1) >= gradient_scaled;
    var reached2 = abs(end2) >= gradient_scaled;
    for (var i = 1; i < SEARCH_STEPS; i++) {
        if (reached1 && reached2) { break; }
        if (!reached1) {
            uv1 -= along * search_step(i);
            end1 = shown(uv1).a - local_average;
            reached1 = abs(end1) >= gradient_scaled;
        }
        if (!reached2) {
            uv2 += along * search_step(i);
            end2 = shown(uv2).a - local_average;
            reached2 = abs(end2) >= gradient_scaled;
        }
    }
    let distance1 = select(f.uv.y - uv1.y, f.uv.x - uv1.x, horizontal);
    let distance2 = select(uv2.y - f.uv.y, uv2.x - f.uv.x, horizontal);
    let nearest1 = distance1 < distance2;
    let pixel_offset = 0.5 - min(distance1, distance2) / (distance1 + distance2);
    let darker_centre = m.a < local_average;
    let right_way = ((select(end2, end1, nearest1)) < 0.0) != darker_centre;
    let edge_offset = select(0.0, pixel_offset, right_way);
    let average = (2.0 * (n + s + w + e) + nw + ne + sw + se) / 12.0;
    let subpixel1 = clamp(abs(average - m.a) / range, 0.0, 1.0);
    let subpixel2 = (-2.0 * subpixel1 + 3.0) * subpixel1 * subpixel1;
    let offset = max(edge_offset, subpixel2 * subpixel2 * SUBPIXEL);
    return vec4<f32>(shown(f.uv + across * offset * step_length).rgb, 1.0);
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
    samples: wgpu::TextureView,
    depth: wgpu::TextureView,
    resolved: wgpu::TextureView,
    surface_samples: wgpu::TextureView,
    surface: wgpu::TextureView,
    occlusion: wgpu::TextureView,
    blurred: wgpu::TextureView,
    ssao_bind: wgpu::BindGroup,
    blur_bind: wgpu::BindGroup,
    mips: Vec<wgpu::TextureView>,
    down_binds: Vec<wgpu::BindGroup>,
    up_binds: Vec<wgpu::BindGroup>,
    post_bind: wgpu::BindGroup,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: [wgpu::RenderPipeline; 3],
    backfill: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    post: wgpu::RenderPipeline,
    prefilter: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    ssao: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
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

fn cascade(camera: &Camera, near: f64, far: f64) -> Cascade {
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
    let texel = 2.0 * radius / f64::from(SHADOW_SIZE);
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
        bias: (0.5 * texel).max(0.5) / (deep - close),
    }
}

fn cascades(camera: &Camera) -> Vec<Cascade> {
    CASCADE_FAR_M
        .iter()
        .scan(0.0, |near, far| {
            let fitted = cascade(camera, *near, *far);
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
) -> wgpu::RenderPipeline {
    let (depth_write, compare, blend, fragment) = match layer {
        Layer::Backfill => (false, wgpu::CompareFunction::Always, None, "fs"),
        Layer::Opaque => (true, wgpu::CompareFunction::Greater, None, "fs"),
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
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &vertex_buffers(),
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: (layer != Layer::OnTop).then(|| depth_state(depth_write, compare)),
        multisample: multisample(if layer == Layer::OnTop { 1 } else { SAMPLES }),
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
        multisample: multisample(SAMPLES),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
            ],
        });
        let cascade_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic cascade"),
            entries: &[uniform(0, wgpu::ShaderStages::VERTEX)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic"),
            bind_group_layouts: &[Some(&scene_layout), Some(&layout), Some(&shadow_layout)],
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
            bind_group_layouts: &[Some(&scene_layout)],
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
            pipeline(&device, layout, &shader, color, layer)
        });
        let backfill = pipeline(&device, &pipeline_layout, &shader, HDR, Layer::Backfill);
        let sky = sky_pipeline(&device, &sky_layout, &shader, HDR);
        let screen = |entry: &str, color: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
            screen_pipeline(&device, &post_layout, &post_shader, entry, color, blend)
        };
        let post = screen("fs", format, None);
        let prefilter = screen("fs_prefilter", HDR, None);
        let down = screen("fs_down", HDR, None);
        let up = screen("fs_up", HDR, Some(ADDITIVE));
        let ssao = screen("fs_ssao", OCCLUSION, None);
        let blur = screen("fs_blur", OCCLUSION, None);
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
        Self {
            device,
            queue,
            pipelines,
            backfill,
            shadow,
            sky,
            post,
            prefilter,
            down,
            up,
            ssao,
            blur,
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

    pub fn resident(&self) -> usize {
        self.resident.len()
    }

    fn pass<'a>(encoder: &'a mut wgpu::CommandEncoder, targets: &Targets) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.samples,
                    depth_slice: None,
                    resolve_target: Some(&targets.resolved),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(SKY),
                        store: wgpu::StoreOp::Discard,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: &targets.surface_samples,
                    depth_slice: None,
                    resolve_target: Some(&targets.surface),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                }),
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
        pass.set_bind_group(1, &targets.post_bind, &[]);
        pass.draw(0..3, 0..1);
        pass
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
        if SSAO {
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
        if !BLOOM {
            return;
        }
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
            .for_each(|(mip, bind)| self.screen(encoder, mip, wgpu::LoadOp::Load, &self.up, bind));
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
        let view_projection = camera.projection() * camera.view();
        let layer_of = |mesh: &Mesh| {
            LAYERS
                .iter()
                .position(|layer| *layer == mesh.layer)
                .unwrap_or(LAYERS.len())
        };
        let arena_of = |mesh: &Mesh, gpu: &GpuMesh| {
            gpu.place
                .as_ref()
                .filter(|_| mesh.layer == Layer::Opaque)
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
        let cascades = cascades(camera);
        if !matrices.is_empty() {
            self.queue
                .write_buffer(&draws.buffer, 0, bytemuck::cast_slice(&matrices));
        }
        let placed = |slot: usize| u32::try_from(slot).map_or(0..0, |slot| slot..slot + 1);
        let direction = |v: glam::DVec3| v.as_vec3().extend(0.0).to_array();
        let elapsed = self.started.elapsed().as_secs_f32() % 86400.0;
        self.queue.write_buffer(
            &self.scene,
            0,
            &bytes(
                camera
                    .unproject()
                    .as_mat4()
                    .to_cols_array()
                    .into_iter()
                    .chain(view_projection.as_mat4().to_cols_array())
                    .chain(camera.zenith.as_vec3().extend(elapsed).to_array())
                    .chain(direction(camera.sun))
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
                    .chain(direction(camera.eye)),
            ),
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        cascades
            .iter()
            .zip(&targets.shadow_layers)
            .zip(&self.cascades)
            .filter(|_| SHADOWS)
            .for_each(|((cascade, layer), (buffer, bind))| {
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
                        mesh.layer == Layer::Opaque && casts(cascade, **model, mesh)
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
        {
            let mut pass = Self::pass(&mut encoder, targets);
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_bind_group(2, &targets.shadow_bind, &[]);
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
        let mut encoder = self.device.create_command_encoder(&Default::default());
        drop(Self::pass(&mut encoder, targets));
        self.effects(&mut encoder, targets);
        drop(self.finish(&mut encoder, target, targets));
        self.queue.submit([encoder.finish()]);
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }

    pub fn targets(&self, width: u32, height: u32) -> Targets {
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
        let ssao_bind = bind(&surface, &surface, &surface);
        let blur_bind = bind(&occlusion, &surface, &surface);
        let shadow = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic shadow"),
            size: wgpu::Extent3d {
                width: SHADOW_SIZE,
                height: SHADOW_SIZE,
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
            ],
        });
        Targets {
            shadow_layers,
            shadow_bind,
            samples: texture("synthetic samples", HDR, SAMPLES, transient),
            depth: texture("synthetic depth", DEPTH, SAMPLES, transient),
            resolved,
            surface_samples: texture("synthetic surface samples", SURFACE, SAMPLES, transient),
            surface,
            occlusion,
            blurred,
            ssao_bind,
            blur_bind,
            mips,
            down_binds,
            up_binds,
            post_bind,
        }
    }

    pub fn picture(
        &mut self,
        camera: &Camera,
        backfill: &[Arc<Mesh>],
        meshes: &[Arc<Mesh>],
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
        let targets = self.targets(width, height);
        self.draw(
            &target.create_view(&Default::default()),
            &targets,
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
