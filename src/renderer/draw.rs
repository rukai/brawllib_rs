use std::f32::consts;

use cgmath::{Matrix4, Vector3, MetricSpace, Rad, Quaternion, SquareMatrix, InnerSpace};
use zerocopy::AsBytes;

use crate::high_level_fighter::{HighLevelSubaction, CollisionBoxValues};
use crate::renderer::camera::Camera;
use crate::renderer::app::state::InvulnerableType;
use crate::renderer::wgpu_state::{WgpuState, SAMPLE_COUNT, Vertex};

struct Draw {
    bind_group:  wgpu::BindGroup,
    vertices:    wgpu::Buffer,
    indices:     wgpu::Buffer,
    indices_len: usize,
}

pub (crate) fn draw_frame(state: &WgpuState, framebuffer: &wgpu::TextureView, format: wgpu::TextureFormat, width: u32, height: u32, perspective: bool, wireframe: bool, render_ecb: bool, invulnerable_type: &InvulnerableType, subaction: &HighLevelSubaction, frame_index: usize, camera: &Camera) -> wgpu::CommandEncoder {
    let mut command_encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut draws: Vec<Draw> = vec!();

    let multisampled_texture_extent = wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth: 1
    };
    let multisampled_framebuffer_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
        label: None,
    };

    let multisampled_framebuffer = state.device.create_texture(multisampled_framebuffer_descriptor);

    {
        let attachment = multisampled_framebuffer.create_default_view();
        let mut rpass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: if SAMPLE_COUNT == 1 { framebuffer } else { &attachment },
                resolve_target: if SAMPLE_COUNT == 1 { None } else { Some(framebuffer) },
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
        rpass.set_pipeline(&state.render_pipeline);

        let frame = &subaction.frames[frame_index];

        // TODO: Should this stuff go in the camera? Lets not duplicate it...
        let mut subaction_extent = subaction.hurt_box_extent();
        subaction_extent.extend(&subaction.hit_box_extent());
        subaction_extent.extend(&subaction.ledge_grab_box_extent());

        let extent_height = subaction_extent.up    - subaction_extent.down;
        let extent_width  = subaction_extent.right - subaction_extent.left;
        let extent_aspect = extent_width / extent_height;
        let aspect = width as f32 / height as f32;
        let fov = 40.0;

        let fov_rad = fov * consts::PI / 180.0;

        let camera_offset = Vector3::new(
            camera.radius * camera.phi.sin() * camera.theta.sin(),
            camera.radius * camera.phi.cos(),
            camera.radius * camera.phi.sin() * camera.theta.cos(),
        );
        let camera_location = camera.target + camera_offset;
        let view = Matrix4::look_at(camera_location, camera.target, Vector3::new(0.0, 1.0, 0.0));

        let projection = if perspective {
            cgmath::perspective(
                Rad(fov_rad),
                aspect,
                1.0,
                1000.0,
            )
        } else {
            let mut height = extent_height;
            let mut width = extent_width;

            if extent_aspect > aspect {
                height = width / aspect;
            }
            else {
                width = height * aspect;
            }

            cgmath::ortho(
                -width  / 2.0,
                width   / 2.0,
                -height / 2.0,
                height  / 2.0,
                -1000.0,
                1000.0,
            )
        };

        let transform_translation_frame = Matrix4::from_translation(Vector3::new(
            0.0,
            frame.y_pos,
            frame.x_pos,
        ));

        for hurt_box in &frame.hurt_boxes {
            let color = if hurt_box.state.is_intangible() {
                [0.0, 0.0, 1.0, 0.3]
            } else if hurt_box.state.is_invincible() {
                [0.0, 1.0, 0.0, 0.3]
            } else {
                match invulnerable_type {
                    InvulnerableType::Hit => [1.0, 1.0, 0.0, 0.3],
                    InvulnerableType::Grab => if hurt_box.hurt_box.grabbable {
                        [1.0, 1.0, 0.0, 0.3]
                    } else {
                        [0.0, 0.0, 1.0, 0.3]
                    }
                    InvulnerableType::TrapItem => if hurt_box.hurt_box.trap_item_hittable {
                        [1.0, 1.0, 0.0, 0.3]
                    } else {
                        [0.0, 0.0, 1.0, 0.3]
                    }
                }
            };

            // Ah ... so its less of an offset + stretch and more like two seperate independent offsets.
            let prev = hurt_box.hurt_box.offset;
            let next = hurt_box.hurt_box.stretch;
            let radius = hurt_box.hurt_box.radius;

            let model = transform_translation_frame * hurt_box.bone_matrix;
            let transform = projection.clone() * view.clone() * model;
            draws.push(draw_cylinder(state, prev, next, radius, transform, color, wireframe));
        }

        for hitbox in frame.hit_boxes.iter() {
            // only display hitboxes that are used in regular matches
            if let CollisionBoxValues::Hit(hit_values) = &hitbox.next_values {
                if !hit_values.enabled {
                    continue;
                }
            }

            let color = match hitbox.hitbox_id {
                0 => [0.93725, 0.39216, 0.00000, 0.3], // orange
                1 => [1.00000, 0.00000, 0.00000, 0.3], // red
                2 => [1.00000, 0.00000, 1.00000, 0.3], // purple
                3 => [0.09412, 0.83922, 0.78823, 0.3], // turqoise
                4 => [0.14118, 0.83992, 0.09412, 0.3], // green
                _ => [1.00000, 1.00000, 1.00000, 0.3], // white
            };

            let next = Vector3::new(hitbox.next_pos.x, hitbox.next_pos.y + frame.y_pos, hitbox.next_pos.z + frame.x_pos);
            let prev = hitbox.prev_pos.map(|prev| Vector3::new(prev.x, prev.y + frame.y_pos, prev.z + frame.x_pos)).unwrap_or(next.clone());
            let radius = hitbox.next_size;
            let transform = projection.clone() * view.clone();
            draws.push(draw_cylinder(state, prev, next, radius, transform, color, wireframe));
        }

        if let Some(ref ledge_grab_box) = frame.ledge_grab_box {
            let _color = [1.0, 1.0, 1.0, 0.5];
            let vertices_array = [
                Vertex { _pos: [0.0, ledge_grab_box.up,   ledge_grab_box.left,  1.0], _color },
                Vertex { _pos: [0.0, ledge_grab_box.up,   ledge_grab_box.right, 1.0], _color },
                Vertex { _pos: [0.0, ledge_grab_box.down, ledge_grab_box.left,  1.0], _color },
                Vertex { _pos: [0.0, ledge_grab_box.down, ledge_grab_box.right, 1.0], _color },
            ];

            let indices_array: [u16; 6] = [
                0, 1, 2,
                1, 2, 3,
            ];

            let vertices = state.device.create_buffer_with_data(vertices_array.as_bytes(), wgpu::BufferUsage::VERTEX);
            let indices = state.device.create_buffer_with_data(indices_array.as_bytes(), wgpu::BufferUsage::INDEX);

            let model = Matrix4::from_translation(Vector3::new(0.0, frame.y_pos, frame.x_pos));
            let transform = projection.clone() * view.clone() * model;
            let transform: &[f32; 16] = transform.as_ref();
            let uniform_buf = state.device.create_buffer_with_data(
                transform.as_bytes(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            );

            let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &state.bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buf.slice(..)),
                    },
                ],
                label: None,
            });

            let indices_len = indices_array.len();
            draws.push(Draw { bind_group, vertices, indices, indices_len });
        }

        if render_ecb {
            // transN
            let _color = if frame.ecb.transn_y == frame.ecb.bottom {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 1.0, 1.0, 1.0]
            };

            let mut vertices_vec: Vec<Vertex> = vec!();
            let mut indices_vec: Vec<u16> = vec!();

            let iterations = 40;
            vertices_vec.push(Vertex { _pos: [0.0, 0.0, 0.0, 1.0], _color });
            for i in 0..iterations {
                let angle = i as f32 * 2.0 * consts::PI / (iterations as f32);
                let (sin, cos) = angle.sin_cos();
                let x = cos * 0.3;
                let y = sin * 0.3;
                vertices_vec.push(Vertex { _pos: [0.0, y, x, 1.0], _color });
                indices_vec.push(0);
                indices_vec.push(i + 1);
                indices_vec.push((i + 1) % iterations + 1);
            }

            let vertices = state.device.create_buffer_with_data(vertices_vec.as_bytes(), wgpu::BufferUsage::VERTEX);
            let indices = state.device.create_buffer_with_data(indices_vec.as_bytes(), wgpu::BufferUsage::INDEX);

            let model = Matrix4::from_translation(Vector3::new(
                0.0,
                frame.y_pos + frame.ecb.transn_y,
                frame.x_pos + frame.ecb.transn_x,
            ));
            let transform = projection.clone() * view.clone() * model;
            let transform: &[f32; 16] = transform.as_ref();
            let uniform_buf = state.device.create_buffer_with_data(
                transform.as_bytes(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            );

            let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &state.bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buf.slice(..)),
                    },
                ],
                label: None,
            });

            let indices_len = indices_vec.len();
            draws.push(Draw { bind_group, vertices, indices, indices_len });

            // ECB
            let _color = [0.945, 0.361, 0.0392, 1.0];
            let mid_y = (frame.ecb.top + frame.ecb.bottom) / 2.0;
            let vertices_array = [
                Vertex { _pos: [0.0, frame.ecb.top,    0.0,             1.0], _color },
                Vertex { _pos: [0.0, mid_y,            frame.ecb.left,  1.0], _color },
                Vertex { _pos: [0.0, mid_y,            frame.ecb.right, 1.0], _color },
                Vertex { _pos: [0.0, frame.ecb.bottom, 0.0,             1.0], _color },
            ];

            let indices_array: [u16; 6] = [
                0, 1, 2,
                1, 2, 3,
            ];

            let vertices = state.device.create_buffer_with_data(vertices_array.as_bytes(), wgpu::BufferUsage::VERTEX);
            let indices = state.device.create_buffer_with_data(indices_array.as_bytes(), wgpu::BufferUsage::INDEX);

            let model = Matrix4::from_translation(Vector3::new(0.0, frame.y_pos, frame.x_pos));
            let transform = projection.clone() * view.clone() * model;
            let transform: &[f32; 16] = transform.as_ref();
            let uniform_buf = state.device.create_buffer_with_data(
                transform.as_bytes(),
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            );

            let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &state.bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(uniform_buf.slice(..)),
                    },
                ],
                label: None,
            });

            let indices_len = indices_array.len();
            draws.push(Draw { bind_group, vertices, indices, indices_len });
        }

        for draw in &draws {
            rpass.set_bind_group(0, &draw.bind_group, &[]);
            rpass.set_index_buffer(draw.indices.slice(..));
            rpass.set_vertex_buffer(0, draw.vertices.slice(..));
            rpass.draw_indexed(0..draw.indices_len as u32, 0, 0..1);
        }
    }

    command_encoder
}

