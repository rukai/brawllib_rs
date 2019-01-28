use brawllib_rs::fighter::Fighter;
use brawllib_rs::high_level_fighter::HighLevelFighter;

use getopts::Options;

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
    opts.optopt("f", "fighter", "filter by fighter name", "FIGHTER_NAME");
    opts.optopt("a", "subaction", "filter by subaction", "ACTION_NAME");
    opts.optopt("i", "frame", "filter by frame", "FRAME_INDEX");
    opts.optopt("l", "datalevel", "level to display data at", "[fighter|subaction|frame]");

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
    let subaction_filter = matches.opt_str("a");
    let frame_filter = matches.opt_str("i").map_or(None, |x| x.parse::<usize>().ok());
    let data_level = matches.opt_str("l").unwrap_or(String::from("fighter")).to_lowercase();

    // display frame data
    match data_level.as_ref() {
        "frame" => {
            for fighter in Fighter::load(brawl_dir, mod_dir, true) {
                if let &Some(ref fighter_filter) = &fighter_filter {
                    if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                        continue;
                    }
                }
                println!("Fighter name: {}", fighter.cased_name);

                let hl_fighter = HighLevelFighter::new(&fighter);
                for subaction in hl_fighter.subactions {
                    if let &Some(ref subaction_filter) = &subaction_filter {
                        if subaction.name.to_lowercase() != subaction_filter.to_lowercase() {
                            continue;
                        }
                    }
                    println!("Subaction name: {}", subaction.name);

                    if let Some(frame_filter) = frame_filter {
                        if let Some(frame) = subaction.frames.get(frame_filter) {
                            println!("{:#?}", frame);
                        }
                    }
                    else {
                        println!("{:#?}", subaction.frames);
                    }
                }
            }
        }
        "subaction" => {
            for fighter in Fighter::load(brawl_dir, mod_dir, true) {
                if let &Some(ref fighter_filter) = &fighter_filter {
                    if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                        continue;
                    }
                }
                println!("Fighter name: {}", fighter.cased_name);

                let hl_fighter = HighLevelFighter::new(&fighter);
                for mut subaction in hl_fighter.subactions {
                    if let &Some(ref subaction_filter) = &subaction_filter {
                        if subaction.name.to_lowercase() != subaction_filter.to_lowercase() {
                            continue;
                        }
                    }

                    if let Some(frame_filter) = frame_filter {
                        if frame_filter < subaction.frames.len() {
                            subaction.frames = vec!(subaction.frames.remove(frame_filter));
                        } else {
                            subaction.frames.clear();
                        }
                    }
                    println!("{:#?}", subaction);
                }
            }
        }
        "fighter" => {
            for fighter in Fighter::load(brawl_dir, mod_dir, true) {
                if let &Some(ref fighter_filter) = &fighter_filter {
                    if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                        continue;
                    }
                }

                let mut hl_fighter = HighLevelFighter::new(&fighter);

                // filter by subaction
                if let &Some(ref subaction_filter) = &subaction_filter {
                    let mut new_subactions = vec!();
                    for subaction in hl_fighter.subactions {
                        if subaction.name.to_lowercase() == subaction_filter.to_lowercase() {
                            new_subactions.push(subaction);
                        }
                    }
                    hl_fighter.subactions = new_subactions;
                }

                // filter by frame
                for subaction in &mut hl_fighter.subactions {
                    if let Some(frame_filter) = frame_filter {
                        if frame_filter < subaction.frames.len() {
                            subaction.frames = vec!(subaction.frames.remove(frame_filter));
                        } else {
                            subaction.frames.clear();
                        }
                    }
                }
                println!("{:#?}", hl_fighter);
            }
        }
        _ => {
            println!("Invalid data level");
            print_usage(program, opts);
            return;
        }
    }
}
