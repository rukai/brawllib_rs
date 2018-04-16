extern crate brawllib_rs;
extern crate cgmath;
extern crate getopts;
extern crate three;
extern crate mint;

use brawllib_rs::fighter::Fighter;
use brawllib_rs::high_level_fighter::{HighLevelFighter, HighLevelAction};

use three::{
    Window,
    Object,
    Geometry,
};
use three::material::{
    Wireframe,
    Phong,
    Material,
};
use three::controls::Key;

use getopts::Options;
use cgmath::{
    Matrix4,
    Matrix3,
    Quaternion,
    Rotation3,
    Zero,
};

use std::fs;
use std::env;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt("d", "dir", "full path to a brawl directory", "DIRECTORY_NAME");
    opts.optopt("m", "mod", "full path to a mod directory that will overwrite brawl files", "DIRECTORY_NAME");
    opts.optopt("f", "fighter", "fighter name", "FIGHTER_NAME");
    opts.optopt("a", "action", "action name", "ACTION_NAME");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(program, opts);
            return;
        }
    };

    let brawl_dir = if let Some(path) = matches.opt_str("d") {
        match fs::read_dir(path) {
            Ok(dir) => dir,
            Err(_) => {
                println!("The passed brawl directory does not exist.");
                print_usage(program, opts);
                return;
            }
        }
    } else {
        println!("Need to pass a brawl directory");
        print_usage(program, opts);
        return;
    };
    let mod_dir = matches.opt_str("m").map_or(None, |x| fs::read_dir(x).ok());

    let fighter_name = if let Some(fighter_name) = matches.opt_str("f") {
        fighter_name
    } else {
        println!("Need to pass a fighter name");
        print_usage(program, opts);
        return;
    };

    let action_name = if let Some(action_name) = matches.opt_str("a") {
        action_name
    } else {
        println!("Need to pass an action name");
        print_usage(program, opts);
        return;
    };

    for fighter in Fighter::load(brawl_dir, mod_dir, true) {
        if fighter.cased_name.to_lowercase() == fighter_name.to_lowercase() {
            let hl_fighter = HighLevelFighter::new(&fighter);
            for action in hl_fighter.actions {
                if action.name.to_lowercase() == action_name.to_lowercase() {
                    let window_name = format!("Brawllib_rs visualiser - {} {}", fighter_name, action_name);
                    display_action(action, window_name);
                    return;
                }
            }
            println!("Passed action was not found");
            return;
        }
    }
    println!("Passed fighter was not found");
}

fn display_action(action: HighLevelAction, window_name: String) {
    let mut win = Window::new(window_name);

    // setup camera
    let camera = win.factory.perspective_camera(40.0, 1.0 .. 1000.0);
    camera.set_position([0.0, 10.0, 60.0]);

    // setup lighting
    let light = win.factory.point_light(0xffff00, 0.9);
    light.set_position([0.0, 25.0, 40.0]);
    win.scene.add(light);

    // setup text
    let font = win.factory.load_font_karla();
    let mut text = win.factory.ui_text(&font, "");
    text.set_font_size(30.0);
    win.scene.add(&text);

    // state
    let mut frame_index = 0;
    let mut angle = cgmath::Rad::zero();
    let mut wireframe = false;
    let mut wireframe_key = KeyState::None;
    let mut step_key = KeyState::None;
    let mut state = State::Play;

    while win.update() {
        // process user input
        if let Some(diff) = win.input.timed(three::AXIS_LEFT_RIGHT) {
            angle -= cgmath::Rad(1.5 * diff);
        }

        wireframe_key.update(win.input.hit(Key::Key1));
        if wireframe_key.is_pressed() {
            wireframe = !wireframe;
        }

        step_key.update(win.input.hit(Key::Space));
        if step_key.is_pressed() {
            state = State::Step;
        }
        if win.input.hit(Key::Return) {
            state = State::Play;
        }

        // advance frame
        if state.frame_advance() {
            frame_index += 1;
            if frame_index >= action.frames.len() {
                frame_index = 0;
            }
            text.set_text(format!("frame: {}/{}", frame_index+1, action.frames.len()));
        }
        if let State::Step = state {
            state = State::Pause;
        }

        // generate hurtboxes
        let frame = &action.frames[frame_index];
        let hurt_box_group = win.factory.group();
        for hurt_box in &frame.hurt_boxes {
            let diameter = hurt_box.hurt_box.radius * 2.0;
            let stretch = hurt_box.hurt_box.stretch;
            let transform = hurt_box.bone_matrix * Matrix4::<f32>::from_translation(hurt_box.hurt_box.offset + stretch);
            let object = {
                // The cuboid is generated with [0, 0, 0] at the center
                let geometry = Geometry::cuboid(diameter + stretch.x.abs(), diameter + stretch.y.abs(), diameter + stretch.z.abs());
                let material: Material = if wireframe {
                    Material::Wireframe (Wireframe { color: 0xFFFF00 })
                } else {
                    Material::Phong (Phong {
                        color: 0xFFFF00,
                        glossiness: 80.0,
                    })
                };
                win.factory.mesh(geometry, material)
            };

            let transform3 = Matrix3::new(
                hurt_box.bone_matrix.x.x,
                hurt_box.bone_matrix.x.y,
                hurt_box.bone_matrix.x.z,
                hurt_box.bone_matrix.y.x,
                hurt_box.bone_matrix.y.y,
                hurt_box.bone_matrix.y.z,
                hurt_box.bone_matrix.z.x,
                hurt_box.bone_matrix.z.y,
                hurt_box.bone_matrix.z.z,
            );
            let orientation: Quaternion<f32> = transform3.into();
            object.set_orientation(orientation);
            object.set_position([transform.w.x, transform.w.y, transform.w.z]);

            hurt_box_group.add(&object);
        }

        hurt_box_group.set_orientation(Quaternion::from_angle_y(angle));
        win.scene.add(&hurt_box_group);
        win.render(&camera);
        win.scene.remove(hurt_box_group);
    }
}

enum State {
    Play,
    Step,
    Pause,
}

impl State {
    fn frame_advance(&self) -> bool {
        match self {
            State::Play  => true,
            State::Step  => true,
            State::Pause => false,
        }
    }
}

enum KeyState {
    Press,
    Hold,
    None,
}

impl KeyState {
    fn update(&mut self, value: bool) {
        *self = match self {
            KeyState::Press | KeyState::Hold => {
                if value {
                    KeyState::Hold
                } else {
                    KeyState::None
                }
            }
            KeyState::None => {
                if value {
                    KeyState::Press
                } else {
                    KeyState::None
                }
            }
        }
    }

    fn is_pressed(&self) -> bool {
        match self {
            KeyState::Press => true,
            KeyState::Hold  => false,
            KeyState::None  => false,
        }
    }
}
