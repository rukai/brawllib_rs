use std::collections::HashMap;
use std::fs::{File, ReadDir};
use std::fs;
use std::io::Read;
use std::path::Path;

use parse;
use parse::Arc;

#[derive(Debug)]
pub struct Fighter {
    pub cased_name: String,
    pub moveset: Arc,
    pub motion: Arc,
    pub models: Vec<Arc>
}

impl Fighter {
    /// This is the main entry point of the library.
    /// Call this function to get Fighter structs that correspond to each fighters folder in the 'fighter' directory
    ///
    /// brawl_fighter_dir must point at an exported Brawl 'fighter' directory.
    /// mod_fighter_dir may point at a brawl mod 'fighter' directory.
    /// Individual files in mod_fighter_dir will replace files in the brawl_fighter_dir with the same name.
    ///
    /// If single_model is true then only one model for each fighter is loaded, otherwise all models are loaded.
    /// It's much faster to only process one model so set this to true if you only need one.
    pub fn load(brawl_fighter_dir: ReadDir, mod_fighter_dir: Option<ReadDir>, single_model: bool) -> Vec<Fighter> {
        let mut fighters = vec!();
        for fighter_data in fighter_datas(brawl_fighter_dir, mod_fighter_dir) {
            let moveset_file_name = format!("Fit{}.pac", fighter_data.cased_name);
            let moveset = if let Some(data) = fighter_data.data.get(&moveset_file_name) {
                parse::arc(data)
            } else {
                println!("Missing moveset file: {}", moveset_file_name);
                continue;
            };

            let motion_file_name = format!("Fit{}MotionEtc.pac", fighter_data.cased_name);
            let motion = if let Some(data) = fighter_data.data.get(&motion_file_name) {
                parse::arc(data)
            } else {
                // TODO: This is being hit because some fighters just use another fighters motion file
                //       Handle this in the FighterFolder by duplicating the file in each special case.
                // TODO: This is being hit because some fighters dont have a MotionEtc file.
                //       instead they have seperate Motion and Etc files.
                //       Need to investigate if I can handle this by just reading the Motion file instead of the MotionEtc file
                println!("Missing motion file: {}", motion_file_name);
                continue;
            };

            let mut models = vec!();
            for i in 0..100 {
                if let Some(model_data) = fighter_data.data.get(&format!("Fit{}{:02}.pac", fighter_data.cased_name, i)) {
                    models.push(parse::arc(&model_data));
                    if single_model {
                        break;
                    }
                }
                else {
                    break;
                }
            }

            let fighter = Fighter {
                cased_name: fighter_data.cased_name,
                moveset,
                motion,
                models,
            };
            fighters.push(fighter);
        }
        fighters
    }
}

/// Returns the binary fighter data for all fighters
/// Replaces brawl fighter data with mod fighter data
fn fighter_datas(brawl_fighter_dir: ReadDir, mod_fighter_dir: Option<ReadDir>) -> Vec<FighterData> {
    let mut fighter_datas = vec!();
    for fighter_path in brawl_fighter_dir {
        let fighter_path = fighter_path.unwrap();
        if fighter_path.file_type().unwrap().is_dir() {
            if let Some(fighter_data) = fighter_data(&fighter_path.path()) {
                fighter_datas.push(fighter_data);
            }
        }
    }

    if let Some(mod_fighter_dir) = mod_fighter_dir {
        for fighter_path in mod_fighter_dir {
            let fighter_path = fighter_path.unwrap();
            if fighter_path.file_type().unwrap().is_dir() {
                let fighter_path = fighter_path.path();
                let dir_name = fighter_path.file_name().unwrap().to_str().unwrap().to_string();

                if let Some(fighter_data) = fighter_datas.iter_mut().find(|x| x.cased_name.to_lowercase() == dir_name) {
                    // fighter data already exists, overwrite and insert new files
                    for data_path in fs::read_dir(&fighter_path).unwrap() {
                        let data_path = data_path.unwrap().path();
                        let mut file_data: Vec<u8> = vec!();
                        File::open(&data_path).unwrap().read_to_end(&mut file_data).unwrap();
                        fighter_data.data.insert(data_path.file_name().unwrap().to_str().unwrap().to_string(), file_data);
                    }
                }
                else {
                    // fighter data doesnt exist yet, create it
                    if let Some(fighter_data) = fighter_data(&fighter_path) {
                        fighter_datas.push(fighter_data);
                    }
                }
            }
        }
    }

    fighter_datas
}

/// Returns the binary fighter data for each file in the passed dir
fn fighter_data(fighter_path: &Path) -> Option<FighterData> {
    let dir_name = fighter_path.file_name().unwrap().to_str().unwrap().to_string();
    let mut cased_name: Option<String> = None;

    for data_path in fs::read_dir(&fighter_path).unwrap() {
        let data_name = data_path.unwrap().path().file_name().unwrap().to_str().unwrap().to_string();
        if data_name.to_lowercase() == format!("Fit{}.pac", dir_name).to_lowercase() {
            cased_name = Some(String::from(data_name.trim_right_matches(".pac").trim_left_matches("Fit")));
        }
    }

    if let Some(cased_name) = cased_name {
        let mut data = HashMap::new();
        for data_path in fs::read_dir(&fighter_path).unwrap() {
            let data_path = data_path.unwrap().path();
            let mut file_data: Vec<u8> = vec!();
            File::open(&data_path).unwrap().read_to_end(&mut file_data).unwrap();
            data.insert(data_path.file_name().unwrap().to_str().unwrap().to_string(), file_data);
        }
        Some(FighterData { cased_name, data })
    } else { None }
}

struct FighterData {
    cased_name: String,
    data: HashMap<String, Vec<u8>>
}
