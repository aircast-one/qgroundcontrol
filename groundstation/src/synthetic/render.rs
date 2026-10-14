use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::{Arc, Weak};

use range_alloc::RangeAllocator;

use super::geo::Camera;
use super::mesh::{Layer, Mesh, solid};

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SKY: wgpu::Color = wgpu::Color { r: 0.3, g: 0.45, b: 0.66, a: 1.0 };
const VERTEX_BYTES: u64 = 32;
const INDEX_BYTES: u64 = 4;
const ARENA_VERTICES: u32 = 1 << 19;
const ARENA_INDICES: u32 = 1 << 21;
const DRAW_BYTES: u64 = 128;
const SCENE_BYTES: u64 = 96;
const SAMPLES: u32 = 4;
const FIRST_DRAW_SLOTS: usize = 1024;
const MISSING_TEXTURE: [u8; 4] = [255, 255, 255, 255];
const LAYERS: [Layer; 3] = [Layer::Opaque, Layer::Translucent, Layer::OnTop];

const ANISOTROPY: u16 = 16;
const SHADER: &str = r"
struct Scene { unproject: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var picture: texture_2d<f32>;
@group(1) @binding(1) var tile_sampler: sampler;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32> };
struct Placed {
    @location(3) mvp0: vec4<f32>, @location(4) mvp1: vec4<f32>, @location(5) mvp2: vec4<f32>, @location(6) mvp3: vec4<f32>,
    @location(7) model0: vec4<f32>, @location(8) model1: vec4<f32>, @location(9) model2: vec4<f32>, @location(10) model3: vec4<f32>,
};
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) seen: vec3<f32>, @location(2) normal: vec3<f32> };
struct SkyOut { @builtin(position) position: vec4<f32>, @location(0) screen: vec2<f32> };

const ZENITH = vec3<f32>(0.06, 0.18, 0.5);
const HORIZON = vec3<f32>(0.5, 0.62, 0.78);
const SUNLIGHT = vec3<f32>(1.0, 0.9, 0.72);
const HAZE_M = 25000.0;
const AMBIENT = 0.3;

fn sky(direction: vec3<f32>) -> vec3<f32> {
    let above = pow(clamp(dot(direction, scene.zenith.xyz), 0.0, 1.0), 0.4);
    let glow = pow(max(dot(direction, scene.sun.xyz), 0.0), 16.0) * 0.3;
    return mix(HORIZON, ZENITH, above) + SUNLIGHT * glow;
}

fn sunlit(colour: vec3<f32>, normal: vec3<f32>, seen: vec3<f32>) -> vec3<f32> {
    if (dot(normal, normal) < 0.25) { return colour; }
    let outward = select(normalize(normal), -normalize(normal), dot(normal, seen) > 0.0);
    let level = AMBIENT + (1.0 - AMBIENT) * max(dot(scene.zenith.xyz, scene.sun.xyz), 0.0);
    let skylight = AMBIENT * (0.5 + 0.5 * max(dot(outward, scene.zenith.xyz), 0.0));
    return colour * (skylight + (1.0 - AMBIENT) * max(dot(outward, scene.sun.xyz), 0.0)) / level;
}

@vertex fn vs(v: In, placed: Placed) -> Out {
    let mvp = mat4x4<f32>(placed.mvp0, placed.mvp1, placed.mvp2, placed.mvp3);
    let model = mat4x4<f32>(placed.model0, placed.model1, placed.model2, placed.model3);
    return Out(mvp * vec4<f32>(v.position, 1.0), v.uv, (model * vec4<f32>(v.position, 1.0)).xyz, (model * vec4<f32>(v.normal, 0.0)).xyz);
}

@fragment fn fs(f: Out) -> @location(0) vec4<f32> {
    let colour = textureSample(picture, tile_sampler, f.uv);
    let distance = max(length(f.seen), 1.0);
    let haze = 1.0 - exp(-distance / HAZE_M);
    return vec4<f32>(mix(sunlit(colour.rgb, f.normal, f.seen), sky(f.seen / distance), haze), colour.a);
}

