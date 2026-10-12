use std::collections::HashMap;
use std::sync::{Arc, Weak};

use wgpu::util::DeviceExt;

use super::geo::Camera;
use super::mesh::{Layer, Mesh, solid};

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SKY: wgpu::Color = wgpu::Color { r: 0.3, g: 0.45, b: 0.66, a: 1.0 };
const VERTEX_BYTES: u64 = 32;
const DRAW_BYTES: u64 = 128;
const SCENE_BYTES: u64 = 96;
const SAMPLES: u32 = 4;
const MISSING_TEXTURE: [u8; 4] = [255, 255, 255, 255];
const LAYERS: [Layer; 3] = [Layer::Opaque, Layer::Translucent, Layer::OnTop];

const ANISOTROPY: u16 = 16;
const SHADER: &str = r"
struct Scene { unproject: mat4x4<f32>, zenith: vec4<f32>, sun: vec4<f32> };
struct Draw { mvp: mat4x4<f32>, model: mat4x4<f32> };
@group(0) @binding(0) var<uniform> scene: Scene;
@group(1) @binding(0) var<uniform> draw: Draw;
@group(1) @binding(1) var picture: texture_2d<f32>;
@group(1) @binding(2) var tile_sampler: sampler;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32>, @location(2) normal: vec3<f32> };
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

@vertex fn vs(v: In) -> Out {
    return Out(draw.mvp * vec4<f32>(v.position, 1.0), v.uv, (draw.model * vec4<f32>(v.position, 1.0)).xyz, (draw.model * vec4<f32>(v.normal, 0.0)).xyz);
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

    pub fn evict(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, (mesh, _)| mesh.strong_count() > 0);
        before - self.entries.len()
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


struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    matrix: wgpu::Buffer,
    bind: wgpu::BindGroup,
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
    sky: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    scene: wgpu::Buffer,
    scene_bind: wgpu::BindGroup,
    sampler: wgpu::Sampler,
    resident: Resident<GpuMesh>,
    pub uploads: usize,
}

fn uniform(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }
}

fn bytes(values: impl IntoIterator<Item = f32>) -> Vec<u8> {
    values.into_iter().flat_map(f32::to_le_bytes).collect()
}

fn depth_state(write: bool, compare: wgpu::CompareFunction) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(write), depth_compare: Some(compare), stencil: Default::default(), bias: Default::default() }
}

fn multisample() -> wgpu::MultisampleState {
    wgpu::MultisampleState { count: SAMPLES, ..Default::default() }
}

fn pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule, color: wgpu::TextureFormat, layer: Layer) -> wgpu::RenderPipeline {
    let (depth_write, compare, blend, fragment) = match layer {
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
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: VERTEX_BYTES, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3] })],
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
                uniform(0, wgpu::ShaderStages::VERTEX),
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("synthetic"), bind_group_layouts: &[Some(&scene_layout), Some(&layout)], immediate_size: 0 });
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("synthetic sky"), bind_group_layouts: &[Some(&scene_layout)], immediate_size: 0 });
        let pipelines = LAYERS.map(|layer| pipeline(&device, &pipeline_layout, &shader, format, layer));
        let sky = sky_pipeline(&device, &sky_layout, &shader, format);
        let scene = device.create_buffer(&wgpu::BufferDescriptor { label: Some("synthetic scene"), size: SCENE_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let scene_bind = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("synthetic scene"), layout: &scene_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: scene.as_entire_binding() }] });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::MipmapFilterMode::Linear, address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, anisotropy_clamp: ANISOTROPY, ..Default::default() });
        Self { device, queue, format, pipelines, sky, layout, scene, scene_bind, sampler, resident: Resident::default(), uploads: 0 }
    }

    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, mesh: &Mesh) -> GpuMesh {
        let normals = mesh.normals.iter().copied().chain(std::iter::repeat([0.0; 3]));
        let vertices: Vec<u8> = bytes(mesh.positions.iter().zip(&mesh.uvs).zip(normals).flat_map(|((p, uv), n)| p.iter().chain(uv).copied().chain(n)));
        let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
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
        let matrix = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: DRAW_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: matrix.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        });
        GpuMesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &vertices, usage: wgpu::BufferUsages::VERTEX }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &indices, usage: wgpu::BufferUsages::INDEX }),
            count: mesh.indices.len() as u32,
            matrix,
            bind,
        }
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

    pub fn draw(&mut self, target: &wgpu::TextureView, targets: &Targets, camera: &Camera, meshes: &[Arc<Mesh>]) {
        let (device, queue, layout, sampler) = (&self.device, &self.queue, &self.layout, &self.sampler);
        self.uploads += self.resident.ensure(meshes, |mesh| Self::upload(device, queue, layout, sampler, mesh));
        self.resident.evict();
        let view_projection = camera.projection() * camera.view();
        let drawable: Vec<(&Arc<Mesh>, &GpuMesh)> = meshes.iter().filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?))).collect();
        drawable.iter().for_each(|(mesh, gpu)| {
            let model = camera.relative(mesh.to_ecef);
            self.queue.write_buffer(&gpu.matrix, 0, &bytes((view_projection * model).as_mat4().to_cols_array().into_iter().chain(model.as_mat4().to_cols_array())));
        });
        let direction = |v: glam::DVec3| v.as_vec3().extend(0.0).to_array();
        self.queue.write_buffer(&self.scene, 0, &bytes(camera.unproject().as_mat4().to_cols_array().into_iter().chain(direction(camera.zenith)).chain(direction(camera.sun))));
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = Self::pass(&mut encoder, target, targets);
            pass.set_bind_group(0, &self.scene_bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);
            LAYERS.iter().zip(&self.pipelines).for_each(|(layer, pipeline)| {
                pass.set_pipeline(pipeline);
                drawable.iter().filter(|(mesh, _)| mesh.layer == *layer).for_each(|(_, gpu)| {
                    pass.set_bind_group(1, &gpu.bind, &[]);
                    pass.set_vertex_buffer(0, gpu.vertices.slice(..));
                    pass.set_index_buffer(gpu.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..gpu.count, 0, 0..1);
                });
            });
        }
        self.queue.submit([encoder.finish()]);
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

    pub fn picture(&mut self, camera: &Camera, meshes: &[Arc<Mesh>], width: u32, height: u32) -> Result<image::RgbaImage, String> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor { label: Some("synthetic picture"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
        let targets = self.targets(width, height);
        self.draw(&target.create_view(&Default::default()), &targets, camera, meshes);
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
    fn a_mesh_uploads_once_and_leaves_the_gpu_when_the_scene_drops_it() {
        let mut resident: Resident<u64> = Resident::default();
        let (kept, dropped) = (mesh(), mesh());
        assert_eq!(resident.ensure(&[kept.clone(), dropped.clone()], |mesh| mesh.id), 2);
        assert_eq!(resident.ensure(std::slice::from_ref(&kept), |mesh| mesh.id), 0, "not drawn this frame is not a reason to upload again");
        drop(dropped);
        assert_eq!(resident.evict(), 1);
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
}
