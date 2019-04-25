use std::mem;
use std::sync::mpsc;
use std::f32::consts;

use byteorder::{LittleEndian, ByteOrder};
use cgmath::{Matrix4, Vector3, MetricSpace, Rad};

use crate::high_level_fighter::{HighLevelFighter, CollisionBoxValues};

#[derive(Clone, Copy)]
struct Vertex {
    _pos:   [f32; 4],
    _color: [f32; 4],
}

/// Blocks until user closes window
pub fn render_window(high_level_fighter: &HighLevelFighter, subaction: usize) {
    unimplemented!("{} {}", high_level_fighter.name, subaction);
}

/// Returns the bytes of a gif
pub fn render_gif(high_level_fighter: &HighLevelFighter, subaction: usize) -> Vec<u8> {
    let mut result = vec!();

    let width: u16 = 500;
    let height: u16 = 500;

    let subaction = &high_level_fighter.subactions[subaction];

    //let subaction_extent = subaction.hurt_box_extent();
    //let extent_middle_y = (subaction_extent.up   + subaction_extent.down) / 2.0;
    //let extent_middle_z = (subaction_extent.left + subaction_extent.right) / 2.0;
    //let extent_height = subaction_extent.up    - subaction_extent.down;
    //let extent_width  = subaction_extent.right - subaction_extent.left;
    //let extent_aspect = extent_width / extent_height;
    //let aspect = width / height;
    //let fov = 40.0;

    //let projection = cgmath::ortho(
    //    -extent_width  / 2.0,
    //    extent_width   / 2.0,
    //    -extent_height / 2.0,
    //    extent_height  / 2.0,
    //    1.0,
    //    1000.0,
    //);

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
    let vs_bytes = include_bytes!("shaders/fighter.vert.spv");
    let vs_module = device.create_shader_module(vs_bytes);
    let fs_bytes = include_bytes!("shaders/fighter.frag.spv");
    let fs_module = device.create_shader_module(fs_bytes);

    // layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        bindings: &[
            wgpu::BindGroupLayoutBinding {
                binding: 0,
                visibility: wgpu::ShaderStageFlags::VERTEX,
                ty: wgpu::BindingType::UniformBuffer,
            },
        ],
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
        vertex_buffers: &[wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Vertex>() as u32,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    attribute_index: 0,
                    format: wgpu::VertexFormat::Float4,
                    offset: 0,
                },
                wgpu::VertexAttributeDescriptor {
                    attribute_index: 1,
                    format: wgpu::VertexFormat::Float4,
                    offset: 4 * 4,
                },
            ],
        }],
        sample_count: 1,
    });

    // DRAW!
    {
        let (frames_tx, frames_rx) = mpsc::channel();

        for frame in subaction.frames.iter() {
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
            let mut vertices_vec = vec!();
            let mut indices_vec: Vec<u16> = vec!();
            let mut index_count = 0;

            vertices_vec.push(Vertex {
                _pos:   [0.0, 0.5, 0.0, 1.0],
                _color: [1.0, 1.0, 0.0, 1.0],
            });
            vertices_vec.push(Vertex {
                _pos:   [-0.5, -0.5, 0.0, 1.0],
                _color: [1.0, 1.0, 0.0, 1.0],
            });
            vertices_vec.push(Vertex {
                _pos:   [0.5, -0.5, 0.0, 1.0],
                _color: [1.0, 1.0, 0.0, 1.0],
            });

            indices_vec.push(index_count + 0);
            indices_vec.push(index_count + 1);
            indices_vec.push(index_count + 2);
            index_count += 3;

            for hitbox in frame.hit_boxes.iter() {
                // only display hitboxes that are used in regular matches
                if let CollisionBoxValues::Hit(hit_values) = &hitbox.next_values {
                    if !hit_values.enabled {
                        continue;
                    }
                }

                let _color = match hitbox.hitbox_id {
                    0 => [0.93725, 0.39216, 0.00000, 1.0], // orange
                    1 => [1.00000, 0.00000, 0.00000, 1.0], // red
                    2 => [1.00000, 0.00000, 1.00000, 1.0], // purple
                    3 => [0.09412, 0.83922, 0.78823, 1.0], // turqoise
                    4 => [0.14118, 0.83992, 0.09412, 1.0], // green
                    _ => [1.00000, 1.00000, 1.00000, 1.0], // white
                };

                let prev = hitbox.prev_pos.map(|prev| Vector3::new(prev.x, prev.y + frame.y_pos, prev.z + frame.x_pos));
                let next = Vector3::new(hitbox.next_pos.x, hitbox.next_pos.y + frame.y_pos, hitbox.next_pos.z + frame.x_pos);
                let prev_distance = prev.map(|prev| prev.distance(next)).unwrap_or(0.0);

                let width_segments = 23;
                let height_segments = 17;

                let mut grid = vec!();
                for iy in 0..height_segments+1 {
                    let mut vertices_row = vec!();
                    let v = iy as f32 / height_segments as f32;

                    for ix in 0..width_segments+1 {
                        let u = ix as f32 / width_segments as f32;
                        let mut y_offset = 0.0;
                        if v >= 0.0 && v <= 0.5 {
                            y_offset += prev_distance;
                        }

                        let sin_v_pi = (v * consts::PI).sin();
                        let _pos = [
                            hitbox.next_size * (u * consts::PI * 2.0).cos() * sin_v_pi,
                            hitbox.next_size * (v * consts::PI      ).cos() + y_offset,
                            hitbox.next_size * (u * consts::PI * 2.0).sin() * sin_v_pi,
                            1.0
                        ];
                        vertices_vec.push(Vertex {
                            _pos,
                            _color,
                        });

                        index_count += 1; // TODO: before or after push?!?!?
                        vertices_row.push(index_count);
                    }
                    grid.push(vertices_row);
                }

                for iy in 0..height_segments {
                    for ix in 0..width_segments {
                        let a = grid[iy][ix + 1];
                        let b = grid[iy][ix];
                        let c = grid[iy + 1][ix];
                        let d = grid[iy + 1][ix + 1];

                        indices_vec.extend(&[a, b, c]);
                        indices_vec.extend(&[b, c, d]);
                    }
                }
            }

            let vertices = device.create_buffer_mapped(vertices_vec.len(), wgpu::BufferUsageFlags::VERTEX)
                .fill_from_slice(&vertices_vec);

            let indices = device.create_buffer_mapped(indices_vec.len(), wgpu::BufferUsageFlags::INDEX)
                .fill_from_slice(&indices_vec);

            let transform = Matrix4::from_scale(0.05) * Matrix4::from_angle_y(Rad(consts::PI/2.0));
            //let transform = projection.clone();
            let transform: &[f32; 16] = transform.as_ref();
            let uniform_buf = device
                .create_buffer_mapped(
                    16,
                    wgpu::BufferUsageFlags::UNIFORM | wgpu::BufferUsageFlags::TRANSFER_DST,
                )
                .fill_from_slice(transform);

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
            });

            // create the CommandEncoder
            let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
            {
                let mut rpass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: &framebuffer.create_default_view(),
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
                rpass.set_pipeline(&render_pipeline);
                rpass.set_bind_group(0, &bind_group, &[]);
                rpass.set_index_buffer(&indices, 0);
                rpass.set_vertex_buffers(&[(&vertices, 0)]);
                rpass.draw_indexed(0..indices_vec.len() as u32, 0, 0..1);
            }

            command_encoder.copy_texture_to_buffer(framebuffer_copy_view, framebuffer_out_copy_view, texture_extent);

            device.get_queue().submit(&[command_encoder.finish()]);

            let frames_tx = frames_tx.clone();
            framebuffer_out.map_read_async(0, width as u32 * height as u32 * 4, move |result: wgpu::BufferMapAsyncResult<&[u32]>| {
                match result {
                    Ok(data_u32) => {
                        let mut data_u8: Vec<u8> = vec!(0; width as usize * height as usize * 4);
                        // uncommenting causes segfault
                        for (i, value) in data_u32.data.iter().enumerate() {
                            // TODO: Might need to just retain the current endianness?!?
                            LittleEndian::write_u32(&mut data_u8[i * 4 ..], *value);
                        }
                        frames_tx.send(data_u8).unwrap();
                    }
                    Err(error) => {
                        panic!("map_read_async failed: {:?}", error); // We have to panic here to avoid an infinite loop :/
                    }
                }
            });
        }

        // Needed to get the last map_read_async to run.
        device.get_queue().submit(&[]);

        let mut encoder = gif::Encoder::new(&mut result, width, height, &[]).unwrap();
        for _ in subaction.frames.iter() {
            let mut frame_data = frames_rx.recv().unwrap();
            let gif_frame = gif::Frame::from_rgba(width as u16, height as u16, &mut frame_data);
            encoder.write_frame(&gif_frame).unwrap();
        }
        encoder.write_extension(gif::ExtensionData::Repetitions(gif::Repeat::Infinite)).unwrap();
    }

    result
}
