use std::fs::File;
use std::io::Read;

use crate::arc::{Arc, ArcChildData};
use crate::arc;
use crate::sakurai::{SectionData, SectionScript};
use crate::sakurai::fighter_data_common::ArcFighterDataCommon;
use crate::script_ast::ScriptAst;
use crate::high_level_fighter::HighLevelAction;

#[derive(Debug)]
struct FighterCommon {
    pub arc: Arc,
}

impl FighterCommon {
    /// Provide the path to the Fighter.pac file
    pub fn new(path: &str) -> FighterCommon {
        let mut common_data: Vec<u8> = vec!();
        File::open(path).unwrap().read_to_end(&mut common_data).unwrap();
        let arc = arc::arc(&common_data);
        FighterCommon { arc }
    }

    /// retrieves the fighter data common
    pub fn get_fighter_data_common(&self) -> Option<&ArcFighterDataCommon> {
        for sub_arc in &self.arc.children {
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
        for sub_arc in &self.arc.children {
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
}

/// HighLevelFighter already handles common data from the fighter.pac.
/// Only use HighLevelFighterCommon if you want to work with just the common data.
#[derive(Debug)]
pub struct HighLevelFighterCommon {
    pub scripts_section:  Vec<SectionScriptAst>,
    pub actions:          Vec<HighLevelAction>,
    pub scripts_fragment: Vec<ScriptAst>,
}

impl HighLevelFighterCommon {
    /// Provide the path to the Fighter.pac file
    pub fn new(path: &str) -> HighLevelFighterCommon {
        let fighter = FighterCommon::new(path);
        let fighter_data = fighter.get_fighter_data_common().unwrap();
        let fighter_data_scripts = fighter.get_fighter_data_common_scripts();

        let scripts_section:  Vec<SectionScriptAst> = fighter_data_scripts         .iter().map(|x| SectionScriptAst::new(x)).collect();
        let entry_actions:    Vec<ScriptAst>        = fighter_data.entry_actions   .iter().map(|x| ScriptAst       ::new(x)).collect();
        let exit_actions:     Vec<ScriptAst>        = fighter_data.exit_actions    .iter().map(|x| ScriptAst       ::new(x)).collect();
        let scripts_fragment: Vec<ScriptAst>        = fighter_data.fragment_scripts.iter().map(|x| ScriptAst       ::new(x)).collect();

        let mut actions = vec!();
        for i in 0..entry_actions.len() {
            actions.push(HighLevelAction {
                name:         crate::action_names::action_name(i),
                script_entry: entry_actions[i].clone(),
                script_exit:  exit_actions[i].clone(),
            });
        }


        HighLevelFighterCommon { scripts_section, actions, scripts_fragment }
    }
}

#[derive(Debug)]
pub struct SectionScriptAst {
    pub name:   String,
    pub script: ScriptAst,
}

impl SectionScriptAst {
    fn new(section_script: &SectionScript) -> SectionScriptAst {
        SectionScriptAst {
            name:   section_script.name.clone(),
            script: ScriptAst::new(&section_script.script),
        }
    }
}
