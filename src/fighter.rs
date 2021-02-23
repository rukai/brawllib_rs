use std::collections::HashMap;
use std::fs::ReadDir;
use std::fs;
use std::path::Path;

use rayon::prelude::*;

use crate::arc::{Arc, ArcChildData};
use crate::arc;
use crate::bres::BresChildData;
use crate::chr0::Chr0;
use crate::mdl0::bones::Bone;
use crate::sakurai::fighter_data::ArcFighterData;
use crate::sakurai::fighter_data_common::ArcFighterDataCommon;
use crate::sakurai::{SectionData, SectionScript, ArcSakurai};
use crate::wii_memory::WiiMemory;

use fancy_slice::FancySlice;

#[derive(Debug)]
pub struct WiiRDFrameSpeedModifier {
    pub action: bool,
    pub action_subaction_id: u16,
    pub frame: u8,
    pub frame_speed: f32,
}

#[derive(Debug)]
pub struct Fighter {
    pub cased_name: String,
    pub moveset_common: Arc,
    pub moveset: Arc,
    pub motion: Arc,
    pub models: Vec<Arc>,
    pub kirby_hats: Vec<KirbyHat>,
    // TODO: Is there any reason to keep this now I can `mod_type`, any mods are going to be done by psa anyway...
    pub modded_by_psa: bool,
    pub mod_type: ModType,
    pub wiird_frame_speed_modifiers: Vec<WiiRDFrameSpeedModifier>,
}

#[derive(Debug)]
pub struct KirbyHat {
    pub moveset: Arc,
    pub models: Vec<Arc>,
}

impl Fighter {
    /// Call this function to get Fighter structs that correspond to each fighters folder in the 'fighter' directory
    ///
    /// brawl_fighter_dir must point at an exported Brawl 'fighter' directory.
    /// mod_fighter_dir may point at a brawl mod 'fighter' directory.
    /// Individual files in mod_fighter_dir will replace files in the brawl_fighter_dir with the same name.
    ///
    /// If single_model is true then only one model for each fighter is loaded, otherwise all models are loaded.
    /// It's much faster to only process one model so set this to true if you only need one.
    pub fn load(brawl_fighter_dir: ReadDir, mod_fighter_dir: Option<ReadDir>, common_fighter: &Arc, wii_memory: &WiiMemory, single_model: bool) -> Vec<Fighter> {
        // TODO: Could probably make this faster by beginning processing of a fighter_data immediately after it is read from disk.
        // However it might actually slow things down because all the threads are reading from disk at once.
        // Is there a way to stagger the threads so the next thread starts when the previous finishes reading from disk?
        // Will need to benchmark any such changes.
        let fighter_datas = fighter_datas(brawl_fighter_dir, mod_fighter_dir);
            fighter_datas.par_iter()
                .filter_map(|x| Fighter::load_single(x, &fighter_datas, common_fighter, single_model, wii_memory))
                .collect()
    }

