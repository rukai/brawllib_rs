/// A script manually created by brawllib to initialize the state of the memory so that the subaction runs in the most useful way.
/// e.g. enable hitboxes or prevent termination of the subaction

use crate::script_ast::{Block, EventAst};
use crate::script_ast::variable_ast::{VariableAst, LongtermAccessInt, LongtermAccessBool};

pub fn init_hack_script(fighter_name: &str, subaction_name: &str) -> Block {
    let events = match (fighter_name, subaction_name) {
        ("Wolf", "LandingAirN") => vec!(
            EventAst::IntVariableSet { value: 1, variable: VariableAst::LongtermAccessInt(LongtermAccessInt::Address(0x43)) },
        ),
        ("PokeFushigisou", "LandingAirN") => vec!(
            EventAst::BoolVariableSetTrue { variable: VariableAst::LongtermAccessBool(LongtermAccessBool::Address(0x73)) },
        ),
        ("Zelda", "LandingAirN") => vec!(
            EventAst::BoolVariableSetTrue { variable: VariableAst::LongtermAccessBool(LongtermAccessBool::Address(0x71)) },
        ),
        _ => vec!()
    };

    Block { events }
}
