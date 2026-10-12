use std::collections::HashMap;
use std::sync::Arc;

use wgpu::util::DeviceExt;

use super::geo::Camera;
use super::glb::Mesh;

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SKY: wgpu::Color = wgpu::Color { r: 0.25, g: 0.47, b: 0.83, a: 1.0 };
const VERTEX_BYTES: u64 = 20;
const MATRIX_BYTES: u64 = 64;

const SHADER: &str = r"
struct Draw { mvp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> draw: Draw;
@group(0) @binding(1) var picture: texture_2d<f32>;
@group(0) @binding(2) var tile_sampler: sampler;
struct In { @location(0) position: vec3<f32>, @location(1) uv: vec2<f32> };
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(v: In) -> Out { return Out(draw.mvp * vec4<f32>(v.position, 1.0), v.uv); }
@fragment fn fs(f: Out) -> @location(0) vec4<f32> { return textureSample(picture, tile_sampler, f.uv); }
";

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    matrix: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    meshes: HashMap<usize, GpuMesh>,
}

fn key(mesh: &Arc<Mesh>) -> usize {
    Arc::as_ptr(mesh) as usize
}

impl Renderer {
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() })).ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).ok()?;
        Some(Self::with_device(device, queue, COLOR))
    }

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue, color: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("photoreal"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("photoreal draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("photoreal"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("photoreal"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: VERTEX_BYTES, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2] })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Greater), stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(color.into())] }),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        Self { device, queue, pipeline, layout, sampler, meshes: HashMap::new() }
    }

    fn upload(&self, mesh: &Mesh) -> GpuMesh {
        let vertices: Vec<u8> = mesh.positions.iter().zip(&mesh.uvs).flat_map(|(p, uv)| p.iter().chain(uv).flat_map(|v| v.to_le_bytes())).collect();
        let indices: Vec<u8> = mesh.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let (width, height) = mesh.texture.dimensions();
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor { label: None, size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            mesh.texture.as_raw(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: Some(height) },
            size,
        );
        let view = texture.create_view(&Default::default());
        let matrix = self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: MATRIX_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: matrix.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        GpuMesh {
            vertices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &vertices, usage: wgpu::BufferUsages::VERTEX }),
            indices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: &indices, usage: wgpu::BufferUsages::INDEX }),
            count: mesh.indices.len() as u32,
            matrix,
            bind,
        }
    }

    pub fn draw(&mut self, target: &wgpu::TextureView, depth: &wgpu::TextureView, camera: &Camera, meshes: &[Arc<Mesh>]) {
        let missing: Vec<(usize, GpuMesh)> = meshes.iter().filter(|mesh| !self.meshes.contains_key(&key(mesh))).map(|mesh| (key(mesh), self.upload(mesh))).collect();
        self.meshes.extend(missing);
        let wanted: std::collections::HashSet<usize> = meshes.iter().map(key).collect();
        self.meshes.retain(|k, _| wanted.contains(k));
        let view_projection = camera.projection() * camera.view();
        meshes.iter().for_each(|mesh| {
            let mvp = (view_projection * camera.relative(mesh.to_ecef)).as_mat4();
            self.queue.write_buffer(&self.meshes[&key(mesh)].matrix, 0, &mvp.to_cols_array().iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>());
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("photoreal"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(SKY), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: depth, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            meshes.iter().map(|mesh| &self.meshes[&key(mesh)]).for_each(|gpu| {
                pass.set_bind_group(0, &gpu.bind, &[]);
                pass.set_vertex_buffer(0, gpu.vertices.slice(..));
                pass.set_index_buffer(gpu.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..gpu.count, 0, 0..1);
            });
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn depth(&self, width: u32, height: u32) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor { label: Some("photoreal depth"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: DEPTH, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] })
            .create_view(&Default::default())
    }

    pub fn picture(&mut self, camera: &Camera, meshes: &[Arc<Mesh>], width: u32, height: u32) -> Option<image::RgbaImage> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor { label: Some("photoreal picture"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
        let depth = self.depth(width, height);
        self.draw(&target.create_view(&Default::default()), &depth, camera, meshes);
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
        self.device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        let mapped = readback.slice(..).get_mapped_range().ok()?;
        let pixels: Vec<u8> = mapped.chunks(row as usize).flat_map(|line| &line[..(4 * width) as usize]).copied().collect();
        image::RgbaImage::from_raw(width, height, pixels)
    }
}
