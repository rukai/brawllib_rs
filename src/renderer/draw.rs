use std::f32::consts;

use cgmath::{Matrix4, Vector3, MetricSpace, Rad, Quaternion, SquareMatrix, InnerSpace, ElementWise};
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

pub (crate) fn draw_frame(state: &mut WgpuState, framebuffer: &wgpu::TextureView, format: wgpu::TextureFormat, width: u32, height: u32, perspective: bool, wireframe: bool, render_ecb: bool, invulnerable_type: &InvulnerableType, subaction: &HighLevelSubaction, frame_index: usize, camera: &Camera) -> wgpu::CommandEncoder {
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
            let bone_matrix = hurt_box.bone_matrix.clone();

            // extract the scale component from the bone_matrix
            let bone_scale = Vector3::new(
                bone_matrix.x.magnitude(),
                bone_matrix.y.magnitude(),
                bone_matrix.z.magnitude(),
            );

            let radius = hurt_box.hurt_box.radius;
            let stretch = hurt_box.hurt_box.stretch;
            let offset = hurt_box.hurt_box.offset;

            let stretch_face = (stretch / radius).div_element_wise(bone_scale);

            let mut vertices_vec = vec!();
            let mut indices_vec: Vec<u16> = vec!();
            let mut index_count = 0;

            let mut width_segments = 23; // needs to be odd, so we have a middle segment
            let mut height_segments = 17; // needs to be odd, so we have a middle segment

            // Make the wireframes less busy in wireframe mode
            if wireframe {
                width_segments = 11;
                height_segments = 7;
            }

            let _color = if hurt_box.state.is_intangible() {
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

            let mut grid = vec!();
            // modified UV sphere generation from:
            // https://github.com/mrdoob/THREE.js/blob/4ca3860851d0cd33535afe801a1aa856da277f3a/src/geometries/SphereGeometry.js
            for iy in 0..height_segments+1 {
                let mut vertices_row = vec!();
                let v = iy as f32 / height_segments as f32;

                for ix in 0..width_segments+1 {
                    let u = ix as f32 / width_segments as f32;

                    // The x, y and z stretch values, split the sphere in half, across its dimension.
                    // This can result in 8 individual sphere corners.
                    let mut corner_offset = Vector3::new(0.0, 0.0, 0.0);
                    if u >= 0.25 && u <= 0.75 { // X
                        if stretch.x > 0.0 {
                            corner_offset.x = stretch_face.x;
                        }
                    }
                    else if stretch.x < 0.0 {
                        corner_offset.x = stretch_face.x;
                    }

                    if v >= 0.0 && v <= 0.5 { // Y
                        if stretch.y > 0.0 {
                            corner_offset.y = stretch_face.y;
                        }
                    }
                    else if stretch.y < 0.0 {
                        corner_offset.y = stretch_face.y;
                    }

                    if u >= 0.0 && u <= 0.5 { // Z
                        if stretch.z > 0.0 {
                            corner_offset.z = stretch_face.z;
                        }
                    }
                    else if stretch.z < 0.0 {
                        corner_offset.z = stretch_face.z;
                    }

                    // vertex generation is supposed have the 8 sphere corners take up exactly 1/8th of the unit sphere.
                    // However that is difficult because we would need to double up the middle segments.
                    // So instead we just make it look like this is the case by having large width_segments and height_segments.
                    let sin_v_pi = (v * consts::PI).sin();
                    let _pos = [
                        corner_offset.x - (u * consts::PI * 2.0).cos() * sin_v_pi,
                        corner_offset.y + (v * consts::PI).cos(),
                        corner_offset.z + (u * consts::PI * 2.0).sin() * sin_v_pi,
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
                    let a = grid[iy][(ix + 1) % width_segments];
                    let b = grid[iy][ix];
                    let c = grid[iy + 1][ix];
                    let d = grid[iy + 1][(ix + 1) % width_segments];

                    indices_vec.extend(&[a, b, d]);
                    indices_vec.extend(&[b, c, d]);
                }
            }

            let vertices = state.device.create_buffer_with_data(vertices_vec.as_bytes(), wgpu::BufferUsage::VERTEX);
            let indices = state.device.create_buffer_with_data(indices_vec.as_bytes(), wgpu::BufferUsage::INDEX);

            let transform_translation = Matrix4::from_translation(offset.div_element_wise(bone_scale * radius));
            let transform_scale = Matrix4::from_scale(radius);
            let model = transform_translation_frame * bone_matrix * transform_scale * transform_translation;

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
                        resource: wgpu::BindingResource::Buffer(uniform_buf.slice(..))
                    },
                ],
                label: None,
            });

            let indices_len = indices_vec.len();
            draws.push(Draw { bind_group, vertices, indices, indices_len });
        }

        for hitbox in frame.hit_boxes.iter() {
            // only display hitboxes that are used in regular matches
            if let CollisionBoxValues::Hit(hit_values) = &hitbox.next_values {
                if !hit_values.enabled {
                    continue;
                }
            }

            let _color = match hitbox.hitbox_id {
                0 => [0.93725, 0.39216, 0.00000, 0.3], // orange
                1 => [1.00000, 0.00000, 0.00000, 0.3], // red
                2 => [1.00000, 0.00000, 1.00000, 0.3], // purple
                3 => [0.09412, 0.83922, 0.78823, 0.3], // turqoise
                4 => [0.14118, 0.83992, 0.09412, 0.3], // green
                _ => [1.00000, 1.00000, 1.00000, 0.3], // white
            };

            let prev = hitbox.prev_pos.map(|prev| Vector3::new(prev.x, prev.y + frame.y_pos, prev.z + frame.x_pos));
            let next = Vector3::new(hitbox.next_pos.x, hitbox.next_pos.y + frame.y_pos, hitbox.next_pos.z + frame.x_pos);
            let prev_distance = prev.map(|prev| prev.distance(next)).unwrap_or(0.0);

            let width_segments = 23;
            let height_segments = 17;

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
                        hitbox.next_size * (u * consts::PI * 2.0).cos() * sin_v_pi,
                        hitbox.next_size * (v * consts::PI      ).cos() + y_offset,
                        hitbox.next_size * (u * consts::PI * 2.0).sin() * sin_v_pi,
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

            let rotation = if let Some(prev) = prev {
                let diff = (prev - next).normalize();
                if diff.x.is_nan() {
                    // This occurs when prev == next
                    Matrix4::identity()
                } else {
                    let source_angle = Vector3::new(0.0, 1.0, 0.0);
                    Quaternion::from_arc(source_angle, diff, None).into()
                }
            } else {
                Matrix4::identity()
            };
            let model = Matrix4::from_translation(next) * rotation;
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
