use winit_input_helper::WinitInputHelper;
use winit::event::VirtualKeyCode;
use crate::high_level_fighter::HighLevelSubaction;
use crate::renderer::camera::Camera;

pub(crate) enum State {
    Play,
    StepForward,
    StepBackward,
    Pause,
}

pub (crate) enum InvulnerableType {
    Hit,
    Grab,
    TrapItem,
}

pub(crate) struct AppState {
    pub frame_index:       usize,
    pub wireframe:         bool,
    pub perspective:       bool,
    pub render_ecb:        bool,
    pub invulnerable_type: InvulnerableType,
    pub camera:            Camera,
    state:                 State,
}

impl AppState {
    pub fn new(camera: Camera) -> AppState {
        AppState {
            frame_index: 0,
            wireframe: false,
            perspective: false,
            render_ecb: false,
            invulnerable_type: InvulnerableType::Hit,
            camera,
            state: State::Play,
        }
    }

    pub fn update(&mut self, input: &WinitInputHelper<()>, subaction: &HighLevelSubaction) {
        if input.key_pressed(VirtualKeyCode::Key1) {
            self.wireframe = !self.wireframe;
        }
        if input.key_pressed(VirtualKeyCode::Key2) {
            self.perspective = !self.perspective;
        }
        if input.key_pressed(VirtualKeyCode::Key3) {
            self.render_ecb = !self.render_ecb;
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
        if input.key_pressed(VirtualKeyCode::Q) {
            self.invulnerable_type = InvulnerableType::Hit;
        }
        if input.key_pressed(VirtualKeyCode::W) {
            self.invulnerable_type = InvulnerableType::Grab;
        }
        if input.key_pressed(VirtualKeyCode::E) {
            self.invulnerable_type = InvulnerableType::TrapItem;
        }

        let small = 0.00001;

        if input.mouse_held(0) {
            let diff = input.mouse_diff();
            self.camera.theta -= diff.0 / 100.0;
            self.camera.phi -= diff.1 / 100.0;
        }

        if self.camera.theta > std::f32::consts::PI * 2.0 {
            self.camera.theta = 0.0;
        }
        else if self.camera.theta < 0.0 {
            self.camera.theta = std::f32::consts::PI * 2.0;
        }

        if self.camera.phi > std::f32::consts::PI - small {
            self.camera.phi = std::f32::consts::PI - small;
        }
        else if self.camera.phi < small {
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
