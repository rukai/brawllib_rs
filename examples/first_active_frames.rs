use brawllib_rs::brawl_mod::BrawlMod;
use brawllib_rs::high_level_fighter::HighLevelFighter;

use getopts::Options;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    //env_logger::init();

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

    let brawl_mod = BrawlMod::new(&brawl_path, mod_path.as_ref().map(|x| x.as_path()));

    let fighters = match brawl_mod.load_fighters(true) {
        Ok(fighters) => fighters,
        Err(err) => {
            println!("Failed to load brawl mod: {}", err);
            return;
        }
    };

    let mut fighter_map = HashMap::new();
    for fighter in fighters {
        if fighter.cased_name.to_lowercase() == "poketrainer" {
            continue;
        }
        let fighter = HighLevelFighter::new(&fighter);
        let mut subaction_map = HashMap::new();

        for subaction in &fighter.subactions {
            let mut first_active_frame = None;
            for (i, frame) in subaction.frames.iter().enumerate() {
                if !frame.hit_boxes.is_empty() {
                    first_active_frame = Some(i + 1);
                    break;
                }
            }

            let facts = SubactionFacts {
                first_active_frame,
                length: subaction.frames.len(),
            };

            subaction_map.insert(subaction.name.clone(), facts);
        }
        fighter_map.insert(fighter.name.clone(), subaction_map);
    }

    let mut header = String::from("Attack");
    let mut fighter_names: Vec<_> = fighter_map.keys().collect();
    fighter_names.sort();
    for name in &fighter_names {
        header.push_str(&format!(",{}", name));
    }
    println!("{}", header);

    let moves = vec![
        ("Jab", "Attack11"),
        ("UTilt", "AttackHi3"),
        ("DTilt", "AttackLw3"),
        ("FTilt", "AttackS3S"),
        ("FSmash", "AttackS4Start"),
        ("DSmash", "AttackLw4Start"),
        ("USmash", "AttackHi4Start"),
        ("Nair", "AttackAirN"),
        ("Fair", "AttackAirF"),
        ("Bair", "AttackAirB"),
        ("Dair", "AttackAirLw"),
        ("Uair", "AttackAirHi"),
        ("Uair", "AttackDash"),
        ("Grab", "Catch"),
        ("Dash Grab", "CatchDash"),
        ("Pivot Grab", "CatchTurn"),
        ("Ledge Getup Attack Quick", "CliffAttackQuick"),
        ("Ledge Getup Attack Slow", "CliffAttackSlow"),
        ("Getup Attack Face Down", "DownAttackD"),
        ("Getup Attack Face Up", "DownAttackU"),
        ("Slip Getup Attack", "SlipAttack"),
    ];

    for (name, subaction_name) in moves {
        let mut row = name.to_string();
        for name in &fighter_names {
            if let Some(facts) = fighter_map[name.clone()].get(subaction_name) {
                if let Some(first_active_frame) = facts.first_active_frame {
                    row.push_str(&format!(",{}", first_active_frame))
                } else {
                    if subaction_name == "AttackS4Start" {
                        if let Some(first_active_frame) = fighter_map[name.clone()]
                            .get("AttackS4S")
                            .and_then(|x| x.first_active_frame)
                        {
                            row.push_str(&format!(",{}", facts.length + first_active_frame - 1));
                        } else {
                            row.push_str(",Unknown");
                        }
                    } else if subaction_name == "AttackLw4Start" {
                        if let Some(first_active_frame) = fighter_map[name.clone()]
                            .get("AttackLw4")
                            .and_then(|x| x.first_active_frame)
                        {
                            row.push_str(&format!(",{}", facts.length + first_active_frame - 1));
                        } else {
                            row.push_str(",Unknown");
                        }
                    } else if subaction_name == "AttackHi4Start" {
                        if let Some(first_active_frame) = fighter_map[name.clone()]
                            .get("AttackHi4")
                            .and_then(|x| x.first_active_frame)
                        {
                            row.push_str(&format!(",{}", facts.length + first_active_frame - 1));
                        } else {
                            row.push_str(",Unknown");
                        }
                    } else {
                        row.push_str(",Unknown");
                    }
                }
            } else {
                row.push_str(",Unknown");
            }
        }
        println!("{}", row);
    }
}

struct SubactionFacts {
    first_active_frame: Option<usize>,
    length: usize,
}