    fn load_single(fighter_data: &FighterData, other_fighters: &[FighterData], common_fighter: &Arc, single_model: bool, wii_memory: &WiiMemory) -> Option<Fighter> {
        info!("Parsing fighter: {}", fighter_data.cased_name);
        let moveset_file_name = format!("Fit{}.pac", fighter_data.cased_name);
        let moveset = if let Some(data) = fighter_data.data.get(&moveset_file_name) {
            let data = FancySlice::new(data);
            arc::arc(data, wii_memory, false)
        } else {
            error!("Failed to load {}, missing moveset file: {}", fighter_data.cased_name, moveset_file_name);
            return None;
        };

        let moveset_common = common_fighter.clone();

        let psa_sequence = [0xfa, 0xde, 0xf0, 0x0d];
        let modded_by_psa = fighter_data.data.get(&moveset_file_name)
            .map(|a| a.windows(4).any(|b| b == psa_sequence))
            .unwrap_or(false);

        let motion_file_name = format!("Fit{}Motion.pac", fighter_data.cased_name);
        let motion_etc_file_name = format!("Fit{}MotionEtc.pac", fighter_data.cased_name);
        // we prioritise the Motion.pac file because sometimes mods replace a MotionEtc.pac with a Motion.pac (but never the other way around)
        let motion = if let Some(data) = fighter_data.data.get(&motion_file_name) {
            // TODO: I'm going to need better abstractions here as I cant read the Fit{}Etc file
            // Currently I dont need that file at all (What does it even contain?)
            // But when I do, I'll need to rethink how I abstract characters with and without combined Motion + Etc
            let data = FancySlice::new(data);
            arc::arc(data, wii_memory, false)
        } else {
            if let Some(data) = fighter_data.data.get(&motion_etc_file_name) {
                let data = FancySlice::new(data);
                arc::arc(data, wii_memory, false)
            } else {
                error!("Failed to load {}, Missing motion file: {} or {}", fighter_data.cased_name, motion_file_name, motion_etc_file_name);
                return None;
            }
        };

        let mut models = vec!();
        for i in 0..100 {
            if let Some(model_data) = fighter_data.data.get(&format!("Fit{}{:02}.pac", fighter_data.cased_name, i)) {
                let data = FancySlice::new(model_data);
                models.push(arc::arc(data, wii_memory, false));
                if single_model {
                    break;
                }
            }
            else {
                break;
            }
        }

        let mut kirby_hats = vec!();
        for other_fighter in other_fighters {
            if let Some(moveset_data) = fighter_data.data.get(&format!("FitKirby{}.pac", other_fighter.cased_name)) {
                info!("Parsing kirby hat: {}", other_fighter.cased_name);
                let moveset_data = FancySlice::new(moveset_data);
                let moveset = arc::arc(moveset_data, wii_memory, true);

                let mut models = vec!();
                for i in 0..100 {
                    if let Some(model_data) = fighter_data.data.get(&format!("FitKirby{}{:02}.pac", other_fighter.cased_name, i)) {
                        let data = FancySlice::new(model_data);
                        models.push(arc::arc(data, wii_memory, true));
                        if single_model {
                            break;
                        }
                    }
                    else {
                        break;
                    }
                }

                kirby_hats.push(KirbyHat { moveset, models });
            }
        }

        let mod_type = match (fighter_data.read_from_vanilla, fighter_data.read_from_mod) {
            (true, true)   => ModType::ModFromBase,
            (true, false)  => ModType::NotMod,
            (false, true)  => ModType::ModFromScratch,
            (false, false) => unreachable!("The data has to have been read from somewhere"),
        };

        let mut wiird_frame_speed_modifiers = vec!();
        let mut fighter_byte = 1;
        let mut offset = 0x80581000;
        let required_fighter_id = crate::fighter_maps::fighter_id(&fighter_data.cased_name);
        while fighter_byte != 0 {
            fighter_byte = wii_memory.read_u8(offset);
            if required_fighter_id.map(|id| id == fighter_byte).unwrap_or(false) || fighter_byte == 0xFF {
                wiird_frame_speed_modifiers.push(WiiRDFrameSpeedModifier {
                    action:              wii_memory.read_u8(offset + 2) & 0xF0 == 0,
                    action_subaction_id: wii_memory.read_u16(offset + 2) & 0x0FFF,
                    frame:               wii_memory.read_u8(offset + 1),
                    frame_speed:         wii_memory.read_f32(offset + 4),
                });
            }

            offset += 8;
        }

        Some(Fighter {
            cased_name: fighter_data.cased_name.clone(),
            moveset_common,
            moveset,
            motion,
            models,
            kirby_hats,
            modded_by_psa,
            mod_type,
            wiird_frame_speed_modifiers,
        })
    }

