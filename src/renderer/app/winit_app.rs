use std::sync::Arc;
use std::sync::mpsc::{Sender, channel};

use wgpu::InstanceDescriptor;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use winit_input_helper::WinitInputHelper;

use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::camera::Camera;
use crate::renderer::draw::draw_frame;
use crate::renderer::wgpu_state::{CompatibleSurface, WgpuState};

use crate::renderer::app::state::{AppEventIncoming, AppEventOutgoingHandler, AppState};

/// Interactive hitbox renderer app compatible with desktop and web.
///
/// Implementation details:
/// Glues together:
/// *   AppState: All application logic goes in here
/// *   WgpuState: All rendering logic goes in here
/// *   Other bits and pieces missing from WgpuState because they aren't needed for rendering to GIF.
pub struct WinitApp {
    pub wgpu_state: WgpuState,
    pub app_state: AppState,
    pub input: WinitInputHelper,
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub surface_configuration: wgpu::SurfaceConfiguration,
    pub subaction: HighLevelSubaction,
    pub event_tx: Sender<AppEventIncoming>,
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // This method is called when the app is resumed, including when it
        //  is first started. If we do not have a window, we have to create one.
        // if self.window.is_none() {
        //     self.window = Some(platform::create_window(event_loop));
        // }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        // Pass every event to the WinitInputHelper.
        // It will return true if it receives a RequestedRedraw event: you should then render.
        if self.input.process_window_event(&event) {
            // TODO: does this comment make sense in winit 0.30
            // app loop relies on this blocking until draw completes due to PresentMode configuration.
            // Currently we rely on PresentMode::Immediate on desktop which in theory should not work as it does not block.
            // We had to swap to it because PresentMode::Fifo times out which is possibly a wgpu bug.
            let frame = self.surface.get_current_texture().unwrap();
            let command_encoder = draw_frame(
                &mut self.wgpu_state,
                &frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                self.surface_configuration.width,
                self.surface_configuration.height,
                self.app_state.perspective,
                self.app_state.wireframe,
                self.app_state.render_ecb,
                &self.app_state.invulnerable_type,
                &self.subaction,
                self.app_state.frame_index,
                &self.app_state.camera,
            );
            self.wgpu_state.queue.submit(Some(command_encoder.finish()));
            frame.present();
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        self.input.process_device_event(&event);
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.input.end_step();

        if self.input.close_requested() || self.input.destroyed() {
            el.exit();
        }
        self.app_state.update(
            &self.input,
            &self.subaction,
            self.surface_configuration.width as u16,
            self.surface_configuration.height as u16,
        );

        if let Some(size) = self.input.window_resized() {
            self.surface_configuration.width = size.width;
            self.surface_configuration.height = size.height;
            self.surface
                .configure(&self.wgpu_state.device, &self.surface_configuration);
        }

        // We arrive here constantly because of ControlFlow::Poll.
        // So we request a redraw here to constantly redraw.
        self.window.request_redraw();
    }

    fn new_events(&mut self, _: &ActiveEventLoop, _: StartCause) {
        self.input.step();
    }
}

impl WinitApp {
    pub async fn new(window: Window, subaction: HighLevelSubaction) -> Self {
        let window = Arc::new(window);
        let input = WinitInputHelper::new();
        let size = window.inner_size();

        let instance =
            wgpu::util::new_instance_with_webgpu_detection(&InstanceDescriptor::default()).await;
        let surface = instance.create_surface(window.clone()).unwrap();
        let wgpu_state = WgpuState::new(
            instance,
            CompatibleSurface::Surface(&surface),
            wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            },
        )
        .await;

        let surface_configuration = wgpu::SurfaceConfiguration {
            format: wgpu_state.format,
            // does not work on desktop as we immediately write the 3 frames then timeout waiting for the next frame
            #[cfg(target_arch = "wasm32")]
            present_mode: wgpu::PresentMode::Fifo,
            // does not work on webgl as it does not support Immediate
            #[cfg(not(target_arch = "wasm32"))]
            present_mode: wgpu::PresentMode::Immediate,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            width: size.width.max(1),
            height: size.height.max(1),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&wgpu_state.device, &surface_configuration);

        let camera = Camera::new(
            &subaction,
            surface_configuration.width as u16,
            surface_configuration.height as u16,
        );

        let (event_tx, event_rx) = channel();
        let app_state = AppState::new(camera, event_rx);

        Self {
            wgpu_state,
            app_state,
            input,
            window,
            surface,
            surface_configuration,
            subaction,
            event_tx,
        }
    }

    /// Sets a function that will be called when various internal events occur within the app
    pub fn set_event_handler(&mut self, event_handler: AppEventOutgoingHandler) {
        self.app_state.set_event_handler(event_handler);
    }

    /// Returns a sender that allows you to send events into the app to control its state
    pub fn get_event_tx(&self) -> Sender<AppEventIncoming> {
        self.event_tx.clone()
    }
}
