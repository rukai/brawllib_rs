extern crate brawllib_rs;
extern crate getopts;
extern crate cgmath;

use brawllib_rs::fighter::Fighter;
use brawllib_rs::mdl0::bones::Bone;
use brawllib_rs::chr0::Chr0;

use getopts::Options;
use cgmath::{Vector3, Matrix4, SquareMatrix};

use std::fs;
use std::env;
use std::iter;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

/// WARNING! THE DATA DISPLAYED IS NOT USEFUL FOR ANYTHING EXCEPT DEBUGGING RAW ANIMATION VALUES.
/// Displayed bone values are dependent on parent bones.
fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt("d", "dir", "full path to a brawl directory", "DIRECTORY_NAME");
    opts.optopt("m", "mod", "full path to a mod directory that will overwrite brawl files", "DIRECTORY_NAME");
    opts.optopt("f", "fighter", "filter by fighter name", "FIGHTER_NAME");
    opts.optopt("a", "action", "filter by action", "ACTION_NAME");
    opts.optopt("i", "frame", "filter by frame", "FRAME_INDEX");

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
    let fighter_filter = matches.opt_str("f");
    let action_filter = matches.opt_str("a");
    let frame_filter = matches.opt_str("i").map_or(None, |x| x.parse::<i32>().ok());

    // display frame data
    for fighter in Fighter::load(brawl_dir, mod_dir, true) {
        if let &Some(ref fighter_filter) = &fighter_filter {
            if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                continue;
            }
        }
        println!("Fighter name: {}", fighter.cased_name);

        if let Some(bones) = fighter.get_bones() {
            for chr0 in fighter.get_animations() {
                if let &Some(ref action_filter) = &action_filter {
                    if chr0.name.to_lowercase() != action_filter.to_lowercase() {
                        continue;
                    }
                }
                println!("Action name: {}", chr0.name);

                let mut frames_to_display: Box<Iterator<Item=i32>> = if let Some(frame_filter) = frame_filter {
                    if frame_filter < chr0.num_frames as i32 {
                        Box::new(iter::once(frame_filter))
                    } else {
                        Box::new(iter::empty())
                    }
                } else {
                    Box::new(0..chr0.num_frames as i32)
                };

                for frame in frames_to_display {
                    println!("frame index: {}", frame);
                    let mut bones = bones.clone();
                    apply_chr0_to_bones_raw(&mut bones, Matrix4::<f32>::identity(), chr0, frame);
                    println!("{:#?}", bones);
                }
            }
        }
    }
}

/// WARNING! THE DATA RETURNED BY THIS FUNCTION IS NOT USEFUL FOR ANYTHING EXCEPT DEBUGGING RAW ANIMATION VALUES.
/// Modifies, in place, the matrices of the passed tree of bones, to follow that of the specified animation frame
/// The resulting matrices are dependent on its parent bone matrix.
/// Returns the MOVES_CHARACTER offset if enabled. this is used by e.g. Ness's double jump
fn apply_chr0_to_bones_raw(bone: &mut Bone, parent_transform: Matrix4<f32>, chr0: &Chr0, frame: i32) -> Option<Vector3<f32>> {
    // by default the bones tpose transformation is used.
    bone.transform = parent_transform * bone.gen_transform();
    let mut offset = None;
    for chr0_child in &chr0.children {
        let transform = parent_transform * chr0_child.get_transform(chr0.loop_value, frame);
        if chr0_child.name == bone.name {
            bone.transform = transform;
        }
    }

    // do the same for all children bones
    for child in bone.children.iter_mut() {
        if let Some(result) = apply_chr0_to_bones_raw(child, bone.transform, chr0, frame) {
            offset = Some(result);
        }
    }
    offset
}
