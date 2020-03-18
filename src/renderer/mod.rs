use std::mem;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::f32::consts;
use std::thread;

use cgmath::{Matrix4, Vector3, Point3, MetricSpace, Rad, Quaternion, SquareMatrix, InnerSpace, ElementWise};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::Event;
use winit_input_helper::WinitInputHelper;
use zerocopy::AsBytes;

use crate::high_level_fighter::{HighLevelFighter, HighLevelSubaction, CollisionBoxValues};

mod app;
mod camera;

use app::{AppState, InvulnerableType};
use camera::Camera;

#[repr(C)]
#[derive(Clone, Copy, AsBytes)]
struct Vertex {
    _pos:   [f32; 4],
    _color: [f32; 4],
}

#[derive(PartialEq)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

const SAMPLE_COUNT: u32 = 8;

fn new_camera(subaction: &HighLevelSubaction, width: u16, height: u16) -> Camera {
    let mut subaction_extent = subaction.hurt_box_extent();
    subaction_extent.extend(&subaction.hit_box_extent());
    subaction_extent.extend(&subaction.ledge_grab_box_extent());

    let extent_middle_y = (subaction_extent.up   + subaction_extent.down) / 2.0;
    let extent_middle_z = (subaction_extent.left + subaction_extent.right) / 2.0;
    let extent_height = subaction_extent.up    - subaction_extent.down;
    let extent_width  = subaction_extent.right - subaction_extent.left;
    let extent_aspect = extent_width / extent_height;
    let aspect = width as f32 / height as f32;

    let radius = (subaction_extent.up - extent_middle_y).max(subaction_extent.right - extent_middle_z);
    let fov = 40.0;
    let fov_rad = fov * consts::PI / 180.0;

    let mut camera_distance = radius / (fov_rad / 2.0).tan();

    // This logic probably only works because this.pixel_width >= this.pixel_height is always true
    if extent_aspect > aspect {
        camera_distance /= aspect;
    }
    else if extent_width > extent_height {
        camera_distance /= extent_aspect;
    }

    let target = Point3::new(0.0, extent_middle_y, extent_middle_z);

    Camera {
        target,
        radius: camera_distance,
        phi: std::f32::consts::PI / 2.0,
        theta: std::f32::consts::PI * 3.0 / 2.0,
    }
}

/// Glues together:
/// *   AppState: All application logic goes in here
/// *   WgpuState: All rendering logic goes in here
/// *   Other bits and pieces missing from WgpuState because they aren't needed for rendering to GIF.
struct App {
    wgpu_state: WgpuState,
    app_state: AppState,
    input: WinitInputHelper,
    _window: Window,
    surface: wgpu::Surface,
    swap_chain: wgpu::SwapChain,
    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    high_level_fighter: HighLevelFighter,
    subaction_index: usize,
}

impl App {
    fn new(event_loop: &EventLoop<()>, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> App {
        let input = WinitInputHelper::new();

        let wgpu_state = WgpuState::new();

        let _window = Window::new(&event_loop).unwrap();
        let size = _window.inner_size();

        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            present_mode: wgpu::PresentMode::Fifo,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8Unorm,
            width: size.width,
            height: size.height,
        };

        let surface = wgpu::Surface::create(&_window);
        let swap_chain = wgpu_state.device.create_swap_chain(&surface, &swap_chain_descriptor);

        let subaction = &high_level_fighter.subactions[subaction_index];

        let camera = new_camera(
            subaction,
            swap_chain_descriptor.width as u16,
            swap_chain_descriptor.height as u16,
        );
        let app_state = AppState::new(camera);

        let high_level_fighter = high_level_fighter.clone();

        App { wgpu_state, app_state, input, _window, surface, swap_chain, swap_chain_descriptor, high_level_fighter, subaction_index }
    }

