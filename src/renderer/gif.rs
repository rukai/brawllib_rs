use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::high_level_fighter::HighLevelFighter;
use crate::renderer::app::state::InvulnerableType;
use crate::renderer::camera::Camera;
use crate::renderer::wgpu_state::WgpuState;
use crate::renderer::draw::draw_frame;

/// Returns a receiver of the bytes of a gif displaying hitbox and hurtboxes
pub async fn render_gif(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Receiver<Vec<u8>> {
    // maximum dimensions for gifs on discord, larger values will result in one dimension being shrunk retaining aspect ratio
    // restricted to u16 because of the gif library we are using
    let width: u16 = 400;
    let height: u16 = 300;

    let subaction = &high_level_fighter.subactions[subaction_index];

    let (frames_tx, frames_rx) = mpsc::channel();
    let (gif_tx, gif_rx) = mpsc::channel();

    // Spawns a thread that takes the rendered frames and quantizes the pixels into a paletted gif
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
        let bytes_per_pixel = 4;
        let framebuffer_out = state.device.create_buffer(framebuffer_out_descriptor);
        let framebuffer_out_copy_view = wgpu::BufferCopyView {
            buffer: &framebuffer_out,
            offset: 0,
            bytes_per_row: width as u32 * bytes_per_pixel,
            rows_per_image: 0,
        };

        let camera = Camera::new(subaction, width, height);
        let mut command_encoder = draw_frame(state, &framebuffer.create_default_view(), width as u32, height as u32, false, false, false, &InvulnerableType::Hit, high_level_fighter, subaction_index, frame_index, &camera);
        command_encoder.copy_texture_to_buffer(framebuffer_copy_view, framebuffer_out_copy_view, texture_extent);
        state.queue.submit(&[command_encoder.finish()]);

        let frames_tx = frames_tx.clone();
        match framebuffer_out.map_read(0, width as u64 * height as u64 * 4).await {
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
        }
    }

    gif_rx
}

/// Returns the bytes of a gif displaying hitbox and hurtboxes
pub fn render_gif_blocking(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Vec<u8> {
    let gif_rx = futures::executor::block_on(render_gif(state, high_level_fighter, subaction_index));
    loop {
        match gif_rx.try_recv() {
            Err(_) => {
                // Needed to get the map_read to run. // TODO: Might not be needed anymore?
                state.device.poll(true);
            }
            Ok(value) => {
                return value
            }
        }
    }
}
