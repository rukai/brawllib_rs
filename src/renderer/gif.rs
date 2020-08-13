use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;

use crate::high_level_fighter::HighLevelFighter;
use crate::renderer::app::state::InvulnerableType;
use crate::renderer::camera::Camera;
use crate::renderer::wgpu_state::WgpuState;
use crate::renderer::draw::draw_frame;


/// Returns a receiver of the bytes of a gif displaying hitbox and hurtboxes
///
/// Most of the time is spent CPU side waiting for the color quantization thread to finish.
/// So if you are batch rendering gifs you will get a massive speedup by running multiple `render_gif`s concurrently.
///
/// TODO: I should probably only expose a wrapper of this function that calls executor::block_on, so the user doesnt have to care about async
pub async fn render_gif(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Receiver<Vec<u8>> {
    // maximum dimensions for gifs on discord, larger values will result in one dimension being shrunk retaining aspect ratio
    // restricted to u16 because of the gif library we are using
    let width: u16 = 400;
    let height: u16 = 300;
    let format = wgpu::TextureFormat::Rgba8Unorm;

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
        let framebuffer_extent = wgpu::Extent3d {
            width: width as u32,
            height: height as u32,
            depth: 1
        };
        let framebuffer_descriptor = &wgpu::TextureDescriptor {
            size: framebuffer_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::COPY_SRC,
            label: None,
        };

        let framebuffer = state.device.create_texture(framebuffer_descriptor);
        let framebuffer_copy_view = wgpu::TextureCopyView {
            texture: &framebuffer,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
        };

        // It is a webgpu requirement that BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
        // So we calculate padded_bytes_per_row by rounding real_bytes_per_row
        // up to the next multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
        // https://en.wikipedia.org/wiki/Data_structure_alignment#Computing_padding
        let bytes_per_pixel = std::mem::size_of::<u32>() as u32;
        let real_bytes_per_row = width as u32 * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row_padding = (align - real_bytes_per_row % align) % align;
        let padded_bytes_per_row = real_bytes_per_row + padded_bytes_per_row_padding;

        let framebuffer_out_descriptor = &wgpu::BufferDescriptor {
            size: padded_bytes_per_row as u64 * height as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
            label: None,
            mapped_at_creation: false,
        };

        let framebuffer_out = state.device.create_buffer(framebuffer_out_descriptor);
        let framebuffer_out_copy_view = wgpu::BufferCopyView {
            buffer: &framebuffer_out,
            layout: wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: padded_bytes_per_row,
                rows_per_image: 0
            }
        };

        let camera = Camera::new(subaction, width, height);
        let mut command_encoder = draw_frame(state, &framebuffer.create_view(&wgpu::TextureViewDescriptor::default()), width as u32, height as u32, false, false, false, &InvulnerableType::Hit, subaction, frame_index, &camera); // 3.0ms
        command_encoder.copy_texture_to_buffer(framebuffer_copy_view, framebuffer_out_copy_view, framebuffer_extent);
        state.queue.submit(Some(command_encoder.finish()));

        let frames_tx = frames_tx.clone();
        let framebuffer_out_slice = framebuffer_out.slice(..);
        let read = framebuffer_out_slice.map_async(wgpu::MapMode::Read);

        state.poll(); // 5ms

        match read.await {
            Ok(()) => {
                // move the padding to the end of the buffer
                let mut padded_buffer = framebuffer_out_slice.get_mapped_range().to_vec();
                for y in 1..height as usize {
                    let padded_offset = y * padded_bytes_per_row as usize;
                    let real_offset = y * real_bytes_per_row as usize;
                    padded_buffer.copy_within(padded_offset..padded_offset+real_bytes_per_row as usize, real_offset)
                }

                // send just the image data ignoring the padding at the end
                let real_buffer = padded_buffer[0..real_bytes_per_row as usize * height as usize].to_vec();
                frames_tx.send(real_buffer).unwrap();
            }
            Err(error) => panic!("map_read failed: {:?}", error),
        }
    }

    gif_rx
}

/// Returns the bytes of a gif displaying hitbox and hurtboxes
pub fn render_gif_blocking(state: &mut WgpuState, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> Vec<u8> {
    let gif_rx = futures::executor::block_on(render_gif(state, high_level_fighter, subaction_index));
    gif_rx.recv().unwrap()
}
