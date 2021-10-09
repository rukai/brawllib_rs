use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use winit::event::Event;
use winit_input_helper::WinitInputHelper;

use crate::high_level_fighter::{HighLevelFighter, HighLevelSubaction};
use crate::renderer::wgpu_state::WgpuState;
use crate::renderer::draw::draw_frame;
use crate::renderer::camera::Camera;

pub(crate) mod state;

use state::AppState;

pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// Opens an interactive window displaying hurtboxes and hitboxes
/// Blocks until user closes window
pub fn render_window(high_level_fighter: &HighLevelFighter, subaction_index: usize) {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();
    let high_level_fighter = high_level_fighter.clone();
    let subaction = high_level_fighter.subactions[subaction_index].clone();
    let mut app = futures::executor::block_on(App::new(window, subaction));

    event_loop.run(move |event, _, control_flow| {
        app.update(event, control_flow);
    });
}


/// Adds an interactive element to the webpage displaying hurtboxes and hitboxes
#[cfg(target_arch = "wasm32")]
pub async fn render_window_wasm(subaction: HighLevelSubaction) {
    use winit::platform::web::WindowExtWebSys;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlElement;

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let document = web_sys::window().unwrap().document().unwrap();

    let visualiser_span = document.get_element_by_id("visualiser").unwrap();
    visualiser_span.append_child(&web_sys::Element::from(window.canvas())).unwrap();

    let button = document.get_element_by_id("foo").unwrap();
    let button_move = button.clone();
    let do_thing = Closure::wrap(
        Box::new(move || {
            button_move.set_inner_html("何も");
        }) as Box<dyn FnMut()>
    );
    button
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(do_thing.as_ref().unchecked_ref()));

    let mut app = App::new(window, subaction).await;

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
    surface_configuration: wgpu::SurfaceConfiguration,
    subaction: HighLevelSubaction,
}

impl App {
    pub async fn new(_window: Window, subaction: HighLevelSubaction) -> App {
        let input = WinitInputHelper::new();
        let size = _window.inner_size();

        let surface_configuration = wgpu::SurfaceConfiguration {
            present_mode: wgpu::PresentMode::Fifo,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: FORMAT,
            width: size.width,
            height: size.height,
        };

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&_window) };
        let wgpu_state = WgpuState::new(instance, Some(&surface), FORMAT).await;
        surface.configure(&wgpu_state.device, &surface_configuration);

        let camera = Camera::new(
            &subaction,
            surface_configuration.width as u16,
            surface_configuration.height as u16,
        );
        let app_state = AppState::new(camera);

        App { wgpu_state, app_state, input, _window, surface, surface_configuration, subaction  }
    }

    pub fn update(&mut self, event: Event<()>, control_flow: &mut ControlFlow) {
        if self.input.update(&event) {
            if self.input.quit() {
                *control_flow = ControlFlow::Exit;
            }

            self.app_state.update(&self.input, &self.subaction);

            if let Some(size) = self.input.window_resized() {
                self.surface_configuration.width = size.width;
                self.surface_configuration.height = size.height;
                self.surface.configure(&self.wgpu_state.device, &self.surface_configuration);
            }

            let frame = self.surface.get_current_texture().unwrap();
            let command_encoder = draw_frame(
                &mut self.wgpu_state,
                &frame.texture.create_view(&wgpu::TextureViewDescriptor::default()),
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
}
