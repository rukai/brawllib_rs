use byteorder::{BigEndian, ByteOrder};

use crate::high_level_fighter::HighLevelFighter;

/// Blocks until user closes window
pub fn render_window(high_level_fighter: &HighLevelFighter, subaction: usize) {
    unimplemented!("{} {}", high_level_fighter.name, subaction);
}

/// Returns the bytes of a gif
pub fn render_gif(high_level_fighter: &HighLevelFighter, subaction: usize) -> Vec<u8> {
    let subaction = &high_level_fighter.subactions[subaction];
    let width: u16 = 500;
    let height: u16 = 500;
    let mut result = vec!();

    let instance = wgpu::Instance::new();
    let adapter = instance.get_adapter(&wgpu::AdapterDescriptor {
        power_preference: wgpu::PowerPreference::LowPower,
    });
    let mut device = adapter.create_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
    });

    // shaders
    let vs_bytes = include_bytes!("shaders/hello_triangle.vert.spv");
    let vs_module = device.create_shader_module(vs_bytes);
    let fs_bytes = include_bytes!("shaders/hello_triangle.frag.spv");
    let fs_module = device.create_shader_module(fs_bytes);

    // layout
    let bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { bindings: &[] });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bind_group_layout,
        bindings: &[],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::PipelineStageDescriptor {
            module: &vs_module,
            entry_point: "main",
        },
        fragment_stage: wgpu::PipelineStageDescriptor {
            module: &fs_module,
            entry_point: "main",
        },
        rasterization_state: wgpu::RasterizationStateDescriptor {
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: wgpu::CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        },
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        color_states: &[wgpu::ColorStateDescriptor {
            format: wgpu::TextureFormat::Rgba8Unorm,
            color: wgpu::BlendDescriptor::REPLACE,
            alpha: wgpu::BlendDescriptor::REPLACE,
            write_mask: wgpu::ColorWriteFlags::ALL,
        }],
        depth_stencil_state: None,
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[],
        sample_count: 1,
    });

    // DRAW!
    {
        let mut encoder = gif::Encoder::new(&mut result, width, height, &[]).unwrap();

        for _ in &subaction.frames {
            // Create buffers
            // We recreate the buffers for each frame, reusing them would mean we need to wait for stuff to finish.
            // Maybe I can implement pooling later.
            let texture_extent = wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth: 1
            };
            let framebuffer_descriptor = &wgpu::TextureDescriptor {
                size: texture_extent,
                array_size: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsageFlags::all(),
            };
            let framebuffer = device.create_texture(framebuffer_descriptor);
            let framebuffer_copy_view = wgpu::TextureCopyView {
                texture: &framebuffer,
                level: 0,
                slice: 0,
                origin: wgpu::Origin3d { x: 0.0, y: 0.0, z: 0.0 },
            };

            let framebuffer_out_usage = &wgpu::BufferDescriptor {
                size: width as u32 * height as u32 * 4,
                usage: wgpu::BufferUsageFlags::all(),
            };
            let framebuffer_out = device.create_buffer(framebuffer_out_usage);
            let framebuffer_out_copy_view = wgpu::BufferCopyView {
                buffer: &framebuffer_out,
                offset: 0,
                row_pitch: 0,
                image_height: height as u32,
            };

            // create the CommandEncoder
            println!("command_encoder");
            let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
            {
                let mut rpass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &framebuffer.create_default_view(),
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color::GREEN,
                    }],
                    depth_stencil_attachment: None,
                });
                rpass.set_pipeline(&render_pipeline);
                rpass.set_bind_group(0, &bind_group);
                rpass.draw(0..3, 0..1);
            }

            println!("copy texture to buffer");
            command_encoder.copy_texture_to_buffer(framebuffer_copy_view, framebuffer_out_copy_view, texture_extent);

            println!("submit");
            device.get_queue().submit(&[command_encoder.finish()]);

            println!("read");
            framebuffer_out.map_read_async(0, width as u32 * height as u32 * 4, |result: wgpu::BufferMapAsyncResult<&[u32]>| {
                if let wgpu::BufferMapAsyncResult::Success(data_u32) = result {
                    println!("Success");
                    let mut data_u8 = vec!(0; width as usize * height as usize * 4);
                    // uncommenting causes segfault
                    for (i, value) in data_u32.iter().enumerate() {
                        // TODO: Might need to just retain the current endianness?!?
                        BigEndian::write_u32(&mut data_u8[i * 4 ..], *value);
                        println!("{:x}", value);
                    }
                    //let gif_frame = gif::Frame::from_rgba(width as u16, height as u16, &mut data_u8);
                    //encoder.write_frame(&gif_frame).unwrap();
                }
                else {
                    println!("ERROR");
                }
            });
            std::thread::sleep_ms(1000);
        }
    }

    result
}
