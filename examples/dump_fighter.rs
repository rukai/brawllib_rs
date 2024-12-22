//! You may find it useful to:
//! 1.  Store the output in a .rs file.
//! 2.  Open the file in an IDE like vscode.
//! 3.  "fold all" to get a readable tree structure.

use brawllib_rs::brawl_mod::BrawlMod;

use getopts::Options;

use std::env;
use std::path::PathBuf;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt(
        "d",
        "dir",
        "full path to a brawl directory",
        "DIRECTORY_NAME",
    );
    opts.optopt(
        "m",
        "mod",
        "full path to a mod directory that will overwrite brawl files",
        "DIRECTORY_NAME",
    );
    opts.optopt("f", "fighter", "filter by fighter name", "FIGHTER_NAME");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(program, opts);
            return;
        }
    };

    let brawl_path = if let Some(path) = matches.opt_str("d") {
        PathBuf::from(path)
    } else {
        println!("Need to pass a brawl directory\n");
        print_usage(program, opts);
        return;
    };
    let mod_path = matches.opt_str("m").map(PathBuf::from);
    let fighter_filter = matches.opt_str("f");

    let brawl_mod = BrawlMod::new(&brawl_path, mod_path.as_deref());

    let fighters = match brawl_mod.load_fighters(true) {
        Ok(fighters) => fighters,
        Err(err) => {
            println!("Failed to load brawl mod: {}", err);
            return;
        }
    };

    for fighter in fighters {
        if let Some(fighter_filter) = &fighter_filter {
            if fighter.cased_name.to_lowercase() != fighter_filter.to_lowercase() {
                continue;
            }
        }

        println!("{:#?}", fighter);
    }
}
