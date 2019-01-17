use std::collections::HashMap;
use std::fs::{File, ReadDir};
use std::fs;
use std::io::Read;
use std::path::Path;

use rayon::prelude::*;

use crate::arc::{Arc, ArcChildData};
use crate::arc;
use crate::bres::BresChildData;
use crate::chr0::Chr0;
use crate::mdl0::bones::Bone;
use crate::sakurai::{SectionData, ArcFighterData};

#[derive(Debug)]
pub struct Fighter {
    pub cased_name: String,
    pub moveset: Arc,
    pub motion: Arc,
    pub models: Vec<Arc>,
    pub modded_by_psa: bool
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
        // TODO: Could probably make this faster by beginning processing of a fighter_data immediately after it is read from disk.
        // However it might actually slow things down because all the threads are reading from disk at once.
        // Is there a way to stagger the threads so the next thread starts when the previous finishes reading from disk?
        // Will need to benchmark any such changes.
        fighter_datas(brawl_fighter_dir, mod_fighter_dir)
            .into_par_iter()
            .filter_map(|x| Fighter::load_single(x, single_model))
            .collect()
    }

    fn load_single(fighter_data: FighterData, single_model: bool) -> Option<Fighter> {
        info!("Parsing fighter: {}", fighter_data.cased_name);
        let moveset_file_name = format!("Fit{}.pac", fighter_data.cased_name);
        let moveset = if let Some(data) = fighter_data.data.get(&moveset_file_name) {
            arc::arc(data)
        } else {
            error!("Failed to load {}, missing moveset file: {}", fighter_data.cased_name, moveset_file_name);
            return None;
        };

        let psa_sequence = [0xfa, 0xde, 0xf0, 0x0d];
        let modded_by_psa = fighter_data.data.get(&moveset_file_name)
            .map(|a| a.windows(4).any(|b| b == psa_sequence))
            .unwrap_or(false);

        let motion_file_name = format!("Fit{}MotionEtc.pac", fighter_data.cased_name);
        let motion = if let Some(data) = fighter_data.data.get(&motion_file_name) {
            arc::arc(data)
        } else {
            // TODO: This is being hit because some fighters just use another fighters motion file
            //       Handle this in the FighterFolder by duplicating the file in each special case.
            // TODO: This is being hit because some fighters dont have a MotionEtc file.
            //       instead they have seperate Motion and Etc files.
            //       Need to investigate if I can handle this by just reading the Motion file instead of the MotionEtc file
            error!("Failed to load {}, Missing motion file: {}", fighter_data.cased_name, motion_file_name);
            return None;
        };

        let mut models = vec!();
        for i in 0..100 {
            if let Some(model_data) = fighter_data.data.get(&format!("Fit{}{:02}.pac", fighter_data.cased_name, i)) {
                models.push(arc::arc(&model_data));
                if single_model {
                    break;
                }
            }
            else {
                break;
            }
        }

        Some(Fighter {
            cased_name: fighter_data.cased_name,
            moveset,
            motion,
            models,
            modded_by_psa
        })
    }

    /// retrieves the fighter data
    pub fn get_fighter_data(&self) -> Option<&ArcFighterData> {
        for sub_arc in &self.moveset.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref data) => {
                    for section in &data.sections {
                        if let &SectionData::FighterData (ref fighter_data_ref) = &section.data {
                            return Some(fighter_data_ref);
                        }
                    }
                }
                _ => { }
            }
        }
        None
    }

    /// retrieves the bones from a character model
    pub fn get_bones(&self) -> Option<&Bone> {
        if let Some(model) = self.models.get(0) {
            for sub_arc in model.children.iter() {
                match &sub_arc.data {
                    &ArcChildData::Arc (_) => {
                        panic!("Not expecting arc at this level")
                    }
                    &ArcChildData::Bres (ref bres) => {
                        for bres_child in bres.children.iter() {
                            match &bres_child.data {
                                &BresChildData::Bres (ref model) => {
                                    for model_child in model.children.iter() {
                                        if model_child.name == format!("Fit{}00", self.cased_name) {
                                            match &model_child.data {
                                                &BresChildData::Mdl0 (ref model) => {
                                                    return model.bones.as_ref();
                                                }
                                                _ => { }
                                            }
                                        }
                                    }
                                }
                                &BresChildData::Mdl0 (_) => {
                                    panic!("Not expecting Mdl at this level");
                                }
                                _ => { }
                            }
                        }
                    }
                    _ => { }
                }
            }
        }
        None
    }

    /// retrieves the animations for the character model
    pub fn get_animations(&self) -> Vec<&Chr0> {
        let mut chr0s: Vec<&Chr0> = vec!();
        for sub_arc in &self.motion.children {
            match &sub_arc.data {
                &ArcChildData::Arc (ref arc) => {
                    for sub_arc in &arc.children {
                        match &sub_arc.data {
                            &ArcChildData::Bres (ref bres) => {
                                for bres_child in bres.children.iter() {
                                    match &bres_child.data {
                                        &BresChildData::Bres (ref bres) => {
                                            for bres_child in bres.children.iter() {
                                                match &bres_child.data {
                                                    &BresChildData::Bres (_) => {
                                                        panic!("Not expecting bres at this level");
                                                    }
                                                    &BresChildData::Chr0 (ref chr0) => {
                                                        chr0s.push(chr0);
                                                    }
                                                    _ => { }
                                                }
                                            }
                                        }
                                        &BresChildData::Chr0 (_) => {
                                            panic!("Not expecting Chr0 at this level");
                                        }
                                        _ => { }
                                    }
                                }
                            }
                            &ArcChildData::Arc (_) => {
                                //panic!("Not expecting arc at this level"); // TODO: Whats here
                            }
                            _ => { }
                        }
                    }
                }
                &ArcChildData::Bres (_) => {
                    panic!("Not expecting bres at this level");
                }
                _ => { }
            }
        }
        chr0s
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

                if let Some(fighter_data) = fighter_datas.iter_mut().find(|x| x.cased_name.to_lowercase() == dir_name.to_lowercase()) {
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
        if cased_name == "ZakoBoy" || cased_name == "ZakoGirl" || cased_name == "ZakoChild" || cased_name == "ZakoBall" {
            error!("Can't load: {} (unfixed bug)", cased_name);
            None
        } else {
            let mut data = HashMap::new();
            for data_path in fs::read_dir(&fighter_path).unwrap() {
                let data_path = data_path.unwrap().path();
                let mut file_data: Vec<u8> = vec!();
                File::open(&data_path).unwrap().read_to_end(&mut file_data).unwrap();
                data.insert(data_path.file_name().unwrap().to_str().unwrap().to_string(), file_data);
            }
            Some(FighterData { cased_name, data })
        }
    } else { None }
}

struct FighterData {
    cased_name: String,
    data: HashMap<String, Vec<u8>>
}