fn draw_cylinder(state: &WgpuState, prev: Vector3<f32>, next: Vector3<f32>, radius: f32, external_transform: Matrix4<f32>, _color: [f32; 4], wireframe: bool) -> Draw {
    let prev_distance = prev.distance(next);

    // Make the wireframes less busy in wireframe mode
    let (width_segments, height_segments) = if wireframe {
        (11, 7)
    } else {
        (23, 17)
    };

    let mut vertices_vec = vec!();
    let mut indices_vec: Vec<u16> = vec!();
    let mut index_count = 0;

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
                radius * (u * consts::PI * 2.0).cos() * sin_v_pi,
                radius * (v * consts::PI      ).cos() + y_offset,
                radius * (u * consts::PI * 2.0).sin() * sin_v_pi,
                1.0
            ];
            vertices_vec.push(Vertex { _pos, _color });

            vertices_row.push(index_count);
            index_count += 1;
        }
        grid.push(vertices_row);
    }

    for iy in 0..height_segments {
        for ix in 0..width_segments {
            let a = grid[iy][ix + 1];
            let b = grid[iy][ix];
            let c = grid[iy + 1][ix];
            let d = grid[iy + 1][ix + 1];

            indices_vec.extend(&[a, b, d]);
            indices_vec.extend(&[b, c, d]);
        }
    }

    let vertices = state.device.create_buffer_with_data(vertices_vec.as_bytes(), wgpu::BufferUsage::VERTEX);
    let indices = state.device.create_buffer_with_data(indices_vec.as_bytes(), wgpu::BufferUsage::INDEX);

    let diff = (prev - next).normalize();
    let rotation = if diff.x.is_nan() {
        // This occurs when prev == next
        Matrix4::identity()
    } else {
        let source_angle = Vector3::new(0.0, 1.0, 0.0);
        Quaternion::from_arc(source_angle, diff, None).into()
    };
    let transform = external_transform * Matrix4::from_translation(next) * rotation;
    let transform: &[f32; 16] = transform.as_ref();
    let uniform_buf = state.device.create_buffer_with_data(
        transform.as_bytes(),
        wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    );

    let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &state.bind_group_layout,
        bindings: &[
            wgpu::Binding {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buf.slice(..)),
            },
        ],
        label: None,
    });

    let indices_len = indices_vec.len();
    Draw { bind_group, vertices, indices, indices_len }
}