@fragment fn fs_plain(f: Out) -> @location(0) vec4<f32> {
    return textureSample(picture, tile_sampler, f.uv);
}

@vertex fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOut {
    let corner = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - 1.0;
    return SkyOut(vec4<f32>(corner, 0.0, 1.0), corner);
}

@fragment fn fs_sky(f: SkyOut) -> @location(0) vec4<f32> {
    let through = scene.unproject * vec4<f32>(f.screen, 0.5, 1.0);
    return vec4<f32>(sky(normalize(through.xyz / through.w)), 1.0);
}
";

pub struct Resident<T> {
    entries: HashMap<u64, (Weak<Mesh>, T)>,
}

impl<T> Default for Resident<T> {
    fn default() -> Self {
        Self { entries: HashMap::new() }
    }
}

impl<T> Resident<T> {
    pub fn ensure(&mut self, meshes: &[Arc<Mesh>], mut upload: impl FnMut(&Mesh) -> T) -> usize {
        let missing: Vec<&Arc<Mesh>> = meshes.iter().filter(|mesh| !self.entries.contains_key(&mesh.id)).collect();
        let uploaded = missing.len();
        missing.into_iter().for_each(|mesh| {
            let resident = upload(mesh);
            self.entries.insert(mesh.id, (Arc::downgrade(mesh), resident));
        });
        uploaded
    }

    pub fn evict(&mut self) -> Vec<T> {
        self.entries.extract_if(|_, (mesh, _)| mesh.strong_count() == 0).map(|(_, (_, resident))| resident).collect()
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

fn place_in<B>(arenas: &mut Vec<Option<Arena<B>>>, vertices: u32, indices: u32, open: impl FnOnce(u32, u32) -> B) -> Option<Place> {
    let fitted = arenas.iter_mut().enumerate().find_map(|(arena, slot)| {
        let space = slot.as_mut()?;
        let taken = space.vertex_space.allocate_range(vertices).ok()?;
        match space.index_space.allocate_range(indices) {
            Ok(indices) => Some(Place { arena, vertices: taken, indices }),
            Err(_) => {
                space.vertex_space.free_range(taken);
                None
            }
        }
    });
    fitted.or_else(|| {
        let arena = arenas.iter().position(Option::is_none).unwrap_or(arenas.len());
        let (vertex_capacity, index_capacity) = (vertices.max(ARENA_VERTICES), indices.max(ARENA_INDICES));
        let opened = Some(Arena { vertex_space: RangeAllocator::new(0..vertex_capacity), index_space: RangeAllocator::new(0..index_capacity), buffers: open(vertex_capacity, index_capacity) });
        match arenas.get_mut(arena) {
            Some(slot) => *slot = opened,
            None => arenas.push(opened),
        }
        let space = arenas.get_mut(arena)?.as_mut()?;
        Some(Place { arena, vertices: space.vertex_space.allocate_range(vertices).ok()?, indices: space.index_space.allocate_range(indices).ok()? })
    })
}

fn release<B>(arenas: &mut [Option<Arena<B>>], place: Place) {
    if let Some(space) = arenas.get_mut(place.arena).and_then(Option::as_mut) {
        space.vertex_space.free_range(place.vertices);
        space.index_space.free_range(place.indices);
    }
}

fn close_empty<B>(arenas: &mut [Option<Arena<B>>]) {
    arenas.iter_mut().skip(1).filter(|slot| slot.as_ref().is_some_and(|arena| arena.vertex_space.is_empty() && arena.index_space.is_empty())).for_each(|slot| *slot = None);
}

struct GpuMesh {
    place: Option<Place>,
    bind: wgpu::BindGroup,
}

fn record(pass: &mut wgpu::RenderPass, arenas: &[Option<Arena<ArenaBuffers>>], bound: &Cell<Option<usize>>, gpu: &GpuMesh, instances: Range<u32>) {
    let Some((place, arena)) = gpu.place.as_ref().and_then(|place| Some((place, arenas.get(place.arena)?.as_ref()?))) else { return };
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
    samples: wgpu::TextureView,
    depth: wgpu::TextureView,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipelines: [wgpu::RenderPipeline; 3],
    backfill: wgpu::RenderPipeline,
    sky: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    draws: Option<Draws>,
    scene: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    resident: Resident<GpuMesh>,
    arenas: Vec<Option<Arena<ArenaBuffers>>>,
    pub uploads: usize,
    to_upload: Vec<Weak<Mesh>>,
}

fn uniform(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }
}

fn bytes(values: impl IntoIterator<Item = f32>) -> Vec<u8> {
    bytemuck::cast_slice(&values.into_iter().collect::<Vec<f32>>()).to_vec()
}

fn depth_state(write: bool, compare: wgpu::CompareFunction) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(write), depth_compare: Some(compare), stencil: Default::default(), bias: Default::default() }
}

