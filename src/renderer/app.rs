use winit_input_helper::WinitInputHelper;
use wgpu::winit::{VirtualKeyCode};
use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::camera::Camera;

pub(crate) enum State {
    Play,
    StepForward,
    StepBackward,
    Pause,
}

pub(crate) struct App {
    pub frame_index: usize,
    pub wireframe: bool,
    pub perspective: bool,
    pub camera: Camera,
    state: State,
}

impl App {
    pub fn new(camera: Camera) -> App {
        App {
            frame_index: 0,
            wireframe: false,
            perspective: false,
            camera,
            state: State::Play,
        }
    }

    pub fn update(&mut self, input: &WinitInputHelper, subaction: &HighLevelSubaction) {
        if input.key_pressed(VirtualKeyCode::Key1) {
            self.wireframe = !self.wireframe;
        }
        if input.key_pressed(VirtualKeyCode::Key2) {
            self.perspective = !self.perspective;
        }
        if input.key_pressed(VirtualKeyCode::Back) {
            // TODO: Reset camera
            self.frame_index = 0; // TODO: Probably delete this later, resetting frame_index is kind of only useful for debugging.
        }
        if input.key_pressed(VirtualKeyCode::Space) || input.key_pressed(VirtualKeyCode::Right) {
            self.state = State::StepForward;
        }
        if input.key_pressed(VirtualKeyCode::Left) {
            self.state = State::StepBackward;
        }
        if input.key_pressed(VirtualKeyCode::Return) {
            self.state = State::Play;
        }

        // advance frame
        match self.state {
            State::StepForward | State::Play => {
                self.frame_index += 1;
                if self.frame_index >= subaction.frames.len() {
                    self.frame_index = 0;
                }
            }
            State::StepBackward => {
                if self.frame_index == 0 {
                    self.frame_index = subaction.frames.len() - 1;
                } else {
                    self.frame_index -= 1
                }
            }
            State::Pause => { }
        }

        if let State::StepForward | State::StepBackward = self.state {
            self.state = State::Pause;
        }
    }
}
