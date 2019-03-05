use brawllib_rs::fighter::Fighter;
use brawllib_rs::high_level_fighter::{HighLevelFighter, HighLevelSubaction};

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
use three::controls::{Key, Orbit};

use getopts::Options;
use cgmath::{
    Matrix4,
    Matrix3,
    Quaternion,
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
    opts.optopt("a", "subaction", "subaction name", "ACTION_NAME");

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
    let mod_dir = matches.opt_str("m").map_or(None, |x| Some(fs::read_dir(x).expect("Provided mod directory is invalid")));

    let fighter_name = if let Some(fighter_name) = matches.opt_str("f") {
        fighter_name
    } else {
        println!("Need to pass a fighter name");
        print_usage(program, opts);
        return;
    };

    let subaction_name = if let Some(subaction_name) = matches.opt_str("a") {
        subaction_name
    } else {
        println!("Need to pass an subaction name");
        print_usage(program, opts);
        return;
    };

    for fighter in Fighter::load(brawl_dir, mod_dir, true) {
        if fighter.cased_name.to_lowercase() == fighter_name.to_lowercase() {
            let hl_fighter = HighLevelFighter::new(&fighter);
            for subaction in hl_fighter.subactions {
                if subaction.name.to_lowercase() == subaction_name.to_lowercase() {
                    let window_name = format!("Brawllib_rs visualiser - {} {}", fighter_name, subaction_name);
                    display_subaction(subaction, window_name);
                    return;
                }
            }
            println!("Passed subaction was not found");
            return;
        }
    }
    println!("Passed fighter was not found");
}

fn display_subaction(subaction: HighLevelSubaction, window_name: String) {
    let mut win = Window::new(window_name);

    // setup orbit
    let orbit_group = win.factory.group();
    win.scene.add(&orbit_group);
    let mut controls = Orbit::builder(&orbit_group)
        .position([0.0, 10.0, 60.0])
        .target([0.0, 10.01, 0.0])
        .build();

    // setup camera
    let camera = win.factory.perspective_camera(40.0, 1.0 .. 1000.0);
    orbit_group.add(&camera);

    // setup lighting
    let point_light = win.factory.point_light(0xffff00, 0.5);
    point_light.set_position([0.0, 15.0, 0.0]);
    orbit_group.add(&point_light);
    let ambient_light = win.factory.ambient_light(0xffff00, 0.0); // TODO: even at 0 intensity its still creating light
    ambient_light.set_position([0.0, -15.0, 0.0]); // TODO: Leaving this at [0,0,0] leaves a weird effect on marths leg. Which doesnt make sense, ambient light should be global.
    win.scene.add(&ambient_light);

    // setup text
    let font = win.factory.load_font_karla();
    let mut text = win.factory.ui_text(&font, "");
    text.set_font_size(30.0);
    win.scene.add(&text);

    // state
    let mut frame_index = 0;
    let mut wireframe = false;
    let mut wireframe_key = KeyState::None;
    let mut reset_camera = KeyState::Press; // Need to intiailize the camera
    let mut step_forward_key = KeyState::None;
    let mut step_backward_key = KeyState::None;
    let mut state = State::Play;

    while win.update() {
        wireframe_key.update(win.input.hit(Key::Key1));
        reset_camera.update(win.input.hit(Key::Back));
        step_forward_key.update(win.input.hit(Key::Space) || win.input.hit(Key::Right));
        step_backward_key.update(win.input.hit(Key::Left));
        if wireframe_key.is_pressed() {
            wireframe = !wireframe;
        }

        if reset_camera.is_pressed() {
            controls.reset();
        }
        if step_forward_key.is_pressed() {
            state = State::StepForward;
        }
        if step_backward_key.is_pressed() {
            state = State::StepBackward;
        }
        if win.input.hit(Key::Return) {
            state = State::Play;
        }

        // advance frame
        match state {
            State::StepForward | State::Play => {
                frame_index += 1;
                if frame_index >= subaction.frames.len() {
                    frame_index = 0;
                }
            }
            State::StepBackward => {
                if frame_index == 0 {
                    frame_index = subaction.frames.len() - 1;
                } else {
                    frame_index -= 1
                }
            }
            State::Pause => { }
        }

        // TODO: change to if let when stabilised
        match state {
            State::StepForward | State::StepBackward => {
                state = State::Pause;
            }
            _ => { }
        }

        text.set_text(format!("frame: {}/{}", frame_index+1, subaction.frames.len()));

        // TODO: This will need to be heavily modified to display as rounded cubes.
        //       Render 8 sphere corners then connect them together by planes the length of the stretch value
        //       of that dimension.
        // generate hurtboxes
        let frame = &subaction.frames[frame_index];
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

        win.scene.add(&hurt_box_group);
        controls.update(&win.input);
        win.render(&camera);
        win.scene.remove(hurt_box_group);
    }
}

enum State {
    Play,
    StepForward,
    StepBackward,
    Pause,
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
