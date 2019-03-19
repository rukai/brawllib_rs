use brawllib_rs::wiird;
use brawllib_rs::wiird::{WiiRDBlock, WiiRDCode};

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
    opts.optopt("c", "codeset", "path to a gecko/WiiRD codeset", "CODESET_PATH");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            print_usage(program, opts);
            return;
        }
    };

    let codeset_path = if let Some(path) = matches.opt_str("c") {
        PathBuf::from(path)
    } else {
        println!("Need to pass a codeset path\n");
        print_usage(program, opts);
        return;
    };

    let codeset = match wiird::wiird_load_gct(&codeset_path) {
        Ok(codeset) => codeset,
        Err(err) => {
            println!("Failed to load codeset: {}", err);
            return;
        }
    };

    print_block(&codeset, "");
}

fn print_block(block: &WiiRDBlock, indent: &str) {
    for block in &block.codes {
        match block {
            WiiRDCode::IfStatement { then_branch, else_branch, test, .. } => {
                println!("{}If {:x?}", indent, test);
                print_block(then_branch, &format!("{}    ", indent));
                if let Some(else_branch) = else_branch {
                    println!("{}Else", indent);
                    print_block(else_branch, &format!("{}    ", indent));
                }
            }
            _ => {
                println!("{}{:x?}", indent, block);
            }
        }
    }
}
