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
    Material,
};
use three::material::basic::Basic;

use getopts::Options;
use cgmath::{Matrix4, Zero, Quaternion, Rotation3};

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

    let camera = win.factory.perspective_camera(40.0, 1.0 .. 1000.0);
    camera.set_position([0.0, 15.0, 40.0]);

    let mut frame = 0;
    let mut angle = cgmath::Rad::zero();
    while win.update() {
        if let Some(diff) = win.input.timed(three::AXIS_LEFT_RIGHT) {
            angle -= cgmath::Rad(1.5 * diff);
        }
        let hurt_box_group = win.factory.group();

        frame += 1;
        if frame >= action.frames.len() {
            frame = 0;
        }
        let frame = &action.frames[frame];

        for hurt_box in &frame.hurt_boxes {
            let transform = hurt_box.bone_matrix * Matrix4::<f32>::from_translation(hurt_box.hurt_box.offset);

            let msphere = {
                let geometry = Geometry::uv_sphere(0.5, 5, 5);
                let material = three::material::Wireframe { color: 0xFFFF00 };
                //let material = Material::Basic (Basic {
                //    color: 0xFFFF00,
                //    map: None,
                //});
                win.factory.mesh(geometry, material)
            };
            msphere.set_position([transform.w.x, transform.w.y, transform.w.z]);
            hurt_box_group.add(&msphere);
        }
        hurt_box_group.set_orientation(Quaternion::from_angle_y(angle));
        win.scene.add(&hurt_box_group);
        win.render(&camera);
        win.scene.remove(hurt_box_group);
    }
}