    fn update(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if self.input.update(event) {
            if self.input.quit() {
                *control_flow = ControlFlow::Exit;
            }

            let subaction = &self.high_level_fighter.subactions[self.subaction_index];
            self.app_state.update(&self.input, subaction);

            if let Some(size) = self.input.window_resized() {
                self.swap_chain_descriptor.width = size.width;
                self.swap_chain_descriptor.height = size.height;
                self.swap_chain = self.wgpu_state.device.create_swap_chain(&self.surface, &self.swap_chain_descriptor);
            }

            {
                let framebuffer = self.swap_chain.get_next_texture().unwrap();
                let command_encoder = draw_frame(
                    &mut self.wgpu_state,
                    &framebuffer.view,
                    self.swap_chain_descriptor.width,
                    self.swap_chain_descriptor.height,
                    self.app_state.perspective,
                    self.app_state.wireframe,
                    self.app_state.render_ecb,
                    &self.app_state.invulnerable_type,
                    &self.high_level_fighter,
                    self.subaction_index,
                    self.app_state.frame_index,
                    &self.app_state.camera,
                );
                self.wgpu_state.queue.submit(&[command_encoder.finish()]);
            }
        }
    }
}

/// Opens an interactive window displaying hurtboxes and hitboxes
/// Blocks until user closes window
pub fn render_window(high_level_fighter: &HighLevelFighter, subaction_index: usize) {
    let event_loop = EventLoop::new();
    let mut app = App::new(&event_loop, high_level_fighter, subaction_index);

    event_loop.run(move |event, _, control_flow| {
        app.update(event, control_flow);
    });
}

/// Returns a receiver of the bytes of a gif displaying hitbox and hurtboxes
pub fn render_gif(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Receiver<Vec<u8>> {
    // maximum dimensions for gifs on discord, larger values will result in one dimension being shrunk retaining aspect ratio
    // restricted to u16 because of the gif library we are using
    let width: u16 = 400;
    let height: u16 = 300;

    let subaction = &high_level_fighter.subactions[subaction_index];

    let (frames_tx, frames_rx) = mpsc::channel();
    let (gif_tx, gif_rx) = mpsc::channel();

    // Spawns a thread that takes the rendered frames and quantizes the pixels into a paletted gif
    // alex-charlton.com/posts/Dithering_on_the_GPU/
    let subaction_len = subaction.frames.len();
    thread::spawn(move || {
        let mut result = vec!();
        {
            let mut encoder = gif::Encoder::new(&mut result, width, height, &[]).unwrap();
            for _ in 0..subaction_len {
                let mut frame_data: Vec<u8> = frames_rx.recv().unwrap();
                let gif_frame = gif::Frame::from_rgba_speed(width as u16, height as u16, &mut frame_data, 30);
                encoder.write_frame(&gif_frame).unwrap();
            }
            encoder.write_extension(gif::ExtensionData::Repetitions(gif::Repeat::Infinite)).unwrap();
        }
        gif_tx.send(result).unwrap();
    });

    // Render each frame, sending it to the gif thread
    for (frame_index, _) in subaction.frames.iter().enumerate() {
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
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
        };

        let framebuffer = state.device.create_texture(framebuffer_descriptor);
        let framebuffer_copy_view = wgpu::TextureCopyView {
            texture: &framebuffer,
            mip_level: 0,
            array_layer: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
        };

        let framebuffer_out_descriptor = &wgpu::BufferDescriptor {
            size: width as u64 * height as u64 * 4,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
        };
        let bytes_per_pixel = 4 * 4;
        let framebuffer_out = state.device.create_buffer(framebuffer_out_descriptor);
        let framebuffer_out_copy_view = wgpu::BufferCopyView {
            buffer: &framebuffer_out,
            offset: 0,
            bytes_per_row: width as u32 * bytes_per_pixel, // multiple of 256 bytes??? wtf? yikes!
            rows_per_image: height as u32,
        };

        let camera = new_camera(subaction, width, height);
        let mut command_encoder = draw_frame(state, &framebuffer.create_default_view(), width as u32, height as u32, false, false, false, &InvulnerableType::Hit, high_level_fighter, subaction_index, frame_index, &camera);
        command_encoder.copy_texture_to_buffer(framebuffer_copy_view, framebuffer_out_copy_view, texture_extent);
        state.queue.submit(&[command_encoder.finish()]);

        let frames_tx = frames_tx.clone();
        // TODO: don't block
        match futures::executor::block_on(framebuffer_out.map_read(0, width as u64 * height as u64 * 4)) {
            Ok(data) => {
                let pixel_count = width as usize * height as usize;
                let mut result = data.as_slice().to_vec();
                for i in 0..pixel_count {
                      let b = result[i * 4 + 0];
                    //let g = result[i * 4 + 1];
                      let r = result[i * 4 + 2];
                    //let a = result[i * 4 + 3];

                      result[i * 4 + 0] = r;
                    //result[i * 4 + 1] = g;
                      result[i * 4 + 2] = b;
                    //result[i * 4 + 3] = a;
                }
                frames_tx.send(result).unwrap();
            }
            Err(error) => {
                panic!("map_read failed: {:?}", error); // We have to panic here to avoid an infinite loop :/
            }
        };
        state.device.poll(true);
    }

    gif_rx
}

