use std::sync::mpsc::Sender;

use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::app::winit_app::WinitApp;

pub mod state;
mod winit_app;

use state::{AppEventIncoming, AppEventOutgoingHandler};
/// Interactive hitbox renderer app compatible with desktop and web.
pub struct App {
    winit_app: WinitApp,
    event_loop: Option<EventLoop<()>>,
}

impl App {
    /// Opens a window for the app
    pub async fn new(subaction: HighLevelSubaction) -> Self {
        let event_loop = EventLoop::new().unwrap();
        #[allow(
            deprecated,
            reason = r#"We should fix this, but its going to take some refactoring I dont have capacity for at the moment.
We need to move the window + wgpu resource creation logic inside the winit app.
This will require making async calls inside the winit app, via futures::executor::block_on and the equivalent wasm function.
We should look at https://github.com/gfx-rs/wgpu/pull/5709/files or the repo itself if upstreamed."#
        )]
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        App::new_common(window, event_loop, subaction).await
    }

    #[cfg(target_arch = "wasm32")]
    /// Inserts a surface for the app into the provided element
    pub async fn new_insert_into_element(
        element: web_sys::Element,
        subaction: HighLevelSubaction,
    ) -> Self {
        use winit::platform::web::WindowExtWebSys;

        let event_loop = EventLoop::new().unwrap();
        #[allow(
            deprecated,
            reason = r#"We should fix this, but its going to take some refactoring I dont have capacity for at the moment.
We need to move the window + wgpu resource creation logic inside the winit app.
This will require making async calls inside the winit app, via futures::executor::block_on and the equivalent wasm function.
We should look at https://github.com/gfx-rs/wgpu/pull/5709/files or the repo itself if upstreamed."#
        )]
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        let canvas = window.canvas().unwrap();
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
        App {
            winit_app: WinitApp::new(window, subaction).await,
            event_loop: Some(event_loop),
        }
    }

    /// Starts running the app.
    /// This function blocks until user closes window
    pub fn run(mut self) {
        self.event_loop
            .take()
            .unwrap()
            .run_app(&mut self.winit_app)
            .unwrap();
    }

    /// Sets a function that will be called when various internal events occur within the app
    pub fn set_event_handler(&mut self, event_handler: AppEventOutgoingHandler) {
        self.winit_app.set_event_handler(event_handler);
    }

    /// Returns a sender that allows you to send events into the app to control its state
    pub fn get_event_tx(&self) -> Sender<AppEventIncoming> {
        self.winit_app.get_event_tx()
    }
}
