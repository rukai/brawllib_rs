use brawllib_rs::brawl_mod::BrawlMod;

use getopts::Options;

use std::path::PathBuf;
use std::env;
use std::fs::File;
use std::io::Write;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt("d", "dir", "full path to a brawl directory", "DIRECTORY_NAME");
    opts.optopt("m", "mod", "full path to a mod directory that will overwrite brawl files", "DIRECTORY_NAME");
    opts.optopt("f", "fighter", "fighter name", "FIGHTER_NAME");

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
    let mod_path = matches.opt_str("m").map(|x| PathBuf::from(x));

    let fighter_name = if let Some(fighter_name) = matches.opt_str("f") {
        fighter_name
    } else {
        println!("Need to pass a fighter name\n");
        print_usage(program, opts);
        return;
    };

    let brawl_mod = BrawlMod::new(&brawl_path, mod_path.as_ref().map(|x| x.as_path()));

    let fighters = match brawl_mod.load_fighters(true) {
        Ok(fighters) => fighters,
        Err(err) => {
            println!("Failed to load brawl mod: {}", err);
            return;
        }
    };

    for fighter in fighters {
        if fighter.cased_name.to_lowercase() == fighter_name.to_lowercase() {
            let model = fighter.models.get(0).unwrap().compile();
            let mut file = File::create("modifier_output.pac").unwrap();
            file.write_all(&model).unwrap();

            return;
        }
    }
    println!("Passed fighter was not found");
}
