//! Load a single pac file and dump parsed structure to stdout.
//! brawllib usually operates at a much higher level than a single pac file, so this example is mostly for debugging brawllib itself.

use brawllib_rs::arc;
use brawllib_rs::wii_memory::WiiMemory;
use getopts::Options;
use std::env;
use std::path::PathBuf;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let program = &args[0];

    let mut opts = Options::new();
    opts.optopt("p", "path", "full path to a pac file", "PATH");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(program, opts);
            return;
        }
    };

    let path = if let Some(path) = matches.opt_str("p") {
        PathBuf::from(path)
    } else {
        println!("Need to pass a pac file\n");
        print_usage(program, opts);
        return;
    };
    let data = std::fs::read(path).unwrap();
    let arc = arc::arc(
        fancy_slice::FancySlice::new(&data),
        &WiiMemory::new(),
        false,
    );
    println!("{arc:#?}");
}