    /// retrieves the ArcSakurai
    pub fn get_fighter_sakurai(&self) -> Option<&ArcSakurai> {
        for sub_arc in &self.moveset.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref sakurai) => {
                    return Some(sakurai);
                }
                _ => { }
            }
        }
        None
    }

    /// retrieves the common ArcSakurai
    pub fn get_fighter_sakurai_common(&self) -> Option<&ArcSakurai> {
        for sub_arc in &self.moveset_common.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref sakurai) => {
                    return Some(sakurai);
                }
                _ => { }
            }
        }
        None
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

    /// retrieves the fighter data common
    pub fn get_fighter_data_common(&self) -> Option<&ArcFighterDataCommon> {
        for sub_arc in &self.moveset_common.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref data) => {
                    for section in &data.sections {
                        if let &SectionData::FighterDataCommon (ref fighter_data_ref) = &section.data {
                            return Some(fighter_data_ref);
                        }
                    }
                }
                _ => { }
            }
        }
        None
    }

    /// retrieves the script sections from fighter data common
    pub fn get_fighter_data_common_scripts(&self) -> Vec<&SectionScript> {
        let mut scripts = vec!();
        for sub_arc in &self.moveset_common.children {
            match &sub_arc.data {
                &ArcChildData::Sakurai (ref data) => {
                    for section in &data.sections {
                        if let &SectionData::Script (ref script) = &section.data {
                            scripts.push(script);
                        }
                    }
                }
                _ => { }
            }
        }
        scripts
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
                                    for model_child in model.iter() {
                                        // A check like this would be useful but it doesnt account for cloned mod fighters.
                                        // `if model_child.name.to_lowercase() == format!("Fit{}00", self.cased_name).to_lowercase() { }`
                                        // Instead, the first model is the characters model, so we just return it immediately.

                                        match &model_child.data {
                                            &BresChildData::Mdl0 (ref model) => {
                                                return model.bones.as_ref();
                                            }
                                            _ => { }
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
        // When checking the arc names, the characters name cannot be included
        // because modded characters name them inconsistently.

        if self.motion.name.ends_with("Motion") {
            return Fighter::get_animations_fit_motion(&self.motion);
        }
        else if self.motion.name.ends_with("MotionEtc") {
            for sub_arc in &self.motion.children {
                match &sub_arc.data {
                    &ArcChildData::Arc (ref arc) => {
                        if arc.name.ends_with("Motion") {
                            return Fighter::get_animations_fit_motion(arc);
                        }
                    }
                    _ => panic!("Only expecting Arc at this level"),
                }
            }
        }
        panic!("Could not find Motion Arc");
    }

    /// retrieves the animations for the character model from the Fit{}Motion arc
    pub fn get_animations_fit_motion(motion: &Arc) -> Vec<&Chr0> {
        let mut chr0s: Vec<&Chr0> = vec!();
        for sub_arc in &motion.children {
            match &sub_arc.data {
                &ArcChildData::Bres (ref bres) => {
                    for bres_child in bres.children.iter() {
                        match &bres_child.data {
                            &BresChildData::Bres (ref children) => {
                                for bres_child in children.iter() {
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
        chr0s
    }
}

/// Returns the binary fighter data for all fighters
/// Replaces brawl fighter data with mod fighter data
fn fighter_datas(brawl_fighter_dir: ReadDir, mod_fighter_dir: Option<ReadDir>) -> Vec<FighterData> {
    let mut fighter_datas = vec!();
    for fighter_path in brawl_fighter_dir {
        let fighter_path = fighter_path.unwrap();
        let file_type = fighter_path.file_type().unwrap();
        if file_type.is_dir() {
            if let Some(mut fighter_data) = fighter_data(&fighter_path.path()) {
                fighter_data.read_from_vanilla = true;
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
                        let file_data = std::fs::read(&data_path).unwrap();
                        fighter_data.data.insert(data_path.file_name().unwrap().to_str().unwrap().to_string(), file_data);
                        fighter_data.read_from_mod = true;
                    }
                }
                else {
                    // fighter data doesnt exist yet, create it
                    if let Some(mut fighter_data) = fighter_data(&fighter_path) {
                        fighter_data.read_from_mod = true;
                        fighter_datas.push(fighter_data);
                    }
                }
            }
        }
    }

    // copy missing warioman file from wario
    if let Some(Some(wario_motion_etc)) = fighter_datas.iter()
        .find(|x| x.cased_name == "Wario")
        .map(|x| x.data.get("FitWarioMotionEtc.pac").cloned())
    {
        for fighter_data in &mut fighter_datas {
            if fighter_data.cased_name == "WarioMan" {
                fighter_data.data.insert(String::from("FitWarioManMotionEtc.pac"), wario_motion_etc);
                // Just assume wariomans read_from_* is unaffected by this copy :/
                break;
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
            cased_name = Some(String::from(data_name.trim_end_matches(".pac").trim_start_matches("Fit")));
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
                let file_data = std::fs::read(&data_path).unwrap();
                data.insert(data_path.file_name().unwrap().to_str().unwrap().to_string(), file_data);
            }
            Some(FighterData {
                cased_name,
                data,
                // These fields get set later
                read_from_vanilla: false,
                read_from_mod: false,
            })
        }
    } else { None }
}

struct FighterData {
    cased_name: String,
    data: HashMap<String, Vec<u8>>,
    read_from_vanilla: bool,
    read_from_mod: bool,
}

#[derive(Debug)]
pub enum ModType {
    /// Original brawl fighter.
    /// All .pac files are unmodified from brawl.
    NotMod,
    /// Original brawl fighter that has been modified.
    /// Its .pac files overwrite those of the existing character, but if .pac files are missing existing ones are used.
    ModFromBase,
    /// Fighter that has been made from scratch.
    /// All .pac files define a new unique character without the use of any existing brawl .pac files.
    /// Although it can reference custom coding of a cloned character somehow.
    ModFromScratch,
}
