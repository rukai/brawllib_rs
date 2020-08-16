use std::mem;

use cgmath::Matrix4;
use wgpu::util::DeviceExt;
use zerocopy::AsBytes;

pub(crate) const SAMPLE_COUNT: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, AsBytes)]
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
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface,
            },
        ).await.unwrap();

        let device_descriptor = wgpu::DeviceDescriptor {
            limits: wgpu::Limits::default(),
            features: wgpu::Features::empty(),
            shader_validation: false,
        };
        let (device, queue) = adapter.request_device(&device_descriptor, None).await.unwrap();


        // shaders
        let vs_module = device.create_shader_module(wgpu::include_spirv!("shaders/fighter.vert.spv"));
        let fs_module = device.create_shader_module(wgpu::include_spirv!("shaders/fighter.frag.spv"));

        // layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
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
            layout: Some(&pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: &vs_module,
                entry_point: "main",
            },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: &fs_module,
                entry_point: "main",
            }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                ..Default::default()
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: format,
                color_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha_blend: wgpu::BlendDescriptor {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[wgpu::VertexBufferDescriptor {
                    stride: mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float4,
                            offset: 0,
                        },
                        wgpu::VertexAttributeDescriptor {
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float4,
                            offset: 4 * 4,
                        },
                    ],
                }],
            },
            sample_count: SAMPLE_COUNT,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
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
                        resource: wgpu::BindingResource::Buffer(uniforms_buffer.slice(uniforms_offset..uniforms_offset+uniform_size as u64)),
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
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
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
