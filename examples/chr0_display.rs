use brawllib_rs::fighter::Fighter;

use getopts::Options;

use std::fs;
use std::env;
use std::iter;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

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
    opts.optopt("b", "bone", "filter by bone", "BONE_NAME");

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
    let bone_filter = matches.opt_str("b");

    // display frame data
    for fighter in Fighter::load(brawl_dir, mod_dir, true) {
        if let &Some(ref fighter_filter) = &fighter_filter {
            if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                continue;
            }
        }
        println!("Fighter name: {}", fighter.cased_name);

        for chr0 in fighter.get_animations() {
            if let &Some(ref action_filter) = &action_filter {
                if chr0.name.to_lowercase() != action_filter.to_lowercase() {
                    continue;
                }
            }
            println!("Action name: {}", chr0.name);

            let frames_to_display: Box<Iterator<Item=i32>> = if let Some(frame_filter) = frame_filter {
                if frame_filter < chr0.num_frames as i32 {
                    Box::new(iter::once(frame_filter))
                } else {
                    Box::new(iter::empty())
                }
            } else {
                Box::new(0..chr0.num_frames as i32)
            };

            for frame in frames_to_display {
                println!("Frame index: {}", frame);
                for chr0_child in &chr0.children {
                    let loop_value = chr0.loop_value;
                    if let &Some(ref bone_filter) = &bone_filter {
                        if chr0_child.name.to_lowercase() != bone_filter.to_lowercase() {
                            continue;
                        }
                    }
                    println!("Bone name: {}", chr0_child.name);
                    println!("Translation: {:?}", chr0_child.translation.get_value(loop_value, frame, 0.0));
                    println!("Rot: {:?}", chr0_child.rot.get_value(loop_value, frame, 0.0));
                    println!("Scale: {:?}\n", chr0_child.scale.get_value(loop_value, frame, 1.0));

                    //println!("translation: {:#?}", chr0_child.translation);
                    //println!("rotation: {:#?}", chr0_child.rot);
                    //println!("scale: {:#?}", chr0_child.scale);
                }
            }
        }
    }
}
