use brawllib_rs::brawl_mod::BrawlMod;
use brawllib_rs::high_level_fighter::{
    CollisionBoxValues, GrabBoxValues, HighLevelFighter, HighLevelHitBox,
};
use brawllib_rs::script_ast::GrabTarget;

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

    let fighters: Vec<_> = fighters
        .iter()
        .filter(|x| x.cased_name.to_lowercase() != "poketrainer")
        .collect();

    println!("Bad overlapping hitboxes:");
    println!("If there are 2 Hitboxes occupying the exact same space and one or more hits both grounded and aerial opponents.");
    println!("Or if there are 3 hitboxes occupying the exact same space.");
    println!("This is bad because sometimes it doesnt work correctly, resulting in a hitbox hitting that should never be possible e.g. https://twitter.com/ShaydonJohn/status/1147308339753127936");
    for fighter in fighters.iter() {
        let fighter = HighLevelFighter::new(&fighter);
        'subactions: for subaction in &fighter.subactions {
            for frame in &subaction.frames {
                // group hitboxes together that have a matching size and position
                let mut hitbox_groups: Vec<Vec<&HighLevelHitBox>> = vec![];
                for hitbox in &frame.hit_boxes {
                    let mut hitbox_matched = false;
                    for hitbox_group in &mut hitbox_groups {
                        if hitbox_group[0].next_size == hitbox.next_size
                            && hitbox_group[0].next_pos == hitbox.next_pos
                            && hitbox_group[0].prev_size == hitbox.prev_size
                            && hitbox_group[0].prev_pos == hitbox.prev_pos
                        {
                            hitbox_group.push(hitbox);
                            hitbox_matched = true;
                        }
                    }
                    if !hitbox_matched {
                        hitbox_groups.push(vec![hitbox]);
                    }
                }

                // display grouped hitboxes that are problematic
                for hitbox_group in &hitbox_groups {
                    if (hitbox_group.len() > 1
                        && hitbox_group.iter().any(|x| hitbox_hits_everything(x)))
                        || hitbox_group.len() > 2
                    {
                        println!("{} {}", fighter.name, subaction.name);

                        // avoid spamming the user about every time it occurs on a subaction
                        continue 'subactions;
                    }
                }
            }
        }
    }

    println!("\nCreating an unconditional interrupt on the first line of a subaction:");
    println!("This was used by PMDT before action overrides were understood.");
    println!("All of these cases should be replaced with action overrides so that move staling is properly handled.");
    for fighter in fighters {
        let fighter = HighLevelFighter::new(&fighter);
        for subaction in &fighter.subactions {
            if subaction.bad_interrupts {
                println!("{} {}", fighter.name, subaction.name);
            }
        }
    }
}

fn hitbox_hits_everything(hitbox: &HighLevelHitBox) -> bool {
    match &hitbox.next_values {
        CollisionBoxValues::Hit(hit) => hit.ground && hit.aerial,
        CollisionBoxValues::Grab(GrabBoxValues {
            target: GrabTarget::AerialAndGrounded,
            ..
        }) => true,
        _ => false,
    }
}