fn multisample() -> wgpu::MultisampleState {
    wgpu::MultisampleState { count: SAMPLES, ..Default::default() }
}

fn pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule, color: wgpu::TextureFormat, layer: Layer) -> wgpu::RenderPipeline {
    let (depth_write, compare, blend, fragment) = match layer {
        Layer::Backfill => (false, wgpu::CompareFunction::Always, None, "fs"),
        Layer::Opaque => (true, wgpu::CompareFunction::Greater, None, "fs"),
        Layer::Translucent => (false, wgpu::CompareFunction::Greater, Some(wgpu::BlendState::ALPHA_BLENDING), "fs"),
        Layer::OnTop => (false, wgpu::CompareFunction::Always, Some(wgpu::BlendState::ALPHA_BLENDING), "fs_plain"),
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[
                Some(wgpu::VertexBufferLayout { array_stride: VERTEX_BYTES, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3] }),
                Some(wgpu::VertexBufferLayout {
                    array_stride: DRAW_BYTES,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4],
                }),
            ],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(depth_state(depth_write, compare)),
        multisample: multisample(),
        fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some(fragment), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: color, blend, write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    })
}

fn sky_pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule, color: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic sky"),
        layout: Some(layout),
        vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_sky"), compilation_options: Default::default(), buffers: &[] },
        primitive: Default::default(),
        depth_stencil: Some(depth_state(false, wgpu::CompareFunction::Always)),
        multisample: multisample(),
        fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_sky"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: color, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        multiview_mask: None,
        cache: None,
    })
}

