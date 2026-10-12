use std::collections::HashMap;
use std::sync::{Arc, Weak};

use wgpu::util::DeviceExt;

use super::geo::Camera;
use super::mesh::{Layer, Mesh, solid};

const COLOR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SKY: wgpu::Color = wgpu::Color { r: 0.25, g: 0.47, b: 0.83, a: 1.0 };
const VERTEX_BYTES: u64 = 20;
const MATRIX_BYTES: u64 = 64;
const MISSING_TEXTURE: [u8; 4] = [255, 255, 255, 255];
const LAYERS: [Layer; 3] = [Layer::Opaque, Layer::Translucent, Layer::OnTop];

const ANISOTROPY: u16 = 16;
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

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipelines: [wgpu::RenderPipeline; 3],
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    resident: Resident<GpuMesh>,
    pub uploads: usize,
}

fn pipeline(device: &wgpu::Device, layout: &wgpu::PipelineLayout, shader: &wgpu::ShaderModule, color: wgpu::TextureFormat, layer: Layer) -> wgpu::RenderPipeline {
    let (depth_write, compare, blend) = match layer {
        Layer::Opaque => (true, wgpu::CompareFunction::Greater, None),
        Layer::Translucent => (false, wgpu::CompareFunction::Greater, Some(wgpu::BlendState::ALPHA_BLENDING)),
        Layer::OnTop => (false, wgpu::CompareFunction::Always, Some(wgpu::BlendState::ALPHA_BLENDING)),
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("synthetic"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout { array_stride: VERTEX_BYTES, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2] })],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(depth_write), depth_compare: Some(compare), stencil: Default::default(), bias: Default::default() }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: color, blend, write_mask: wgpu::ColorWrites::ALL })] }),
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

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue, color: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("synthetic"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic draw"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("synthetic"), bind_group_layouts: &[Some(&layout)], immediate_size: 0 });
        let pipelines = LAYERS.map(|layer| pipeline(&device, &pipeline_layout, &shader, color, layer));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::MipmapFilterMode::Linear, address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, anisotropy_clamp: ANISOTROPY, ..Default::default() });
        Self { device, queue, pipelines, layout, sampler, resident: Resident::default(), uploads: 0 }
    }

    fn upload(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, mesh: &Mesh) -> GpuMesh {
        let vertices: Vec<u8> = mesh.positions.iter().zip(&mesh.uvs).flat_map(|(p, uv)| p.iter().chain(uv).flat_map(|v| v.to_le_bytes())).collect();
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
        let matrix = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: MATRIX_BYTES, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
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

    fn pass<'a>(encoder: &'a mut wgpu::CommandEncoder, target: &wgpu::TextureView, depth: &wgpu::TextureView) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("synthetic"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(SKY), store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: depth, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
            ..Default::default()
        })
    }

    pub fn draw(&mut self, target: &wgpu::TextureView, depth: &wgpu::TextureView, camera: &Camera, meshes: &[Arc<Mesh>]) {
        let (device, queue, layout, sampler) = (&self.device, &self.queue, &self.layout, &self.sampler);
        self.uploads += self.resident.ensure(meshes, |mesh| Self::upload(device, queue, layout, sampler, mesh));
        self.resident.evict();
        let view_projection = camera.projection() * camera.view();
        let drawable: Vec<(&Arc<Mesh>, &GpuMesh)> = meshes.iter().filter_map(|mesh| Some((mesh, self.resident.get(mesh.id)?))).collect();
        drawable.iter().for_each(|(mesh, gpu)| {
            let mvp = (view_projection * camera.relative(mesh.to_ecef)).as_mat4();
            self.queue.write_buffer(&gpu.matrix, 0, &mvp.to_cols_array().iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<u8>>());
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = Self::pass(&mut encoder, target, depth);
            LAYERS.iter().zip(&self.pipelines).for_each(|(layer, pipeline)| {
                pass.set_pipeline(pipeline);
                drawable.iter().filter(|(mesh, _)| mesh.layer == *layer).for_each(|(_, gpu)| {
                    pass.set_bind_group(0, &gpu.bind, &[]);
                    pass.set_vertex_buffer(0, gpu.vertices.slice(..));
                    pass.set_index_buffer(gpu.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..gpu.count, 0, 0..1);
                });
            });
        }
        self.queue.submit([encoder.finish()]);
    }

    pub fn clear(&mut self, target: &wgpu::TextureView, depth: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        drop(Self::pass(&mut encoder, target, depth));
        self.queue.submit([encoder.finish()]);
    }

    pub fn present(&self, frame: wgpu::SurfaceTexture) {
        self.queue.present(frame);
    }

    pub fn depth(&self, width: u32, height: u32) -> wgpu::TextureView {
        self.device
            .create_texture(&wgpu::TextureDescriptor { label: Some("synthetic depth"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: DEPTH, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] })
            .create_view(&Default::default())
    }

    pub fn picture(&mut self, camera: &Camera, meshes: &[Arc<Mesh>], width: u32, height: u32) -> Result<image::RgbaImage, String> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let target = self.device.create_texture(&wgpu::TextureDescriptor { label: Some("synthetic picture"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: COLOR, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
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
