use brawllib_rs::fighter::Fighter;
use getopts::Options;

use std::fs;
use std::env;

/// Store the output in a .rs file.
/// Then you can open the file in an IDE like vscode and "fold all" to get a readable tree structure.

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
    let fighter_filter = matches.opt_str("f");

    for fighter in Fighter::load(brawl_dir, mod_dir, true) {
        if let &Some(ref fighter_filter) = &fighter_filter {
            if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                continue;
            }
        }

        println!("{:#?}", fighter);
    }
}
