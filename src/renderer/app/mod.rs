use std::sync::mpsc::{channel, Sender};

use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::camera::Camera;
use crate::renderer::draw::draw_frame;
use crate::renderer::wgpu_state::{CompatibleSurface, WgpuState};

pub mod state;

use state::{AppEventIncoming, AppEventOutgoingHandler, AppState};
/// Interactive hitbox renderer app compatible with desktop and web.
///
/// Implementation details:
/// Glues together:
/// *   AppState: All application logic goes in here
/// *   WgpuState: All rendering logic goes in here
/// *   Other bits and pieces missing from WgpuState because they aren't needed for rendering to GIF.
pub struct App {
    wgpu_state: WgpuState,
    app_state: AppState,
    input: WinitInputHelper,
    window: Window,
    surface: wgpu::Surface,
    surface_configuration: wgpu::SurfaceConfiguration,
    subaction: HighLevelSubaction,
    event_tx: Sender<AppEventIncoming>,
    event_loop: Option<EventLoop<()>>,
}

impl App {
    /// Opens a window for the app
    pub async fn new(subaction: HighLevelSubaction) -> Self {
        let event_loop = EventLoop::new();
        let window = Window::new(&event_loop).unwrap();
        App::new_common(window, event_loop, subaction).await
    }

    #[cfg(target_arch = "wasm32")]
    /// Inserts a surface for the app into the provided element
    pub async fn new_insert_into_element(
        element: web_sys::Element,
        subaction: HighLevelSubaction,
    ) -> Self {
        use winit::platform::web::WindowExtWebSys;

        let event_loop = EventLoop::new();
        let window = Window::new(&event_loop).unwrap();

        let canvas = window.canvas();
        canvas
            .style()
            .set_css_text("display: block; width: 100%; height: 100%");

        element
            .append_child(&web_sys::Element::from(canvas))
            .unwrap();

        App::new_common(window, event_loop, subaction).await
    }

    async fn new_common(
        window: Window,
        event_loop: EventLoop<()>,
        subaction: HighLevelSubaction,
    ) -> App {
        let input = WinitInputHelper::new();
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
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
            present_mode: wgpu::PresentMode::Fifo,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            width: size.width,
            height: size.height,
        };
        surface.configure(&wgpu_state.device, &surface_configuration);

        let camera = Camera::new(
            &subaction,
            surface_configuration.width as u16,
            surface_configuration.height as u16,
        );

        let (event_tx, event_rx) = channel();
        let app_state = AppState::new(camera, event_rx);

        App {
            wgpu_state,
            app_state,
            input,
            window,
            surface,
            surface_configuration,
            subaction,
            event_tx,
            event_loop: Some(event_loop),
        }
    }

    /// Starts running the app.
    /// This function blocks until user closes window
    pub fn run(mut self) {
        self.event_loop
            .take()
            .unwrap()
            .run(move |event, _, control_flow| {
                self.update(event, control_flow);
            });
    }

    /// Sets a function that will be called when various internal events occur within the app
    pub fn set_event_handler(&mut self, event_handler: AppEventOutgoingHandler) {
        self.app_state.set_event_handler(event_handler);
    }

    /// Returns a sender that allows you to send events into the app to control its state
    pub fn get_event_tx(&self) -> Sender<AppEventIncoming> {
        self.event_tx.clone()
    }

    /// Manually update the app state, call this instead of `App::run` if you need to maintain control of the event loop.
    pub fn update(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if let Event::RedrawRequested(_) = event {
            // app loop relies on this blocking until draw completes due to PresentMode::Fifo
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

        if self.input.update(&event) {
            if self.input.quit() {
                *control_flow = ControlFlow::Exit;
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
    }
}
