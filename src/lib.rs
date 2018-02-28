extern crate byteorder;
extern crate cgmath;

#[macro_use] extern crate bitflags;

pub mod parse;
pub mod util;
pub mod bres;
pub mod resources;
pub mod chr0;
pub mod mdl0;
pub mod mbox;
pub mod misc_section;

use std::fs::{File, ReadDir};
use std::fs;
use std::io::Read;

use parse::BrawlFighter;

pub fn fighters(fighter_dir: ReadDir) -> Vec<BrawlFighter> {
    let mut fighters = vec!();
    for fighter_path in fighter_dir {
        let fighter_path = fighter_path.unwrap();
        if fighter_path.file_type().unwrap().is_dir() {
            let fighter_path = fighter_path.path();

            let folder_name = fighter_path.file_name().unwrap().to_str().unwrap().to_string();
            let mut cased_fighter_name: Option<String> = None;

            for data_path in fs::read_dir(&fighter_path).unwrap() {
                let data_path = data_path.unwrap().path();
                let data_name = data_path.file_name().unwrap().to_str().unwrap().to_string();
                if data_name.to_lowercase() == format!("Fit{}.pac", folder_name).to_lowercase() {
                    cased_fighter_name = Some(String::from(data_name.trim_right_matches(".pac").trim_left_matches("Fit")));
                }
            }

            // read
            if let Some(cased_fighter_name) = cased_fighter_name {
                let mut moveset_file = File::open(fighter_path.join(format!("Fit{}.pac", cased_fighter_name)));
                let mut motion_file = File::open(fighter_path.join(format!("Fit{}MotionEtc.pac", cased_fighter_name)));

                let mut model_files = vec!();
                for i in 0..100 {
                    if let Ok(model_file) = File::open(fighter_path.join(format!("Fit{}{:02}.pac", cased_fighter_name, i))) {
                        model_files.push(model_file);
                        break; // TODO: allow this to be toggled on/off
                    }
                    else {
                        break;
                    }
                }

                if let (Ok(mut moveset_file), Ok(mut motion_file)) = (moveset_file, motion_file) {
                    let mut moveset_data: Vec<u8> = vec!();
                    moveset_file.read_to_end(&mut moveset_data).unwrap();

                    let mut motion_data: Vec<u8> = vec!();
                    motion_file.read_to_end(&mut motion_data).unwrap();

                    let mut models = vec!();
                    for mut model_file in model_files {
                        let mut model_data: Vec<u8> = vec!();
                        model_file.read_to_end(&mut model_data).unwrap();
                        models.push(parse::arc(&model_data));
                    }

                    let fighter = BrawlFighter {
                        folder_name,
                        cased_fighter_name: cased_fighter_name,
                        moveset: parse::arc(&moveset_data),
                        motion: parse::arc(&motion_data),
                        models,
                    };
                    fighters.push(fighter);
                }
            }
        }
    }
    fighters
}

