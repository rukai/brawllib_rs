use std::borrow::Cow;
use std::mem;
use std::num::NonZeroU64;

use cgmath::Matrix4;
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

pub(crate) const SAMPLE_COUNT: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    pub _pos:   [f32; 4],
    pub _color: [f32; 4],
}

pub struct WgpuState {
    pub(crate) device:                              wgpu::Device,
    pub(crate) queue:                               wgpu::Queue,
    pub(crate) _bind_group_layout:                  wgpu::BindGroupLayout,
    pub(crate) render_pipeline:                     wgpu::RenderPipeline,
    pub(crate) uniforms_buffer:                     wgpu::Buffer,
    pub(crate) bind_groups:                         Vec<wgpu::BindGroup>,
    pub(crate) multisampled_framebuffer_descriptor: wgpu::TextureDescriptor<'static>,
    pub(crate) multisampled_framebuffer:            wgpu::Texture,
}

impl WgpuState {
    /// Easy initialiser that doesnt handle rendering to a window
    pub async fn new_for_gif() -> WgpuState {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        WgpuState::new(instance, None, wgpu::TextureFormat::Rgba8Unorm).await
    }

    pub async fn new(instance: wgpu::Instance, compatible_surface: Option<&wgpu::Surface>, format: wgpu::TextureFormat) -> WgpuState {
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface,
            },
        ).await.unwrap();

        let device_descriptor = wgpu::DeviceDescriptor {
            limits: wgpu::Limits::default(),
            features: wgpu::Features::empty(),
            label: None,
        };
        let (device, queue) = adapter.request_device(&device_descriptor, None).await.unwrap();


        // shaders
        let shader_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/shader.wgsl"))),
            flags: wgpu::ShaderFlags::all(),
        });

        // layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float4,
                            offset: 0,
                        },
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * 4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: format,
                    color_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha_blend: wgpu::BlendState {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            }
        });
        let uniform_count = 1000;
        let uniform_size = mem::size_of::<Matrix4<f32>>();
        let uniform_size_padded = 256;
        // TODO: I can probably do this without the vec.
        let initial_data = vec!(0; uniform_size_padded * uniform_count);
        let uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &initial_data,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST
        });

        let mut bind_groups = vec!();
        for i in 0..uniform_count {
            let uniforms_offset = (i * uniform_size_padded) as u64;
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniforms_buffer,
                            offset: uniforms_offset,
                            size: NonZeroU64::new(uniform_size as u64),
                        },
                    },
                ],
                label: None,
            });
            bind_groups.push(bind_group);
        }

        let multisampled_framebuffer_descriptor = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width:  100,
                height: 100,
                depth:  1
            },
            mip_level_count: 1,
            sample_count: SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: format,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: None,
        };
        let multisampled_framebuffer = device.create_texture(&multisampled_framebuffer_descriptor);

        WgpuState {
            device,
            queue,
            _bind_group_layout: bind_group_layout,
            render_pipeline,
            uniforms_buffer,
            bind_groups,
            multisampled_framebuffer_descriptor,
            multisampled_framebuffer,
        }
    }

    pub fn poll(&self) {
        self.device.poll(wgpu::Maintain::Wait);
    }
}
