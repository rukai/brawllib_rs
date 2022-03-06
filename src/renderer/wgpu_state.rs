use std::borrow::Cow;
use std::mem;
use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use cgmath::Matrix4;
use wgpu::util::DeviceExt;

// TODO: Detect by capability or something
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const SAMPLE_COUNT: u32 = 4;
#[cfg(target_arch = "wasm32")]
pub(crate) const SAMPLE_COUNT: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub _pos: [f32; 4],
    pub _color: [f32; 4],
}
pub struct WgpuState {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) _bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) render_pipeline_fill: wgpu::RenderPipeline,
    pub(crate) render_pipeline_line: wgpu::RenderPipeline,
    pub(crate) uniforms_buffer: wgpu::Buffer,
    pub(crate) bind_groups: Vec<wgpu::BindGroup>,
    pub(crate) multisampled_framebuffer_descriptor: wgpu::TextureDescriptor<'static>,
    pub(crate) multisampled_framebuffer: wgpu::Texture,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) background_color: wgpu::Color,
}

pub enum CompatibleSurface<'a> {
    Surface(&'a wgpu::Surface),
    Headless(wgpu::TextureFormat),
}

impl WgpuState {
    /// Easy initialiser that doesnt handle rendering to a window
    pub async fn new_for_gif() -> WgpuState {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        WgpuState::new(
            instance,
            CompatibleSurface::Headless(wgpu::TextureFormat::Rgba8UnormSrgb),
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
        .await
    }

    pub async fn new(
        instance: wgpu::Instance,
        compatible_surface: CompatibleSurface<'_>,
        background_color: wgpu::Color,
    ) -> WgpuState {
        let surface = match compatible_surface {
            CompatibleSurface::Surface(surface) => Some(surface),
            CompatibleSurface::Headless(_) => None,
        };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let format = match compatible_surface {
            CompatibleSurface::Surface(surface) => surface.get_preferred_format(&adapter).unwrap(),
            CompatibleSurface::Headless(format) => format,
        };

        // Once we move to webgpu backend instead of webgl we can enable this
        #[cfg(not(target_arch = "wasm32"))]
        let features = wgpu::Features::empty().union(wgpu::Features::POLYGON_MODE_LINE);
        #[cfg(target_arch = "wasm32")]
        let features = wgpu::Features::empty();

        let device_descriptor = wgpu::DeviceDescriptor {
            limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            features,
            label: None,
        };
        let (device, queue) = adapter
            .request_device(&device_descriptor, None)
            .await
            .unwrap();

        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/shader.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline_line = WgpuState::render_pipeline(
            &device,
            format,
            &shader_module,
            &pipeline_layout,
            // Once we move to webgpu backend instead of webgl we can enable this
            #[cfg(not(target_arch = "wasm32"))]
            wgpu::PolygonMode::Line,
            #[cfg(target_arch = "wasm32")]
            wgpu::PolygonMode::Fill,
        );
        let render_pipeline_fill = WgpuState::render_pipeline(
            &device,
            format,
            &shader_module,
            &pipeline_layout,
            wgpu::PolygonMode::Fill,
        );
        const UNIFORM_SIZE: usize = mem::size_of::<Matrix4<f32>>();
        const UNIFORM_COUNT: usize = 1000;
        const UNIFORM_SIZE_PADDED: usize = 256;
        let initial_data = [0; UNIFORM_SIZE_PADDED * UNIFORM_COUNT];
        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &initial_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut bind_groups = vec![];
        for i in 0..UNIFORM_COUNT {
            let uniforms_offset = (i * UNIFORM_SIZE_PADDED) as u64;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms_buffer,
                        offset: uniforms_offset,
                        size: NonZeroU64::new(UNIFORM_SIZE as u64),
                    }),
                }],
                label: None,
            });
            bind_groups.push(bind_group);
        }

        let multisampled_framebuffer_descriptor = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: None,
        };
        let multisampled_framebuffer = device.create_texture(&multisampled_framebuffer_descriptor);

        WgpuState {
            device,
            queue,
            _bind_group_layout: bind_group_layout,
            render_pipeline_fill,
            render_pipeline_line,
            uniforms_buffer,
            bind_groups,
            multisampled_framebuffer_descriptor,
            multisampled_framebuffer,
            format,
            background_color,
        }
    }

    fn render_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        shader_module: &wgpu::ShaderModule,
        pipeline_layout: &wgpu::PipelineLayout,
        polygon_mode: wgpu::PolygonMode,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader_module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 0,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 4 * 4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        })
    }

    pub fn poll(&self) {
        self.device.poll(wgpu::Maintain::Wait);
    }
}
