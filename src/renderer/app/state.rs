use std::sync::mpsc::Receiver;

use winit::event::VirtualKeyCode;
use winit_input_helper::WinitInputHelper;

use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::camera::{Camera, CharacterFacing};

pub type AppEventOutgoingHandler = Box<dyn Fn(AppEventOutgoing)>;

pub enum AppEventIncoming {
    SetState(State),
    SetFrame(usize),
    SetInvulnerableType(InvulnerableType),
    SetWireframe(bool),
    SetRenderEcb(bool),
    SetPerspective(bool),
    ResetCamera(CharacterFacing),
}

pub enum AppEventOutgoing {
    NewState(State),
    NewFrame(usize),
    NewInvulnerableType(InvulnerableType),
    NewWireframe(bool),
    NewRenderEcb(bool),
    NewPerspective(bool),
}

#[derive(Clone)]
pub enum State {
    Play,
    StepForward,
    StepBackward,
    Pause,
}

#[derive(Clone)]
pub enum InvulnerableType {
    Hit,
    Grab,
    TrapItem,
}

pub struct AppState {
    pub frame_index: usize,
    pub wireframe: bool,
    pub perspective: bool,
    pub render_ecb: bool,
    pub invulnerable_type: InvulnerableType,
    pub camera: Camera,
    state: State,
    event_handler: Option<AppEventOutgoingHandler>,
    event_rx: Receiver<AppEventIncoming>,
}

impl AppState {
    pub fn new(camera: Camera, event_rx: Receiver<AppEventIncoming>) -> AppState {
        AppState {
            frame_index: 0,
            wireframe: false,
            perspective: false,
            render_ecb: false,
            invulnerable_type: InvulnerableType::Hit,
            camera,
            state: State::Play,
            event_handler: None,
            event_rx,
        }
    }

    pub fn update(
        &mut self,
        input: &WinitInputHelper,
        subaction: &HighLevelSubaction,
        window_width: u16,
        window_height: u16,
    ) {
        for event in self.event_rx.try_iter() {
            match event {
                AppEventIncoming::SetState(state) => {
                    self.state = state.clone();
                    self.send_event(AppEventOutgoing::NewState(state));
                }
                AppEventIncoming::SetFrame(frame) => {
                    self.frame_index = frame;
                    self.send_event(AppEventOutgoing::NewFrame(frame));
                }
                AppEventIncoming::SetInvulnerableType(invulnerable_type) => {
                    self.invulnerable_type = invulnerable_type.clone();
                    self.send_event(AppEventOutgoing::NewInvulnerableType(invulnerable_type));
                }
                AppEventIncoming::SetWireframe(wireframe) => {
                    self.wireframe = wireframe;
                    self.send_event(AppEventOutgoing::NewWireframe(wireframe));
                }
                AppEventIncoming::SetRenderEcb(ecb) => {
                    self.render_ecb = ecb;
                    self.send_event(AppEventOutgoing::NewRenderEcb(ecb));
                }
                AppEventIncoming::SetPerspective(perspective) => {
                    self.perspective = perspective;
                    self.send_event(AppEventOutgoing::NewPerspective(perspective));
                }
                AppEventIncoming::ResetCamera(facing) => {
                    self.camera.reset(window_width, window_height, facing)
                }
            }
        }

        if input.key_pressed(VirtualKeyCode::Key1) {
            self.wireframe = !self.wireframe;
            self.send_event(AppEventOutgoing::NewWireframe(self.wireframe));
        }
        if input.key_pressed(VirtualKeyCode::Key2) {
            self.perspective = !self.perspective;
            self.send_event(AppEventOutgoing::NewPerspective(self.perspective));
        }
        if input.key_pressed(VirtualKeyCode::Key3) {
            self.render_ecb = !self.render_ecb;
            self.send_event(AppEventOutgoing::NewRenderEcb(self.render_ecb));
        }
        if input.key_pressed(VirtualKeyCode::Back) {
            self.camera
                .reset(window_width, window_height, CharacterFacing::Right);
        }
        if input.key_pressed(VirtualKeyCode::Space) || input.key_pressed(VirtualKeyCode::Right) {
            self.set_state(State::StepForward);
        }
        if input.key_pressed(VirtualKeyCode::Left) {
            self.set_state(State::StepBackward);
        }
        if input.key_pressed(VirtualKeyCode::Return) {
            self.set_state(State::Play);
        }
        if input.key_pressed(VirtualKeyCode::Q) {
            self.invulnerable_type = InvulnerableType::Hit;
            self.send_event(AppEventOutgoing::NewInvulnerableType(
                self.invulnerable_type.clone(),
            ));
        }
        if input.key_pressed(VirtualKeyCode::W) {
            self.invulnerable_type = InvulnerableType::Grab;
            self.send_event(AppEventOutgoing::NewInvulnerableType(
                self.invulnerable_type.clone(),
            ));
        }
        if input.key_pressed(VirtualKeyCode::E) {
            self.invulnerable_type = InvulnerableType::TrapItem;
            self.send_event(AppEventOutgoing::NewInvulnerableType(
                self.invulnerable_type.clone(),
            ));
        }

        let small = 0.00001;

        if input.mouse_held(0) {
            let diff = input.mouse_diff();
            self.camera.theta -= diff.0 / 100.0;
            self.camera.phi -= diff.1 / 100.0;
        }

        if self.camera.theta > std::f32::consts::PI * 2.0 {
            self.camera.theta = 0.0;
        } else if self.camera.theta < 0.0 {
            self.camera.theta = std::f32::consts::PI * 2.0;
        }

        if self.camera.phi > std::f32::consts::PI - small {
            self.camera.phi = std::f32::consts::PI - small;
        } else if self.camera.phi < small {
            self.camera.phi = small;
        }

        self.camera.radius -= input.scroll_diff() * 2.0;
        let min_camera_radius = 0.0000001;
        if self.camera.radius < min_camera_radius {
            self.camera.radius = min_camera_radius;
        }

        // advance frame
        match self.state {
            State::StepForward | State::Play => {
                if self.frame_index == subaction.frames.len() - 1 {
                    self.set_frame_index(0);
                } else {
                    self.set_frame_index(self.frame_index + 1);
                }
            }
            State::StepBackward => {
                if self.frame_index == 0 {
                    self.set_frame_index(subaction.frames.len() - 1);
                } else {
                    self.set_frame_index(self.frame_index - 1);
                }
            }
            State::Pause => {}
        }

        if let State::StepForward | State::StepBackward = self.state {
            self.set_state(State::Pause);
        }
    }

    fn set_frame_index(&mut self, frame_index: usize) {
        self.frame_index = frame_index;
        self.send_event(AppEventOutgoing::NewFrame(frame_index));
    }

    fn set_state(&mut self, state: State) {
        self.state = state.clone();
        self.send_event(AppEventOutgoing::NewState(state));
    }

    fn send_event(&self, event: AppEventOutgoing) {
        if let Some(event_handler) = &self.event_handler {
            event_handler(event);
        }
    }

    pub fn set_event_handler(&mut self, event_handler: AppEventOutgoingHandler) {
        self.event_handler = Some(event_handler);
    }
}