impl Renderer {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() })).map_err(|error| format!("no GPU adapter: {error}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|error| format!("the GPU refused a device: {error}"))?;
        Ok(Self::with_device(device, queue, COLOR))
    }

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("synthetic"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("synthetic scene"), entries: &[uniform(0, wgpu::ShaderStages::FRAGMENT)] });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("synthetic"), bind_group_layouts: &[Some(&scene_layout), Some(&layout)], immediate_size: 0 });
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("synthetic sky"), bind_group_layouts: &[Some(&scene_layout)], immediate_size: 0 });
        let pipelines = LAYERS.map(|layer| pipeline(&device, &pipeline_layout, &shader, format, layer));
        let backfill = pipeline(&device, &pipeline_layout, &shader, format, Layer::Backfill);
        let sky = sky_pipeline(&device, &sky_layout, &shader, format);
        let scene = device.create_buffer(&wgpu::BufferDescriptor { label: Some("synthetic scene"), size: SCENE_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("synthetic scene"), layout: &scene_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: scene.as_entire_binding() }] });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::MipmapFilterMode::Linear, address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, anisotropy_clamp: ANISOTROPY, ..Default::default() });
        Self { device, queue, format, pipelines, backfill, sky, layout, draws: None, scene, scene_bind, sampler, resident: Resident::default(), arenas: Vec::new(), uploads: 0, to_upload: Vec::new() }
    }

    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, arenas: &mut Vec<Option<Arena<ArenaBuffers>>>, mesh: &Mesh) -> GpuMesh {
        let normals = mesh.normals.iter().copied().chain(std::iter::repeat([0.0; 3]));
        let vertices: Vec<u8> = bytes(mesh.positions.iter().zip(&mesh.uvs).zip(normals).flat_map(|((p, uv), n)| p.iter().chain(uv).copied().chain(n)));
        let levels = mesh.take_texture().unwrap_or_else(|| vec![solid(MISSING_TEXTURE)]);
        let (width, height) = levels[0].dimensions();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
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
                wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                image.as_raw(),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
        });
        let view = texture.create_view(&Default::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) }],
        });
        let (vertex_count, index_count) = (u32::try_from(vertices.len() as u64 / VERTEX_BYTES).unwrap_or(0), u32::try_from(mesh.indices.len()).unwrap_or(0));
        let opened = |vertices: u32, indices: u32| ArenaBuffers {
            vertices: device.create_buffer(&wgpu::BufferDescriptor { label: Some("synthetic vertices"), size: u64::from(vertices) * VERTEX_BYTES, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }),
            indices: device.create_buffer(&wgpu::BufferDescriptor { label: Some("synthetic indices"), size: u64::from(indices) * INDEX_BYTES, usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false }),
        };
        let place = (vertex_count > 0 && index_count > 0).then(|| place_in(arenas, vertex_count, index_count, opened)).flatten();
        if let Some((place, arena)) = place.as_ref().and_then(|place| Some((place, arenas.get(place.arena)?.as_ref()?))) {
            let rebased: Vec<u32> = mesh.indices.iter().map(|index| index + place.vertices.start).collect();
            queue.write_buffer(&arena.buffers.vertices, u64::from(place.vertices.start) * VERTEX_BYTES, &vertices);
            queue.write_buffer(&arena.buffers.indices, u64::from(place.indices.start) * INDEX_BYTES, bytemuck::cast_slice(&rebased));
        }
        GpuMesh { place, bind }
    }

    pub fn resident(&self) -> usize {
        self.resident.len()
    }

    fn pass<'a>(encoder: &'a mut wgpu::CommandEncoder, target: &wgpu::TextureView, targets: &Targets) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &targets.samples, depth_slice: None, resolve_target: Some(target), ops: wgpu::Operations { load: wgpu::LoadOp::Clear(SKY), store: wgpu::StoreOp::Discard } })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &targets.depth, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Discard }), stencil_ops: None }),
            ..Default::default()
        })
    }

    pub fn warm(&mut self, arrived: Vec<Arc<Mesh>>, most: usize) -> usize {
        let queued = std::mem::take(&mut self.to_upload);
        let missing: Vec<Arc<Mesh>> = queued.iter().filter_map(Weak::upgrade).chain(arrived).filter(|mesh| self.resident.get(mesh.id).is_none()).collect();
        let (first, rest) = missing.split_at(missing.len().min(most));
        let (device, queue, layout, sampler, arenas) = (&self.device, &self.queue, &self.layout, &self.sampler, &mut self.arenas);
        self.uploads += self.resident.ensure(first, |mesh| Self::upload(device, queue, layout, sampler, arenas, mesh));
        self.to_upload = rest.iter().map(Arc::downgrade).collect();
        self.to_upload.len()
    }

    pub fn draw(&mut self, target: &wgpu::TextureView, targets: &Targets, camera: &Camera, backfill: &[Arc<Mesh>], meshes: &[Arc<Mesh>]) {
        let (device, queue, layout, sampler, arenas) = (&self.device, &self.queue, &self.layout, &self.sampler, &mut self.arenas);
        let everything: Vec<Arc<Mesh>> = backfill.iter().chain(meshes).cloned().collect();
        self.uploads += self.resident.ensure(&everything, |mesh| Self::upload(device, queue, layout, sampler, arenas, mesh));
        self.resident.evict().into_iter().filter_map(|gone| gone.place).for_each(|place| release(&mut self.arenas, place));
        close_empty(&mut self.arenas);
        let view_projection = camera.projection() * camera.view();
        let layer_of = |mesh: &Mesh| LAYERS.iter().position(|layer| *layer == mesh.layer).unwrap_or(LAYERS.len());
        let arena_of = |mesh: &Mesh, gpu: &GpuMesh| gpu.place.as_ref().filter(|_| mesh.layer == Layer::Opaque).map_or(0, |place| place.arena);
        let drawable: Vec<(&Arc<Mesh>, &GpuMesh)> = meshes
            .iter()
            .filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?)))
            .enumerate()
            .map(|(order, (mesh, gpu))| ((layer_of(mesh), arena_of(mesh, gpu), order), (mesh, gpu)))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        let behind: Vec<(&Arc<Mesh>, &GpuMesh)> = backfill.iter().filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?))).collect();
        let slots = behind.len() + drawable.len();
        if self.draws.as_ref().is_none_or(|draws| draws.slots < slots) {
            self.draws = Some(self.draw_buffer(draw_slots(slots)));
        }
        let Some(draws) = self.draws.as_ref() else { return };
        let matrices: Vec<f32> = behind
            .iter()
            .chain(&drawable)
            .flat_map(|(mesh, _)| {
                let model = camera.relative(mesh.to_ecef);
                (view_projection * model).as_mat4().to_cols_array().into_iter().chain(model.as_mat4().to_cols_array())
            })
            .collect();
        if !matrices.is_empty() {
            self.queue.write_buffer(&draws.buffer, 0, bytemuck::cast_slice(&matrices));
        }
        let placed = |slot: usize| u32::try_from(slot).map_or(0..0, |slot| slot..slot + 1);
        let direction = |v: glam::DVec3| v.as_vec3().extend(0.0).to_array();
        self.queue.write_buffer(&self.scene, 0, &bytes(camera.unproject().as_mat4().to_cols_array().into_iter().chain(direction(camera.zenith)).chain(direction(camera.sun))));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = Self::pass(&mut encoder, target, targets);
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);
            pass.set_vertex_buffer(1, draws.buffer.slice(..));
            let bound = Cell::new(None);
            pass.set_pipeline(&self.backfill);
            behind.iter().enumerate().for_each(|(slot, (_, gpu))| record(&mut pass, &self.arenas, &bound, gpu, placed(slot)));
            LAYERS.iter().zip(&self.pipelines).for_each(|(layer, pipeline)| {
                pass.set_pipeline(pipeline);
                drawable.iter().enumerate().filter(|(_, (mesh, _))| mesh.layer == *layer).for_each(|(slot, (_, gpu))| record(&mut pass, &self.arenas, &bound, gpu, placed(behind.len() + slot)));
            });
        }
        self.queue.submit([encoder.finish()]);
    }

    fn draw_buffer(&self, slots: usize) -> Draws {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("synthetic draws"), size: slots as u64 * DRAW_BYTES, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Draws { buffer, slots }
    }

    pub fn clear(&mut self, target: &wgpu::TextureView, targets: &Targets) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        drop(Self::pass(&mut encoder, target, targets));
        self.queue.submit([encoder.finish()]);
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }

    pub fn targets(&self, width: u32, height: u32) -> Targets {
        let attachment = |label: &str, format: wgpu::TextureFormat| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: SAMPLES,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TRANSIENT_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Targets { samples: attachment("synthetic samples", self.format), depth: attachment("synthetic depth", DEPTH) }
    }

    pub fn picture(&mut self, camera: &Camera, backfill: &[Arc<Mesh>], meshes: &[Arc<Mesh>], width: u32, height: u32) -> Result<image::RgbaImage, String> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor { label: Some("synthetic picture"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
        let targets = self.targets(width, height);
        self.draw(&target.create_view(&Default::default()), &targets, camera, backfill, meshes);
        let row = (4 * width).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: u64::from(row * height), usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &readback, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
            size,
        );
        self.queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|error| error.to_string())?;
        let mapped = readback.slice(..).get_mapped_range().map_err(|error| error.to_string())?;
        let pixels: Vec<u8> = mapped.chunks(row as usize).flat_map(|line| &line[..(4 * width) as usize]).copied().collect();
        image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| "the read-back picture had the wrong size".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DMat4;

    fn mesh() -> Arc<Mesh> {
        Arc::new(Mesh::new(DMat4::IDENTITY, Vec::new(), Vec::new(), Vec::new(), image::RgbaImage::new(1, 1), Layer::Opaque))
    }

    #[test]
    fn every_draw_gets_its_own_slot_in_one_shared_buffer() {
        assert_eq!(draw_slots(10), 1024, "the buffer starts big enough for a city");
        assert_eq!(draw_slots(1100), 2048, "and doubles rather than growing a draw at a time");
    }

    #[test]
    fn a_mesh_uploads_once_and_leaves_the_gpu_when_the_scene_drops_it() {
        let mut resident: Resident<u64> = Resident::default();
        let (kept, dropped) = (mesh(), mesh());
        assert_eq!(resident.ensure(&[kept.clone(), dropped.clone()], |mesh| mesh.id), 2);
        assert_eq!(resident.ensure(std::slice::from_ref(&kept), |mesh| mesh.id), 0, "not drawn this frame is not a reason to upload again");
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
        assert_eq!(resident.ensure(std::slice::from_ref(&second), |mesh| mesh.id), 1, "the new mesh gets its own upload");
        assert_eq!(resident.get(second.id), Some(&second.id));
        assert_eq!(resident.get(first_id), None);
    }

    #[test]
    fn meshes_share_one_arena_and_an_emptied_extra_arena_gives_its_memory_back() {
        let mut arenas: Vec<Option<Arena<()>>> = Vec::new();
        let first = place_in(&mut arenas, 100, 300, |_, _| ()).expect("room");
        let second = place_in(&mut arenas, 100, 300, |_, _| ()).expect("room");
        assert_eq!((first.arena, second.arena, second.vertices.clone()), (0, 0, 100..200), "packed after the first in the same buffers");
        let opened = Cell::new(0);
        let big = place_in(&mut arenas, ARENA_VERTICES + 1, 3, |vertices, _| opened.set(vertices)).expect("room");
        assert_eq!((big.arena, opened.get()), (1, ARENA_VERTICES + 1), "a mesh bigger than an arena gets one sized to fit");
        release(&mut arenas, big);
        close_empty(&mut arenas);
        assert!(arenas[1].is_none(), "an emptied extra arena is freed");
        assert_eq!(place_in(&mut arenas, ARENA_VERTICES, 3, |_, _| ()).map(|place| place.arena), Some(1), "and its slot is reused");
        [first, second].into_iter().for_each(|place| release(&mut arenas, place));
        close_empty(&mut arenas);
        assert!(arenas[0].is_some(), "the first arena stays for the next tiles");
    }

    #[test]
    fn an_arena_out_of_index_room_hands_back_the_vertices_it_tried() {
        let mut arenas: Vec<Option<Arena<()>>> = Vec::new();
        place_in(&mut arenas, 10, ARENA_INDICES, |_, _| ()).expect("room");
        assert_eq!(place_in(&mut arenas, 10, 3, |_, _| ()).map(|place| place.arena), Some(1));
        assert_eq!(arenas[0].as_ref().map(|arena| arena.vertex_space.total_available()), Some(ARENA_VERTICES - 10));
    }
}