/// Returns the bytes of a gif displaying hitbox and hurtboxes
pub fn render_gif_blocking(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Vec<u8> {
    let gif_rx = render_gif(state, high_level_fighter, subaction_index);
    loop {
        match gif_rx.try_recv() {
            Err(_) => {
                // Needed to get the map_read to run.
                state.device.poll(true);
            }
            Ok(value) => {
                return value
            }
        }
    }
}

pub struct WgpuState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    bind_group_layout: wgpu::BindGroupLayout,
    render_pipeline: wgpu::RenderPipeline,
}

impl WgpuState {
    pub fn new() -> WgpuState {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
            },
            wgpu::BackendBit::PRIMARY,
        );
        let adapter = futures::executor::block_on(adapter).unwrap();
        let (device, queue) = futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            limits: wgpu::Limits::default(),
            extensions: wgpu::Extensions {
                anisotropic_filtering: false,
            },
        }));

        // shaders
        let vs = include_bytes!("shaders/fighter.vert.spv");
        let vs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&vs[..])).unwrap());
        let fs = include_bytes!("shaders/fighter.frag.spv");
        let fs_module = device.create_shader_module(&wgpu::read_spirv(std::io::Cursor::new(&fs[..])).unwrap());

        // layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            bindings: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
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
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8Unorm,
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
            sample_count: SAMPLE_COUNT,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        WgpuState {
            device,
            queue,
            bind_group_layout,
            render_pipeline,
        }
    }

    pub fn poll(&self) {
        self.device.poll(true);
    }
}

struct Draw {
    bind_group:  wgpu::BindGroup,
    vertices:    wgpu::Buffer,
    indices:     wgpu::Buffer,
    indices_len: usize,
}

fn draw_frame(state: &mut WgpuState, framebuffer: &wgpu::TextureView, width: u32, height: u32, perspective: bool, wireframe: bool, render_ecb: bool, invulnerable_type: &InvulnerableType, high_level_fighter: &HighLevelFighter, subaction_index: usize, frame_index: usize, camera: &Camera) -> wgpu::CommandEncoder {
    let mut command_encoder = state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
    let mut draws = vec!();

    let multisampled_texture_extent = wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth: 1
    };
    let multisampled_framebuffer_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        array_layer_count: 1,
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
    };

    let multisampled_framebuffer = state.device.create_texture(multisampled_framebuffer_descriptor);

    {
        let attachment = multisampled_framebuffer.create_default_view();
        let mut rpass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &attachment,
                resolve_target: Some(framebuffer),
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

        let subaction = &high_level_fighter.subactions[subaction_index];
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
                Vector3::new(bone_matrix.x.x, bone_matrix.x.y, bone_matrix.x.z).magnitude(),
                Vector3::new(bone_matrix.y.x, bone_matrix.y.y, bone_matrix.y.z).magnitude(),
                Vector3::new(bone_matrix.z.x, bone_matrix.z.y, bone_matrix.z.z).magnitude(),
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
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
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
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
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
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
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
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
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
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0..64,
                        },
                    },
                ],
            });

            let indices_len = indices_array.len();
            draws.push(Draw { bind_group, vertices, indices, indices_len });
        }

        for draw in &draws {
            rpass.set_bind_group(0, &draw.bind_group, &[]);
            rpass.set_index_buffer(&draw.indices, 0, 0);
            rpass.set_vertex_buffer(0, &draw.vertices, 0, 0);
            rpass.draw_indexed(0..draw.indices_len as u32, 0, 0..1);
        }
    }

    command_encoder
}
