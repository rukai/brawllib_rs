use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::Event;
use winit_input_helper::WinitInputHelper;

use crate::high_level_fighter::HighLevelFighter;
use crate::renderer::wgpu_state::WgpuState;
use crate::renderer::draw::draw_frame;
use crate::renderer::camera::Camera;

pub(crate) mod state;

use state::AppState;

/// Opens an interactive window displaying hurtboxes and hitboxes
/// Blocks until user closes window
pub fn render_window(high_level_fighter: &HighLevelFighter, subaction_index: usize) {
    let event_loop = EventLoop::new();
    let mut app = App::new(&event_loop, high_level_fighter, subaction_index);

    event_loop.run(move |event, _, control_flow| {
        app.update(event, control_flow);
    });
}

/// Glues together:
/// *   AppState: All application logic goes in here
/// *   WgpuState: All rendering logic goes in here
/// *   Other bits and pieces missing from WgpuState because they aren't needed for rendering to GIF.
pub struct App {
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
    pub fn new(event_loop: &EventLoop<()>, high_level_fighter: &HighLevelFighter, subaction_index: usize) -> App {
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

        let camera = Camera::new(
            subaction,
            swap_chain_descriptor.width as u16,
            swap_chain_descriptor.height as u16,
        );
        let app_state = AppState::new(camera);

        let high_level_fighter = high_level_fighter.clone();

        App { wgpu_state, app_state, input, _window, surface, swap_chain, swap_chain_descriptor, high_level_fighter, subaction_index }
    }

    pub fn update(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
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
